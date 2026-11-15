//! Vault path and layout migration boundary.

use std::path::{Component, Path, PathBuf};

pub(crate) fn safe_relative_path(raw: &str) -> Option<PathBuf> {
    let path = Path::new(raw);
    if raw.trim().is_empty() || path.is_absolute() {
        return None;
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => out.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}
