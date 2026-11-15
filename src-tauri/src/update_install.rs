//! 安装交接：完整校验、等待写入清空、助手确认准备完成后才请求正常退出。
use crate::{portable_update as portable, *};
use tauri::Manager;

fn prepare(version: String, asset_path: String) -> Result<(PathBuf, portable::Job), String> {
    let receipt: serde_json::Value = serde_json::from_slice(
        &fs::read(data_dir().join("updates/download-receipt.json"))
            .map_err(|_| "缺少已校验的更新下载记录")?,
    )
    .map_err(|e| e.to_string())?;
    let kind = crate::installation::kind();
    let receipt_kind = receipt["kind"].as_str().unwrap_or("portable");
    if receipt["version"] != version
        || receipt["path"] != asset_path
        || receipt_kind != kind
        || !crate::downloads::checked(
            Path::new(&asset_path),
            receipt["bytes"].as_u64().unwrap_or(0),
            receipt["sha256"].as_str().unwrap_or(""),
        )
    {
        return Err("更新包与下载记录、安装类型或摘要不一致".into());
    }
    let incoming =
        semver::Version::parse(version.trim_start_matches('v')).map_err(|e| e.to_string())?;
    if incoming < semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap() {
        return Err("拒绝安装较旧版本".into());
    }
    let stage = crate::staging::create("application-update")?;
    let unpacked = stage.join("payload");
    ensure_dir(&unpacked).map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let destination = exe.parent().ok_or("程序目录无效")?.to_path_buf();
    let mut files = vec![];
    let mut installer = None;
    if kind == "portable" {
        let manifest = crate::update_package::unpack(Path::new(&asset_path), &unpacked, &version)?;
        crate::update_package::precheck(&manifest, env!("CARGO_PKG_VERSION"), &destination)?;
        if crate::installation::kind_at(&unpacked) != "portable" {
            return Err("便携更新包包含错误的安装身份".into());
        }
        files = manifest.files;
        let path = unpacked.join("release-manifest.json");
        files.push(portable::FileRecord {
            path: "release-manifest.json".into(),
            bytes: fs::metadata(&path).map_err(|e| e.to_string())?.len(),
            sha256: portable::hash(&path)?,
        });
    } else {
        let path = unpacked.join("installer.msi");
        fs::copy(&asset_path, &path).map_err(|e| e.to_string())?;
        let record = portable::FileRecord {
            path: "installer.msi".into(),
            bytes: receipt["bytes"].as_u64().unwrap_or(0),
            sha256: receipt["sha256"].as_str().unwrap_or("").into(),
        };
        crate::msi_update::validate_package(&path, record.bytes, &record.sha256, &version)?;
        installer = Some(record);
    }
    fs::copy(
        destination.join("astral-updater.exe"),
        stage.join("updater-runner.exe"),
    )
    .map_err(|e| format!("复制更新助手失败：{e}"))?;
    let job = portable::Job {
        schema: 1,
        id: uuid::Uuid::new_v4().to_string(),
        source: unpacked,
        destination,
        backup: stage.join("backup"),
        pid: std::process::id(),
        version,
        files,
        existed: vec![],
        status: "new".into(),
        message: String::new(),
        kind: kind.into(),
        data_directory: Some(data_dir()),
        installer,
    };
    let path = stage.join("job.json");
    portable::write_json(&path, &job)?;
    Ok((path, job))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn update_apply(
    version: String,
    asset_path: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let gate = crate::update_gate::Guard::enter()?;
    // 闸门先阻止新请求；已经登记的排队任务仍按原顺序完成。
    while app.state::<tasks::TaskManager>().active() {
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    let lane = app.state::<tasks::TaskManager>().lane.clone();
    let permit = lane.acquire_owned().await.map_err(|e| e.to_string())?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let state = app.state::<AppState>();
        let _guard = state.mutation.lock().map_err(|_| "库写入锁不可用")?;
        let (path, job) = prepare(version, asset_path)?;
        let pending = data_dir().join("updates/pending.json");
        let previous = data_dir().join("updates/last-result.json");
        if previous.exists() {
            fs::rename(
                &previous,
                data_dir().join(format!("updates/result-previous-{}.json", job.id)),
            )
            .map_err(|e| e.to_string())?;
        }
        portable::write_json(&pending, &serde_json::json!({"job":path}))?;
        let mut child = crate::updates::spawn_helper(&path, false).map_err(|error| {
            let _ = fs::remove_file(&pending);
            error
        })?;
        let marker = path
            .parent()
            .unwrap()
            .join(format!("ready-{}.json", job.id));
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            let ready = fs::read(&marker)
                .ok()
                .and_then(|v| serde_json::from_slice::<serde_json::Value>(&v).ok())
                .is_some_and(|v| v["ready"] == true && v["id"] == job.id);
            if ready {
                break;
            }
            if child.try_wait().map_err(|e| e.to_string())?.is_some() || Instant::now() >= deadline
            {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&pending);
                let error = fs::read_to_string(path.with_extension("error.log"))
                    .unwrap_or_else(|_| "更新助手未完成准备，当前程序保持打开".into());
                return Err(error);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        app.state::<crate::directory_monitor::DirectoryMonitor>()
            .stop();
        app.exit(0);
        Ok("更新助手已接管".into())
    })
    .await
    .map_err(|e| e.to_string())?;
    if result.is_ok() {
        std::mem::forget(gate);
    }
    result
}
