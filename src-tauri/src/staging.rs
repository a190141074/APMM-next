//! 专用暂存路径与白名单清理边界，禁止把用户输入作为递归删除根。
use std::{
    fs,
    path::{Path, PathBuf},
};
pub fn work_root() -> PathBuf {
    if let Some(path) = std::env::var_os("APMM_WORK_DIR") {
        return PathBuf::from(path);
    }
    if cfg!(windows) && Path::new("B:/").is_dir() {
        return PathBuf::from("B:/APMM-next-work");
    }
    std::env::temp_dir().join("apmm-next-work")
}
pub fn root() -> PathBuf {
    work_root().join("staging")
}
pub fn create(label: &str) -> Result<PathBuf, String> {
    if !label
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return Err("无效暂存用途".into());
    }
    let dir = root().join(format!("{}-{}", label, uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
pub fn remove(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }
    let parent = root().canonicalize().map_err(|e| e.to_string())?;
    let target = path.canonicalize().map_err(|e| e.to_string())?;
    if !target.starts_with(&parent) || target == parent || !target.is_dir() || path.is_symlink() {
        return Err("拒绝清理暂存根以外的路径".into());
    }
    let mut pending = vec![target.clone()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let meta = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            let mut linked = meta.file_type().is_symlink();
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                linked |= meta.file_attributes() & 0x400 != 0;
            }
            if linked {
                return Err("暂存目录包含链接，已保留供检查".into());
            }
            if meta.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    fs::remove_dir_all(target).map_err(|e| e.to_string())
}
/// 跨盘载荷先复制到目标所在卷，核对内容后原子提交，原载荷保留用于重试。
pub fn copy_new(source: &Path, target: &Path) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    fn hash(path: &Path) -> Result<Vec<u8>, String> {
        let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut digest = Sha256::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = file.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            digest.update(&buf[..n]);
        }
        Ok(digest.finalize().to_vec())
    }
    let parent = target.parent().ok_or("目标文件缺少父目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".apmm-{}.pending", uuid::Uuid::new_v4()));
    let result = (|| {
        if target.exists() {
            return Err("目标文件已经存在，拒绝覆盖".into());
        }
        fs::copy(source, &temp).map_err(|e| e.to_string())?;
        fs::OpenOptions::new()
            .write(true)
            .open(&temp)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        if hash(source)? != hash(&temp)? {
            return Err("跨盘复制校验失败".into());
        }
        // MoveFileW 原子移动且拒绝覆盖，也支持便携盘使用的 exFAT 文件系统。
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let from = temp
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>();
            let to = target
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>();
            unsafe {
                windows::Win32::Storage::FileSystem::MoveFileW(
                    windows_core::PCWSTR(from.as_ptr()),
                    windows_core::PCWSTR(to.as_ptr()),
                )
            }
            .map_err(|e| e.to_string())?;
        }
        #[cfg(not(windows))]
        fs::hard_link(&temp, target).map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = fs::remove_file(&temp);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commits_new_file_without_overwriting_existing_destination() {
        let root = create("copy-test").unwrap();
        let source = root.join("source");
        let target = root.join("target");
        fs::write(&source, b"verified payload").unwrap();
        copy_new(&source, &target).unwrap();
        assert_eq!(fs::read(&source).unwrap(), fs::read(&target).unwrap());
        fs::write(&source, b"other").unwrap();
        assert!(copy_new(&source, &target).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"verified payload");
        remove(root).unwrap();
    }
}
