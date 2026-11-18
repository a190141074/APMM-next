//! 有界下载、归档检查和独立暂存，原 PNG 字节保持不变。
use super::*;
pub(super) fn cache_archive(
    base: &str,
    work: &Work,
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    use tauri::Manager;
    let state = app.state::<crate::tasks::TaskManager>();
    let flag = state
        .current()
        .map(|(_, flag)| flag)
        .unwrap_or_else(|| std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    crate::downloads::fetch(
        crate::downloads::Download {
            identity: &format!("workshop:{}:{}", work.id, work.version),
            urls: vec![format!("{base}/{}", work.archive)],
            bytes: work.bytes,
            sha256: &work.sha256,
        },
        &flag,
        |done, total, speed| state.download_progress(app, done, total, speed),
    )
}

pub(super) fn unpack_checked(
    work: &Work,
    archive: &Path,
    stage: &Path,
) -> Result<Vec<PathBuf>, String> {
    let file = fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("作品 ZIP 损坏：{e}"))?;
    if zip.len() != work.files.len() {
        return Err("ZIP 文件数与清单不符".into());
    }
    ensure_dir(stage).map_err(|e| e.to_string())?;
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if entry.is_dir()
            || !safe_name(&name)
            || !seen.insert(name.to_lowercase())
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("ZIP 含非法路径或重复文件".into());
        }
        let meta = work
            .files
            .iter()
            .find(|item| item.name == name)
            .ok_or("ZIP 含清单外文件")?;
        if entry.size() != meta.bytes {
            return Err(format!("{} 大小不符", name));
        }
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(meta.bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != meta.bytes
            || !sha256_bytes(&bytes).eq_ignore_ascii_case(&meta.sha256)
            || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        {
            return Err(format!("{} 校验失败", name));
        }
        validate_png(&bytes)?;
        let path = stage.join(name);
        fs::write(&path, bytes).map_err(|e| e.to_string())?;
        paths.push(path);
    }
    Ok(paths)
}

/// 在分配像素缓冲前设置解码预算，拒绝只有 PNG 头的伪图片。
pub(crate) fn validate_png(bytes: &[u8]) -> Result<(), String> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("只支持 PNG 资源".into());
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map_err(|_| "PNG 损坏或解码大小超限")?;
    Ok(())
}
