//! 目录枚举不跟随链接，只收集 PNG 和禁用图片。
use crate::*;

/// 文件标识用于跨目录重命名关联，不能用相同内容哈希合并不同候选的信息。
pub(crate) fn file_identity(path: &Path) -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{
            Foundation::HANDLE,
            Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
        };
        let file = fs::File::open(path).ok()?;
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        unsafe {
            GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info).ok()?;
        }
        Some(format!(
            "{}:{}:{}",
            info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow
        ))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = fs::metadata(path).ok()?;
        Some(format!("{}:{}", meta.dev(), meta.ino()))
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = path;
        None
    }
}
pub(crate) fn images_under(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    if !root.is_dir() {
        return Ok(result);
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            // 不跟随目录链接，避免扫描越过游戏目录或资源库边界。
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                pending.push(entry.path());
            }
            if kind.is_file() && image_name(&entry.path()).is_some() {
                result.push(entry.path());
            }
        }
    }
    result.sort();
    Ok(result)
}
pub(crate) fn image_name(path: &Path) -> Option<(String, bool)> {
    let name = path.file_name()?.to_str()?;
    let disabled = name.to_ascii_lowercase().ends_with(".png.disabled");
    let png = if disabled {
        &name[..name.len() - 9]
    } else {
        name
    };
    png.to_ascii_lowercase()
        .ends_with(".png")
        .then(|| (png.to_string(), disabled))
}
