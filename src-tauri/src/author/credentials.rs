//! 每件作品独立的系统凭证；界面只能发起加密备份与恢复，不能读取明文凭证。
use super::*;
use base64::Engine;

fn entry(id: &str) -> Result<keyring::Entry, String> {
    checked_id(id)?;
    keyring::Entry::new("APMM-Workshop", id).map_err(|_| "系统凭据库不可用".into())
}
pub(super) fn secret(id: &str) -> Result<String, String> {
    entry(id)?
        .get_password()
        .map_err(|_| "管理凭证不可用，请恢复加密备份".into())
}
pub(super) fn ensure(id: &str) -> Result<(), String> {
    let entry = entry(id)?;
    match entry.get_password() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            let mut bytes = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut bytes);
            entry
                .set_password(&base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
                .map_err(|_| "无法保存管理凭证".into())
        }
        Err(_) => Err("系统凭据库不可用，未创建作品".into()),
    }
}

#[tauri::command]
pub(crate) async fn author_backup(
    id: String,
    destination: String,
    password: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if password.chars().count() < 10 {
            return Err("备份密码至少需要 10 个字符".into());
        }
        let token = secret(&id)?;
        let payload = serde_json::to_vec(&serde_json::json!({"schema":1,"id":id,"token":token}))
            .map_err(|_| "凭证备份失败")?;
        let encrypted = crate::packages::encrypt_package(&payload, &password)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|_| "目标已存在或无法写入，请选择新文件")?;
        file.write_all(&encrypted)
            .and_then(|_| file.sync_all())
            .map_err(|_| "凭证备份写入失败".into())
    })
    .await
    .map_err(|_| "凭证备份任务中断")?
}

#[tauri::command]
pub(crate) async fn author_restore(path: String, password: String) -> Result<AuthorWork, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if fs::metadata(&path).map_err(|_| "备份不存在")?.len() > 16 * 1024 {
            return Err("凭证备份大小无效".into());
        }
        let bytes = fs::read(path).map_err(|_| "备份读取失败")?;
        let payload = crate::packages::decrypt_package(&bytes, &password)
            .map_err(|_| "备份密码错误或文件损坏")?;
        let value: serde_json::Value =
            serde_json::from_slice(&payload).map_err(|_| "凭证备份格式无效")?;
        let id = value["id"].as_str().ok_or("缺少作品编号")?;
        let token = value["token"].as_str().ok_or("缺少管理凭证")?;
        checked_id(id)?;
        if value["schema"] != 1
            || token.len() != 43
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err("凭证备份格式无效".into());
        }
        // 新草稿可能尚未发送到服务；恢复不要求联网，首次同步再验证所有权。
        let _guard = lock()?;
        let mut state = load()?;
        if let Ok(existing) = secret(id) {
            if existing != token {
                return Err("本机已有不同的作品凭证，未覆盖".into());
            }
        }
        entry(id)?
            .set_password(token)
            .map_err(|_| "无法保存管理凭证")?;
        if !state.works.iter().any(|work| work.id == id) {
            state.works.push(AuthorWork {
                id: id.into(),
                title: "已恢复作品，等待同步".into(),
                ..Default::default()
            });
        }
        let work = state
            .works
            .iter()
            .find(|work| work.id == id)
            .unwrap()
            .clone();
        save(&state)?;
        Ok(work)
    })
    .await
    .map_err(|_| "凭证恢复任务中断")?
}
