//! 原图来源只读解析；插件产物必须匹配对应表摘要，外部目录由用户显式指定。
use crate::*;
use std::sync::OnceLock;
/// 禁用 PNG 仍可只读预览；只识别现有图片扩展，不扩大目录访问范围。
pub(crate) fn image_extension(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let name = name.strip_suffix(".disabled").unwrap_or(&name);
    Path::new(name).extension()?.to_str().map(str::to_string)
}
/// 图片协议仅可读取工作区或用户指定的原图目录，不暴露任意本地文件。
pub fn allowed_image(path: &Path) -> bool {
    let Ok(real) = path.canonicalize() else {
        return false;
    };
    if !matches!(
        image_extension(&real).as_deref(),
        Some("png" | "webp" | "jpg" | "jpeg" | "gif")
    ) {
        return false;
    }
    let settings = load_library().settings;
    let mut roots = vec![
        data_dir().join("vault"),
        data_dir().join("assets/outimg"),
        preview_cache_dir(),
    ];
    if !settings.modimg_path.is_empty() {
        roots.push(PathBuf::from(&settings.modimg_path));
    }
    if !settings.outimg_path.is_empty() {
        roots.push(PathBuf::from(settings.outimg_path));
    }
    roots
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .any(|p| real.starts_with(p))
}
type Fingerprint = (u64, SystemTime, String);
fn fingerprint(path: &Path) -> Option<String> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Fingerprint>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let meta = fs::metadata(path).ok()?;
    let stamp = meta.modified().ok()?;
    if let Some((size, time, hash)) = cache.lock().ok()?.get(path) {
        if *size == meta.len() && *time == stamp {
            return Some(hash.clone());
        }
    }
    let hash = sha256_file(path).ok()?;
    cache
        .lock()
        .ok()?
        .insert(path.to_path_buf(), (meta.len(), stamp, hash.clone()));
    Some(hash)
}
pub fn valid_local(path: &Path, expected: &str) -> bool {
    expected.len() == 64 && fingerprint(path).is_some_and(|h| h.eq_ignore_ascii_case(expected))
}
pub fn resolve(canonical: &str, value: &serde_json::Value, settings: &Settings) -> Option<String> {
    resolve_with_status(canonical, value, settings).0
}
/// 返回可读取的原图及来源；损坏或尚未提取的原图不会被伪装成成功。
pub fn resolve_with_status(
    canonical: &str,
    value: &serde_json::Value,
    settings: &Settings,
) -> (Option<String>, String) {
    let local = data_dir()
        .join("assets/outimg")
        .join(format!("{canonical}.png"));
    if value["sha256"]
        .as_str()
        .is_some_and(|hash| valid_local(&local, hash))
    {
        return (Some(local.to_string_lossy().into_owned()), "local".into());
    }
    if !settings.outimg_path.is_empty() {
        let file = Path::new(&settings.outimg_path).join(format!("{canonical}.png"));
        if valid_png(&file) {
            return (Some(file.to_string_lossy().into_owned()), "external".into());
        }
    }
    let cached = value["previewFile"]
        .as_str()
        .and_then(preview_path_for_file)
        .or_else(|| preview_path_for_file(&format!("{canonical}.webp")))
        .or_else(|| preview_path_for_file(&format!("{canonical}.png")))
        .filter(|p| allowed_image(Path::new(p)));
    let status = if cached.is_some() {
        "cache"
    } else if local.exists() {
        "invalid"
    } else if data_dir().join("assets/maps").is_dir() {
        "missing"
    } else {
        "not-extracted"
    };
    (cached, status.into())
}

/// 展示快照只查询校验缓存；首次校验交给原图后台工作线程。
pub(crate) fn resolve_cached(
    canonical: &str,
    value: &serde_json::Value,
    settings: &Settings,
) -> (Option<String>, String) {
    let local = data_dir()
        .join("assets/outimg")
        .join(format!("{canonical}.png"));
    let mut pending = false;
    if let Some(expected) = value["sha256"].as_str().filter(|v| v.len() == 64) {
        match crate::original_cache::lookup(&local, false) {
            Some(Ok(hash)) if hash.eq_ignore_ascii_case(expected) => {
                return (Some(local.to_string_lossy().into_owned()), "local".into())
            }
            None => pending = true,
            _ => {}
        }
    }
    if !settings.outimg_path.is_empty() {
        let external = Path::new(&settings.outimg_path).join(format!("{canonical}.png"));
        match crate::original_cache::lookup(&external, true) {
            Some(Ok(_)) => {
                return (
                    Some(external.to_string_lossy().into_owned()),
                    "external".into(),
                )
            }
            None => pending = true,
            _ => {}
        }
    }
    // 预览路径由现有解析器限制在预览缓存内，协议读取时仍检查允许的根目录。
    let preview = value["previewFile"]
        .as_str()
        .and_then(preview_path_for_file)
        .or_else(|| preview_path_for_file(&format!("{canonical}.webp")))
        .or_else(|| preview_path_for_file(&format!("{canonical}.png")));
    let status = if pending {
        "checking"
    } else if preview.is_some() {
        "cache"
    } else if local.exists() {
        "invalid"
    } else if data_dir().join("assets/maps").is_dir() {
        "missing"
    } else {
        "not-extracted"
    };
    (preview, status.into())
}

pub(crate) fn external_names_cached(settings: &Settings) -> Vec<serde_json::Value> {
    if settings.outimg_path.is_empty() {
        return vec![];
    }
    fs::read_dir(&settings.outimg_path).into_iter().flatten().flatten().filter_map(|entry| {
        if !entry.file_type().ok()?.is_file() || image_extension(&entry.path()).as_deref() != Some("png") { return None; }
        let parsed = parse_filename(&entry.file_name().to_string_lossy())?;
        Some(serde_json::json!({"canonical":parsed.canonical,"section":parsed.section_key,"heroId":parsed.hero_id}))
    }).collect()
}
fn valid_png(path: &Path) -> bool {
    let mut header = [0u8; 24];
    fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut header))
        .is_ok()
        && &header[..8] == b"\x89PNG\r\n\x1a\n"
        && header[16..20] != [0; 4]
        && header[20..24] != [0; 4]
}
pub fn external_names(settings: &Settings) -> Vec<serde_json::Value> {
    if settings.outimg_path.is_empty() {
        return vec![];
    }
    fs::read_dir(&settings.outimg_path).into_iter().flatten().flatten().filter_map(|entry|{
  if !valid_png(&entry.path()){return None}
  let name=entry.file_name().to_string_lossy().to_string();let parsed=parse_filename(&name)?;
  Some(serde_json::json!({"canonical":parsed.canonical,"section":parsed.section_key,"heroId":parsed.hero_id}))
 }).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replaced_original_invalidates_cached_hash() {
        let root = crate::staging::create("original-test").unwrap();
        let path = root.join("image.png");
        fs::write(&path, b"one").unwrap();
        let hash = sha256_file(&path).unwrap();
        assert!(valid_local(&path, &hash));
        fs::write(&path, b"changed").unwrap();
        assert!(!valid_local(&path, &hash));
        crate::staging::remove(root).unwrap();
    }
}
