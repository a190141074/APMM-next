//! Settings and preset commands. Persistent settings remain owned by Library.

use crate::{load_library, save_library, AppState, Settings};

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn settings_get(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    let mutation = state.mutation.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
        Ok(load_library().settings)
    })
    .await
    .map_err(|error| format!("读取设置失败：{error}"))?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn settings_set(
    patch: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<Settings, String> {
    let mutation = state.mutation.clone();
    // 配色迁移和设置保存仍持有同一把库锁，但不在窗口线程等待后台扫描。
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
        settings_set_inner(patch)
    })
    .await
    .map_err(|error| format!("保存设置失败：{error}"))?
}

fn settings_set_inner(patch: serde_json::Value) -> Result<Settings, String> {
    let mut lib = load_library();
    if let Some(o) = patch.as_object() {
        if let Some(value) = o.get("authorMode").and_then(|value| value.as_bool()) {
            lib.settings.author_mode = value;
        }
        if let Some(value) = o.get("appearance") {
            let appearance: crate::appearance::AppearanceSettings =
                serde_json::from_value(value.clone()).map_err(|_| "配色设置格式无效")?;
            appearance.validate()?;
            lib.settings.appearance = Some(appearance);
        }
        if let Some(v) = o.get("modimgPath").and_then(|v| v.as_str()) {
            lib.settings.modimg_path = v.to_string();
        }
        if let Some(v) = o.get("assetRoot").and_then(|v| v.as_str()) {
            lib.settings.asset_root = v.to_string();
        }
        if let Some(v) = o.get("outimgPath").and_then(|v| v.as_str()) {
            lib.settings.outimg_path = v.trim().to_string();
        }
        if let Some(v) = o.get("monitorDirectories").and_then(|v| v.as_bool()) {
            lib.settings.monitor_directories = v;
        }
        if let Some(v) = o.get("previewDownloadPrompt").and_then(|v| v.as_bool()) {
            lib.settings.preview_download_prompt = v;
        }
        if let Some(v) = o.get("viewMode").and_then(|v| v.as_str()) {
            lib.settings.view_mode = Some(v.to_string());
        }
        if let Some(v) = o.get("thumbSize").and_then(|v| v.as_str()) {
            lib.settings.thumb_size = Some(v.to_string());
        }
        if let Some(v) = o.get("resourceFilter").and_then(|v| v.as_str()) {
            if matches!(v, "all" | "applied" | "has-mod" | "no-mod") {
                lib.settings.resource_filter = Some(v.to_string());
            }
        }
        if let Some(v) = o.get("searchMode").and_then(|v| v.as_str()) {
            if matches!(v, "smart" | "id") {
                lib.settings.search_mode = Some(v.to_string());
            }
        }
        if let Some(v) = o.get("pluginPlatform").and_then(|v| v.as_str()) {
            if crate::plugin_platform_info(v).is_some() {
                lib.settings.plugin_platform = Some(v.to_string());
            }
        }
        // 兼容旧客户端的字段，但写入范围限定为该次请求明确或当前选中的平台。
        let platform = lib
            .settings
            .plugin_platform
            .clone()
            .unwrap_or_else(|| crate::PLUGIN_DEFAULT_PLATFORM.into());
        for (field, which) in [
            ("pluginLocallowDir", "locallow"),
            ("pluginDataDir", "data"),
            ("pluginRootDir", "root"),
        ] {
            if let Some(v) = o.get(field).and_then(|v| v.as_str()) {
                crate::apx_directories::set_directory(
                    &mut lib.settings,
                    &platform,
                    which,
                    v.into(),
                )?;
            }
        }
        if let Some(v) = o.get("updateAutoCheck").and_then(|v| v.as_bool()) {
            lib.settings.update_auto_check = v;
        }
        if let Some(v) = o.get("updateIntervalMinutes").and_then(|v| v.as_u64()) {
            lib.settings.update_interval_minutes =
                Some(crate::normalized_update_interval_minutes(v));
            lib.settings.update_interval_hours = None;
        } else if let Some(v) = o.get("updateIntervalHours").and_then(|v| v.as_u64()) {
            let hours = u32::try_from(v).unwrap_or(0);
            lib.settings.update_interval_minutes =
                Some(crate::legacy_update_interval_minutes(hours));
            lib.settings.update_interval_hours = None;
        }
        if let Some(v) = o.get("updateSkippedVersion").and_then(|v| v.as_str()) {
            let trimmed = v.trim().to_string();
            lib.settings.update_skipped_version = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            };
        }
        if let Some(v) = o.get("updateLastCheckedAt").and_then(|v| v.as_u64()) {
            lib.settings.update_last_checked_at = Some(v);
        }
    }
    save_library(&lib)?;
    Ok(lib.settings)
}
