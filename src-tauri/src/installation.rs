//! 安装身份来自当前程序旁的发行标记。另一份 MSI 安装不会改变便携副本的数据目录。
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
pub const UPGRADE_CODE: &str = "DF7031A2-E41A-465C-BCBA-4DF3FCB73894";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub schema: u32,
    pub kind: String,
}
pub fn kind_at(directory: &Path) -> &'static str {
    let identity = std::fs::read(directory.join("installation.json"))
        .ok()
        .and_then(|v| serde_json::from_slice::<Identity>(&v).ok());
    if identity.is_some_and(|v| v.schema == 1 && v.kind == "msi") {
        "msi"
    } else {
        "portable"
    }
}
pub fn kind() -> &'static str {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(kind_at))
        .unwrap_or("portable")
}
pub fn installed_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("APMM")
}
pub fn data_for(directory: &Path) -> PathBuf {
    if kind_at(directory) == "msi" {
        installed_data()
    } else {
        directory.join("data")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_copy_keeps_its_own_installation_identity() {
        let root = crate::staging::create("installation-test").unwrap();
        let installed = root.join("installed");
        let portable = root.join("portable");
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::create_dir_all(&portable).unwrap();
        std::fs::write(
            installed.join("installation.json"),
            br#"{"schema":1,"kind":"msi"}"#,
        )
        .unwrap();
        assert_eq!(kind_at(&installed), "msi");
        assert_eq!(kind_at(&portable), "portable");
        assert_eq!(data_for(&installed), installed_data());
        assert_eq!(data_for(&portable), portable.join("data"));
        crate::staging::remove(root).unwrap();
    }
}
