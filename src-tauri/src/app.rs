use std::fs;
use std::path::Path;
use std::sync::{atomic::AtomicBool, Arc, Mutex};

use crate::{cleanup_library_inner, migrate_vault, mime_of, not_found, percent_decode, AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run<F>(handler: F)
where
    F: Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static,
{
    // 便携版浏览器资料留在程序数据目录；临时文件使用配置的工作盘。
    if std::env::var_os("WEBVIEW2_USER_DATA_FOLDER").is_none() {
        std::env::set_var(
            "WEBVIEW2_USER_DATA_FOLDER",
            crate::data_dir().join("webview"),
        );
    }
    let temp = crate::staging::work_root().join("temp");
    if fs::create_dir_all(&temp).is_ok() {
        std::env::set_var("TEMP", &temp);
        std::env::set_var("TMP", &temp);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(directory) = exe.parent() {
            if let Err(error) = crate::update_bootstrap::finish(directory, env!("CARGO_PKG_VERSION")) {
                crate::write_log("error", "update.bootstrap", &error, None);
                eprintln!("{error}");
                return;
            }
        }
    }
    match crate::updates::recover_at_startup() {
        Ok(true) => return,
        Err(error) => crate::write_log("error", "update.recovery", &error, None),
        _ => {}
    }
    if let Err(error) = crate::migration_journal::recover(&crate::data_dir()) {
        crate::write_log("error", "migration.recovery", &error, None);
        eprintln!("{error}");
        return;
    }
    let _migrated = migrate_vault();
    let _cleaned = cleanup_library_inner();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(crate::tasks::TaskManager::restore())
        .manage(crate::directory_monitor::DirectoryMonitor::default())
        .manage(AppState {
            scan: Mutex::new(None),
            mutation: Arc::new(Mutex::new(())),
            scan_busy: Arc::new(AtomicBool::new(false)),
            preview_busy: Arc::new(AtomicBool::new(false)),
            preview_cancel: Arc::new(AtomicBool::new(false)),
            plugin_cancel: Arc::new(AtomicBool::new(false)),
            apx_cache_busy: Arc::new(Mutex::new(())),
            update_busy: Arc::new(AtomicBool::new(false)),
            update_cancel: Arc::new(AtomicBool::new(false)),
            import_queue: Arc::new(Mutex::new(None)),
        })
        .register_uri_scheme_protocol("vault", |_ctx, request| {
            let uri = request.uri().to_string();
            let after = uri.split("://").nth(1).unwrap_or("");
            let encoded = match after.find(".localhost/") {
                Some(i) => &after[i + ".localhost/".len()..],
                None => after.strip_prefix("localhost/").unwrap_or(after),
            };
            // 修订参数只影响 WebView 请求缓存，不能成为文件路径的一部分。
            let path_str = percent_decode(encoded.split('?').next().unwrap_or(encoded));
            if !crate::originals::allowed_image(Path::new(&path_str)) {
                return not_found();
            }
            let ext = crate::originals::image_extension(Path::new(&path_str)).unwrap_or_default();
            let mime = mime_of(&ext);

            match fs::read(Path::new(&path_str)) {
                Ok(data) => tauri::http::Response::builder()
                    .status(200)
                    .header("Content-Type", mime)
                    .header("Cache-Control", "no-store")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(data)
                    .unwrap_or_else(|_| not_found()),
                Err(_) => not_found(),
            }
        })
        .register_asynchronous_uri_scheme_protocol("thumb", |_ctx, request, responder| {
            let uri = request.uri().to_string();
            let after = uri.split("://").nth(1).unwrap_or("");
            let encoded = after
                .split_once(".localhost/")
                .map(|(_, p)| p)
                .unwrap_or_else(|| after.strip_prefix("localhost/").unwrap_or(after));
            let decoded = percent_decode(encoded.split('?').next().unwrap_or(encoded));
            let path = std::path::PathBuf::from(decoded);
            tauri::async_runtime::spawn(async move {
                if !crate::originals::allowed_image(&path) {
                    responder.respond(not_found());
                    return;
                }
                let response = match crate::thumbnails::load(path).await {
                    Ok(bytes) => tauri::http::Response::builder()
                        .status(200)
                        .header("Content-Type", "image/png")
                        .header("Cache-Control", "no-store")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(bytes)
                        .unwrap_or_else(|_| not_found()),
                    Err(_) => not_found(),
                };
                responder.respond(response);
            });
        })
        .invoke_handler(move |invoke| {
            if !crate::update_gate::allowed(invoke.message.command()) {
                invoke.resolver.reject("正在准备安装更新，新的写入操作暂时关闭");
                return true;
            }
            handler(invoke)
        })
        .setup(|app| {
            crate::tasks::bridge(app.handle());
            crate::directory_monitor::start(app.handle());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                use tauri::Manager;
                app.state::<crate::directory_monitor::DirectoryMonitor>()
                    .stop();
            }
        });
}
