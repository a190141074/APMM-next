//! Asset scanning and cache boundary.

use std::path::{Path, PathBuf};

pub(crate) fn relative_cache_key(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn normalize_scan_root(raw: &str) -> Option<PathBuf> {
    let path = PathBuf::from(raw.trim());
    (!path.as_os_str().is_empty()).then_some(path)
}
