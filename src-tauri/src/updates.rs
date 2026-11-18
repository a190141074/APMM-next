//! 便携更新命令；下载收据、发行清单与外部助手共同约束替换范围。
use crate::portable_update as portable;
use crate::*;
use tauri::Manager;
pub(crate) const UPDATE_RUNTIME_FILES: [&str; 4] = [
    "astral-mod-manager.exe",
    "astral-mod-manager.exe.manifest",
    "WebView2Loader.dll",
    "api-ms-win-core-synch-l1-2-0.dll",
];
fn now_secs() -> u64 {
    unix_now()
}
fn emit_update_progress(app: &tauri::AppHandle, phase: &str, message: &str) {
    let _ = app.emit(
        "app-update-progress",
        serde_json::json!({"phase":phase,"message":message}),
    );
}
pub(crate) fn find_update_payload(root: &Path) -> Option<PathBuf> {
    let mut level = vec![root.to_path_buf()];
    for _ in 0..4 {
        let mut next = Vec::new();
        for dir in level {
            if UPDATE_RUNTIME_FILES.iter().all(|n| dir.join(n).is_file()) {
                return Some(dir);
            }
            if let Ok(entries) = fs::read_dir(dir) {
                next.extend(
                    entries
                        .flatten()
                        .filter(|e| e.path().is_dir())
                        .map(|e| e.path()),
                );
            }
        }
        level = next;
    }
    None
}
fn pending_path() -> PathBuf {
    data_dir().join("updates/pending.json")
}
fn pending_job() -> Result<PathBuf, String> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(pending_path()).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    Ok(PathBuf::from(value["job"].as_str().ok_or("更新记录无效")?))
}
pub(crate) fn spawn_helper(path: &Path, recover: bool) -> Result<std::process::Child, String> {
    let helper = path
        .parent()
        .ok_or("更新目录无效")?
        .join("updater-runner.exe");
    let mut cmd = std::process::Command::new(helper);
    cmd.arg(path);
    if recover {
        cmd.arg("--recover");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.spawn().map_err(|e| e.to_string())
}
pub fn recover_at_startup() -> Result<bool, String> {
    let Ok(path) = pending_job() else {
        return Ok(false);
    };
    let mut job = portable::load_job(&path)?;
    if job.kind == "msi" {
        return Ok(false);
    }
    if job.status == "awaiting-startup" {
        let verified = job.version == env!("CARGO_PKG_VERSION")
            && job
                .files
                .iter()
                .all(|f| portable::verify_file(&job.destination, f).is_ok());
        if !verified {
            job.pid = std::process::id();
            portable::write_json(&path, &job)?;
            spawn_helper(&path, true)?;
            return Ok(true);
        }
        if crate::helper_process::acknowledged(&job) {
            job.status = "complete".into();
            job.message = "新版工作区已成功启动".into();
            portable::write_json(&path, &job)?;
            portable::write_json(&data_dir().join("updates/last-result.json"), &job)?;
        }
    }
    if matches!(
        job.status.as_str(),
        "applying" | "prepared" | "recovery-required"
    ) {
        job.pid = std::process::id();
        portable::write_json(&path, &job)?;
        spawn_helper(&path, true)?;
        return Ok(true);
    }
    Ok(false)
}
#[tauri::command]
pub async fn update_startup_status() -> Result<serde_json::Value, String> {
    let mut awaiting = false;
    if let Ok(path) = pending_job() {
        let job = portable::load_job(&path)?;
        if job.status == "awaiting-startup" && job.version == env!("CARGO_PKG_VERSION") {
            awaiting = true;
            portable::write_json(
                &data_dir()
                    .join("updates")
                    .join(format!("ack-{}.json", job.id)),
                &serde_json::json!({"version":job.version,"ready":true}),
            )?;
        }
    }
    let result = data_dir().join("updates/last-result.json");
    if awaiting {
        for _ in 0..50 {
            if result.is_file() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    // 原子领取一次性提示，保留已读记录供诊断，不在每次启动重复展示。
    let consumed = data_dir()
        .join("updates")
        .join(format!("result-read-{}.json", uuid::Uuid::new_v4()));
    if fs::rename(&result, &consumed).is_err() {
        return Ok(serde_json::Value::Null);
    }
    Ok(fs::read(consumed)
        .ok()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub fn update_download_status() -> serde_json::Value {
    fs::read(data_dir().join("updates/download-receipt.json"))
        .ok()
        .and_then(|v| serde_json::from_slice::<serde_json::Value>(&v).ok())
        .filter(|v| v["path"].as_str().is_some_and(|p| Path::new(p).is_file()))
        .filter(|v| {
            v["version"].as_str().is_some_and(|version| {
                update_channel::version_cmp(version, env!("CARGO_PKG_VERSION"))
                    == std::cmp::Ordering::Greater
            })
        })
        .unwrap_or(serde_json::Value::Null)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn update_check(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let result = tauri::async_runtime::spawn_blocking(|| {
        update_channel::fetch_update_manifest_with_source()
    })
    .await
    .map_err(|e| format!("检查更新失败：{}", e))?;
    let (manifest, _source) = result.map_err(|error| {
        let lower = error.to_ascii_lowercase();
        if error.contains("格式无效") || error.contains("缺少版本号") || lower.contains("utf-8")
        {
            format!("更新信息解析失败，请稍后重试。（技术详情：{}）", error)
        } else if error.contains("HTTP")
            || error.contains("获取更新清单")
            || lower.contains("timeout")
            || lower.contains("connection")
        {
            format!(
                "无法连接更新服务，请检查网络后重试。（技术详情：{}）",
                error
            )
        } else {
            format!("更新服务暂不可用，请稍后重试。（技术详情：{}）", error)
        }
    })?;
    let available =
        update_channel::version_cmp(&manifest.version, &current) == std::cmp::Ordering::Greater;
    let stamp = now_secs();
    let mut settings = load_library().settings;
    if let Ok(_guard) = state.mutation.lock() {
        let mut lib = load_library();
        lib.settings.update_last_checked_at = Some(stamp);
        if save_library(&lib).is_ok() {
            settings = lib.settings;
        } else {
            settings.update_last_checked_at = Some(stamp);
        }
    }
    Ok(serde_json::json!({
        "available": available,
        "currentVersion": current,
        "latestVersion": manifest.version,
        "notes": manifest.notes,
        "publishedAt": manifest.published_at,
        "assets": manifest.assets,
        "autoCheck": settings.update_auto_check,
        "intervalMinutes": settings.update_interval_minutes(),
        "skippedVersion": settings.update_skipped_version,
        "lastCheckedAt": settings.update_last_checked_at,
    }))
}

fn update_download_blocking(
    version: String,
    platform: Option<String>,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<String, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("用户取消下载".into());
    }
    emit_update_progress(&app, "manifest", "正在检测更新通道…");
    let manifest = update_channel::check_for_update(env!("CARGO_PKG_VERSION"))?
        .ok_or_else(|| "当前已是最新版本".to_string())?;
    if !version.eq_ignore_ascii_case(&manifest.version) {
        return Err(format!(
            "请求版本 {} 与远端最新版本 {} 不一致",
            version, manifest.version
        ));
    }
    let platform = platform.unwrap_or_else(|| "win-x86_64".to_string());
    let asset = update_channel::pick_asset(&manifest, &platform, crate::installation::kind())
        .ok_or_else(|| format!("未找到适用于 {} 的安装包", platform))?;
    emit_update_progress(
        &app,
        "download",
        &format!("正在通过更新通道下载 {}", asset.url),
    );
    let path = update_channel::download_update_asset(asset, &cancel, |done, total, speed| {
        app.state::<tasks::TaskManager>()
            .download_progress(&app, done, total, speed)
    })?;
    portable::write_json(
        &data_dir().join("updates/download-receipt.json"),
        &serde_json::json!({"version":version,"path":path,"bytes":asset.bytes,"sha256":asset.sha256,"kind":asset.kind}),
    )?;
    emit_update_progress(&app, "complete", "安装包已下载并完成校验");
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn update_download(
    version: String,
    platform: Option<String>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    if state.update_busy.swap(true, Ordering::AcqRel) {
        return Err("更新下载任务正在进行中".into());
    }
    state.update_cancel.store(false, Ordering::Release);
    let busy = Arc::clone(&state.update_busy);
    let cancel = Arc::clone(&state.update_cancel);
    let result = tauri::async_runtime::spawn_blocking(move || {
        update_download_blocking(version, platform, app, cancel)
    })
    .await
    .map_err(|error| format!("更新下载任务失败：{}", error));
    busy.store(false, Ordering::Release);
    result?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn update_cancel(state: tauri::State<AppState>) -> Result<(), String> {
    state.update_cancel.store(true, Ordering::Release);
    Ok(())
}
