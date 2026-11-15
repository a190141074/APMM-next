//! 只读原图生成独立缩略图；缓存按内容摘要和解码版本隔离，不修改导出或应用载荷。
use crate::*;
use std::sync::OnceLock;

pub async fn load(source: PathBuf) -> Result<Vec<u8>, String> {
    static LANE: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    let _permit = LANE
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(1)))
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        generate(&source, &data_dir().join("cache/thumbnails"))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn generate(source: &Path, cache: &Path) -> Result<Vec<u8>, String> {
    static LANE: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = LANE
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "缩略图生成锁不可用")?;
    let hash = sha256_file(source)?;
    let target = cache.join(format!("{hash}-384-v1.png"));
    if let Ok(bytes) = fs::read(&target) {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Ok(bytes);
        }
    }
    let mut reader = image::ImageReader::open(source)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| e.to_string())?;
    let thumbnail = decoded.thumbnail(384, 384);
    let mut output = Cursor::new(Vec::new());
    thumbnail
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let bytes = output.into_inner();
    ensure_dir(cache).map_err(|e| e.to_string())?;
    let pending = target.with_extension("png.pending");
    fs::write(&pending, &bytes).map_err(|e| e.to_string())?;
    fs::rename(&pending, &target).map_err(|e| e.to_string())?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_preserves_source_alpha_and_uses_new_identity_after_source_change() {
        let root = crate::staging::create("thumbnail-test").unwrap();
        let source = root.join("original.png");
        let pixels = image::RgbaImage::from_fn(800, 400, |x, _| {
            image::Rgba([23, 45, 67, if x < 400 { 0 } else { 255 }])
        });
        pixels.save(&source).unwrap();
        let before = sha256_file(&source).unwrap();
        let cache = root.join("cache");
        let png = generate(&source, &cache).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (384, 192));
        assert_eq!(decoded.get_pixel(0, 0)[3], 0);
        assert_eq!(decoded.get_pixel(383, 0)[3], 255);
        assert_eq!(sha256_file(&source).unwrap(), before);
        assert_eq!(generate(&source, &cache).unwrap(), png);
        image::RgbaImage::new(16, 16).save(&source).unwrap();
        assert_ne!(generate(&source, &cache).unwrap(), png);
        assert_eq!(fs::read_dir(cache).unwrap().count(), 2);
        crate::staging::remove(root).unwrap();
    }
}
