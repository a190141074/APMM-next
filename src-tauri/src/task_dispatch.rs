//! 把现有命令放入统一任务中心；后台生命周期独立于页面。
use crate::*;
use serde_json::{json, Value};
use tauri::Manager;
#[tauri::command]
pub fn task_execute(
    kind: String,
    title: String,
    args: Value,
    app: tauri::AppHandle,
) -> Result<String, String> {
    if !matches!(
        kind.as_str(),
        "scan"
            | "images-import"
            | "preview"
            | "apx-install"
            | "apx-restore"
            | "apx-cache"
            | "apx-export"
            | "workshop-install"
            | "workshop-uninstall"
            | "package-export"
            | "quick-export"
            | "package-commit"
            | "migration"
            | "update-download"
            | "aux-install"
            | "aux-run"
    ) {
        return Err("未知任务类型".into());
    }
    let manager = app.state::<crate::tasks::TaskManager>();
    let (id, cancel) = manager.begin(&title, &kind, args.clone());
    let returned = id.clone();
    let lane = manager.lane.clone();
    tauri::async_runtime::spawn(async move {
        let _permit = lane.acquire_owned().await.unwrap();
        if cancel.load(Ordering::Acquire) {
            app.state::<crate::tasks::TaskManager>().finish(
                &app,
                &id,
                Err("排队任务已取消".into()),
            );
            return;
        }
        app.state::<crate::tasks::TaskManager>().start(&app, &id);
        let value = |key: &str| args[key].as_str().unwrap_or_default().to_string();
        let platform = args["platform"].as_str().map(String::from);
        let result: Result<Value, String> = async {
            match kind.as_str() {
                "scan" => {
                    let handle = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        let state = handle.state::<AppState>();
                        let _guard = state.mutation.lock().map_err(|_| "库写入锁不可用")?;
                        reconcile_inner(Some(&handle)).map(|v| json!(v))
                    })
                    .await
                    .map_err(|e| e.to_string())?
                }
                "images-import" => {
                    let handle = app.clone();
                    let input = args.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        let paths: Vec<String> = serde_json::from_value(input["paths"].clone())
                            .map_err(|e| e.to_string())?;
                        if let Some(expected) = input["expectedHashes"].as_object() {
                            for path in &paths {
                                crate::tasks::checkpoint(&handle)?;
                                if expected.get(path).and_then(Value::as_str)
                                    != Some(sha256_file(Path::new(path))?.as_str())
                                {
                                    return Err("图片在预览后发生变化，请重新识别".into());
                                }
                            }
                        }
                        crate::tasks::commit_phase(&handle)?;
                        let output = crate::image_import::import_selected(input, handle.state())?;
                        // 图片导入允许逐项失败，保留每项结果供结果页准确统计和定位。
                        Ok::<Value, String>(json!(output))
                    })
                    .await
                    .map_err(|e| e.to_string())?
                }
                "preview" => preview_download(Some(false), app.clone(), app.state())
                    .await
                    .map(|v| json!(v)),
                "apx-install" => plugin_install(
                    platform,
                    Some(args["force"].as_bool().unwrap_or(false)),
                    app.clone(),
                    app.state(),
                )
                .await
                .map(|v| json!(v)),
                "apx-restore" => plugin_restore(platform, app.clone(), app.state())
                    .await
                    .map(|v| json!(v)),
                "apx-cache" => plugin_prepare_cache(platform, app.clone(), app.state())
                    .await
                    .map(|v| json!(v)),
                "apx-export" => plugin_export(
                    platform,
                    value("destination"),
                    Some(args["useCached"].as_bool().unwrap_or(false)),
                    app.clone(),
                    app.state(),
                )
                .await
                .map(|v| json!(v)),
                "workshop-install" => workshop_install(value("id"), app.clone(), app.state())
                    .await
                    .map(|v| json!(v)),
                "workshop-uninstall" => workshop_uninstall(value("id"), app.state())
                    .await
                    .map(|v| json!(v)),
                "package-export" => package_export(
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?,
                    app.clone(),
                    app.state(),
                )
                .await
                .map(|v| json!(v)),
                "quick-export" => quick_export(
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?,
                    app.clone(),
                    app.state(),
                )
                .await
                .map(|v| json!(v)),
                "package-commit" => package_import_queue_commit(
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?,
                    app.clone(),
                    app.state(),
                )
                .await
                .map(|v| json!(v)),
                "update-download" => {
                    update_download(value("version"), platform, app.clone(), app.state())
                        .await
                        .map(|v| json!(v))
                }
                "migration" => {
                    let handle = app.clone();
                    let task = id.clone();
                    let flag = cancel.clone();
                    let source = PathBuf::from(value("source"));
                    tauri::async_runtime::spawn_blocking(move || {
                        crate::migration::copy_library(&source, &handle, &task, &flag)
                    })
                    .await
                    .map_err(|e| e.to_string())?
                }
                "aux-install" => crate::auxiliary::install(&app, &id, args.clone(), &cancel).await,
                "aux-run" => crate::auxiliary::run(&app, &id, args.clone(), &cancel).await,
                _ => Err("未识别任务".into()),
            }
        }
        .await;
        crate::resource_rules::reload_catalog_names();
        app.state::<crate::tasks::TaskManager>()
            .finish(&app, &id, result);
    });
    Ok(returned)
}
