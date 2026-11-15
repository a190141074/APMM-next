//! 配色文件只导出经过验证的数据，屏幕取色在独立线程持有并释放原生资源。
use crate::appearance::{AppearanceSettings, ThemeScheme};

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn theme_export(destination: String, scheme: ThemeScheme) -> Result<(), String> {
    let check = AppearanceSettings { schemes: vec![ThemeScheme { id: uuid::Uuid::new_v4().to_string(), ..scheme.clone() }], ..Default::default() };
    check.validate()?;
    let data = serde_json::to_vec_pretty(&serde_json::json!({ "type": "apmm-theme", "schema": 1, "scheme": scheme })).map_err(|e| e.to_string())?;
    std::fs::write(destination, data).map_err(|e| format!("导出配色失败：{e}"))
}

#[tauri::command]
pub(crate) async fn screen_pick_color() -> Result<Option<String>, String> {
    #[cfg(windows)]
    { tauri::async_runtime::spawn_blocking(crate::screen_picker::pick).await.map_err(|e| e.to_string())? }
    #[cfg(not(windows))]
    { Err("当前系统不支持屏幕取色，请使用调色盘或数值输入。".into()) }
}
