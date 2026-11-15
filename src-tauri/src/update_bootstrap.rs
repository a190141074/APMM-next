//! 旧版首次升级的助手交接。旧助手跳过自身文件，新程序在启动工作区前补齐经过校验的新助手。
use crate::portable_update as portable;
use std::{fs, path::Path};

/// 仅当当前发行清单明确声明交接文件时执行，保留旧用户数据和不属于发行物的文件。
pub(crate) fn finish(directory: &Path, version: &str) -> Result<bool, String> {
    let manifest_path = directory.join("release-manifest.json");
    if !manifest_path.is_file() { return Ok(false); }
    let manifest: portable::ReleaseManifest = serde_json::from_slice(&fs::read(&manifest_path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if !manifest.files.iter().any(|f| f.path == "astral-updater-next.exe") { return Ok(false); }
    if manifest.schema != 1 || manifest.version != version || manifest.data_schema != 1 {
        return Err("旧版更新交接的发行版本或数据格式不匹配".into());
    }
    let helper = manifest.files.iter().find(|f| f.path == "astral-updater.exe").ok_or("交接清单缺少更新助手")?;
    let next = manifest.files.iter().find(|f| f.path == "astral-updater-next.exe").unwrap();
    if helper.sha256 != next.sha256 || helper.bytes != next.bytes {
        return Err("交接助手与发行助手不一致".into());
    }
    for file in manifest.files.iter().filter(|f| f.path != "astral-updater.exe") {
        portable::verify_file(directory, file)?;
    }
    let marker = directory.join("data/updates/bootstrap-version.json");
    let notified = fs::read(&marker).ok().and_then(|v| serde_json::from_slice::<serde_json::Value>(&v).ok())
        .is_some_and(|v| v["version"] == version);
    if portable::verify_file(directory, helper).is_err() {
        let pending = directory.join("astral-updater.apmm-bootstrap-pending");
        fs::copy(directory.join(&next.path), &pending).map_err(|e| e.to_string())?;
        portable::retry_io(|| fs::rename(&pending, directory.join(&helper.path))).map_err(|e| format!("旧版助手尚未释放，交接未完成：{e}"))?;
        portable::verify_file(directory, helper)?;
    }
    if !notified {
        portable::write_json(&directory.join("data/updates/last-result.json"),
            &serde_json::json!({"status":"complete","version":version,"message":format!("已更新到 {version}，新版更新助手已就绪")}))?;
        portable::write_json(&marker, &serde_json::json!({"version":version}))?;
    }
    Ok(!notified)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_helper_is_replaced_only_by_verified_handover() {
        let root = crate::staging::create("bootstrap-test").unwrap();
        fs::write(root.join("astral-updater.exe"), b"old").unwrap();
        fs::write(root.join("astral-updater-next.exe"), b"new").unwrap();
        fs::create_dir_all(root.join("data")).unwrap(); fs::write(root.join("data/user.txt"), b"user").unwrap();
        let mut helper = portable::FileRecord { path:"astral-updater.exe".into(), bytes:3, sha256:portable::hash(&root.join("astral-updater-next.exe")).unwrap() };
        let mut next = helper.clone(); next.path = "astral-updater-next.exe".into();
        let mut manifest = portable::ReleaseManifest { schema:1,version:"0.3.40".into(),data_schema:1,files:vec![helper.clone(),next] };
        portable::write_json(&root.join("release-manifest.json"), &manifest).unwrap();
        assert!(finish(&root,"0.3.40").unwrap()); assert!(!finish(&root,"0.3.40").unwrap());
        assert_eq!(fs::read(root.join("astral-updater.exe")).unwrap(), b"new");
        assert_eq!(fs::read(root.join("data/user.txt")).unwrap(), b"user");
        helper.sha256 = "bad".into(); manifest.files[0] = helper;
        portable::write_json(&root.join("release-manifest.json"), &manifest).unwrap();
        assert!(finish(&root,"0.3.40").is_err());
        crate::staging::remove(root).unwrap();
    }
}
