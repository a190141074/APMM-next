//! 投稿 HTTP 边界：凭证只在后端读取并写入授权头，回执只保存安全标识。
use super::*;
use serde_json::{json, Value};

pub(super) fn base() -> Option<String> {
    #[cfg(debug_assertions)]
    if let Ok(raw) = std::env::var("APMM_SUBMISSION_TEST_URL") {
        if let Ok(url) = reqwest::Url::parse(&raw) {
            if url.scheme() == "http"
                && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
                && url.port().is_some()
                && url.query().is_none()
                && url.fragment().is_none()
                && url.path() == "/"
            {
                return Some(raw.trim_end_matches('/').into());
            }
        }
    }
    let config: Value = serde_json::from_str(include_str!("../../../config/cnb.json")).ok()?;
    if config["submission"]["enabled"] != true {
        return None;
    }
    let raw = config["submission"]["apiBase"].as_str()?;
    let url = reqwest::Url::parse(raw).ok()?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(raw.trim_end_matches('/').into())
}
fn client() -> Result<Client, String> {
    Client::builder()
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|_| "无法建立投稿连接".into())
}
fn decode(response: reqwest::blocking::Response) -> Result<Value, String> {
    match response.status().as_u16() {
        200..=299 => {}
        401 | 403 => return Err("作品管理凭证无效或没有操作权限".into()),
        409 => return Err("投稿或已发布基线已变化，请同步作品后重新预检".into()),
        413 => return Err("投稿超过服务大小或存储限额".into()),
        429 | 503 => return Err("投稿服务繁忙，请稍后使用同一快照重试".into()),
        422 => return Err("服务拒绝了投稿格式，请检查分类、资料和图片".into()),
        _ => return Err("投稿服务返回错误，请稍后重试".into()),
    }
    let mut bytes = Vec::new();
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "投稿响应中断，请重试")?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("投稿响应过大".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "投稿响应格式无效".into())
}
fn send(method: reqwest::Method, route: &str, id: Option<&str>) -> Result<Value, String> {
    let url = format!("{}{}", base().ok_or("投稿服务尚未配置")?, route);
    let mut request = client()?.request(method, url);
    if let Some(id) = id {
        request = request.bearer_auth(credentials::secret(id)?);
    }
    decode(request.send().map_err(|_| "无法连接投稿服务，请稍后重试")?)
}
#[tauri::command]
pub(crate) async fn author_service() -> Result<Value, String> {
    if base().is_none() {
        return Ok(json!({"available":false,"message":"投稿服务尚未配置；草稿与本地导出仍可使用"}));
    }
    tauri::async_runtime::spawn_blocking(|| match send(reqwest::Method::GET, "/v1/health", None) {
        Ok(limits) => Ok(json!({"available":true,"message":"投稿服务可用","limits":limits})),
        Err(_) => {
            Ok(json!({"available":false,"message":"暂时无法连接投稿服务；草稿与本地导出仍可使用"}))
        }
    })
    .await
    .map_err(|_| "服务检查中断")?
}

pub(crate) async fn upload(id: String, app: tauri::AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let snapshot = snapshot::read(&id)?;
        let archive = snapshot::path(&id)?.join("upload.zip");
        if sha256_file(&archive)? != snapshot.archive_hash {
            return Err("投稿快照已损坏，请重新预检".into());
        }
        crate::tasks::checkpoint(&app)?;
        let work_id = &snapshot.work_id;
        credentials::ensure(work_id)?;
        send(
            reqwest::Method::PUT,
            &format!("/v1/works/{work_id}"),
            Some(work_id),
        )?;
        crate::tasks::checkpoint(&app)?;
        // 接收端提交具有幂等性；交接后取消不能撤销已成功的服务器事务。
        crate::tasks::commit_phase(&app)?;
        let response = client()?
            .put(format!(
                "{}/v1/works/{work_id}/submissions/{id}",
                base().ok_or("投稿服务未配置")?
            ))
            .bearer_auth(credentials::secret(work_id)?)
            .header("Content-Type", "application/zip")
            .header("X-Content-SHA256", &snapshot.archive_hash)
            .body(fs::File::open(archive).map_err(|_| "投稿包读取失败")?)
            .send()
            .map_err(|_| "上传中断，草稿已保留；重试会继续使用同一投稿编号")?;
        let reply = decode(response)?;
        if reply["id"] != id
            || reply["workId"] != *work_id
            || reply["snapshotHash"] != snapshot.archive_hash
        {
            return Err("投稿回执与快照不符".into());
        }
        let receipt = AuthorReceipt {
            id: id.clone(),
            work_id: work_id.clone(),
            draft_id: snapshot.preview.draft.id.clone(),
            snapshot_hash: snapshot.archive_hash.clone(),
            status: status(&reply)?,
            message: String::new(),
            created_at: unix_now(),
        };
        let _guard = lock()?;
        let mut state = load()?;
        state.receipts.retain(|item| item.id != id);
        state.receipts.push(receipt);
        save(&state)?;
        Ok(json!({"submissionId":id,"workId":work_id,"message":"投稿已接收，请等待审核"}))
    })
    .await
    .map_err(|_| "上传任务中断")?
}
fn status(value: &Value) -> Result<String, String> {
    let value = value["status"].as_str().unwrap_or("");
    if [
        "pending",
        "approved",
        "publishing",
        "published",
        "rejected",
        "cancelled",
        "withdrawn",
    ]
    .contains(&value)
    {
        Ok(value.into())
    } else {
        Err("投稿状态无效".into())
    }
}

#[tauri::command]
pub(crate) async fn author_refresh(id: String) -> Result<AuthorState, String> {
    checked_id(&id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let response = send(reqwest::Method::GET, &format!("/v1/works/{id}"), Some(&id))?;
        let published: Option<crate::workshop::Work> =
            serde_json::from_value(response["published"].clone())
                .map_err(|_| "作品基线格式无效")?;
        if published.as_ref().is_some_and(|work| work.id != id) {
            return Err("作品基线编号不符".into());
        }
        let _guard = lock()?;
        let mut state = load()?;
        let work = state
            .works
            .iter_mut()
            .find(|work| work.id == id)
            .ok_or("作品未绑定")?;
        work.revision = response["revision"].as_u64().ok_or("作品修订号缺失")?;
        if let Some(published) = published {
            work.title = published.title.clone();
            work.published = Some(published);
        }
        for remote in response["submissions"].as_array().ok_or("投稿记录缺失")? {
            if let Some(receipt) = state
                .receipts
                .iter_mut()
                .find(|item| remote["id"] == item.id)
            {
                receipt.status = status(remote)?;
            } else {
                let submission_id = remote["id"].as_str().ok_or("投稿编号缺失")?;
                checked_id(submission_id)?;
                let hash = remote["snapshotHash"].as_str().ok_or("投稿摘要缺失")?;
                if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("投稿摘要无效".into());
                }
                state.receipts.push(AuthorReceipt {
                    id: submission_id.into(),
                    work_id: id.clone(),
                    snapshot_hash: hash.into(),
                    status: status(remote)?,
                    ..Default::default()
                });
            }
        }
        save(&state)?;
        Ok(state)
    })
    .await
    .map_err(|_| "同步作品中断")?
}
#[tauri::command]
pub(crate) async fn author_cancel(id: String, work_id: String) -> Result<(), String> {
    checked_id(&id)?;
    checked_id(&work_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        send(
            reqwest::Method::DELETE,
            &format!("/v1/works/{work_id}/submissions/{id}"),
            Some(&work_id),
        )?;
        let _guard = lock()?;
        let mut state = load()?;
        if let Some(receipt) = state
            .receipts
            .iter_mut()
            .find(|r| r.id == id && r.work_id == work_id)
        {
            receipt.status = "cancelled".into();
        }
        save(&state)
    })
    .await
    .map_err(|_| "取消投稿中断")?
}
