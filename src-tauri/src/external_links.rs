//! 用户点击项目引用时交给系统浏览器，拒绝非 HTTPS 地址。
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn open_external(url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "链接地址无效")?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("仅支持 HTTPS 项目链接".into());
    }
    #[cfg(windows)]
    let mut command = std::process::Command::new("explorer");
    #[cfg(target_os = "linux")]
    let mut command = std::process::Command::new("xdg-open");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .arg(url)
        .spawn()
        .map_err(|e| format!("无法打开浏览器：{e}"))?;
    Ok(())
}
