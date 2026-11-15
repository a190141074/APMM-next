//! 主程序与独立更新助手共享发行清单和恢复日志，更新范围仅限发行程序文件。
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const ALLOWED: &[&str] = &[
    "astral-mod-manager.exe",
    "astral-mod-manager.exe.manifest",
    "WebView2Loader.dll",
    "api-ms-win-core-synch-l1-2-0.dll",
    "astral-updater.exe",
    "astral-updater-next.exe",
    "release-manifest.json",
    "LICENSE.txt",
    "THIRD_PARTY_NOTICES.md",
    "licenses/LICENSE.txt",
    "licenses/THIRD_PARTY_NOTICES.md",
    "installation.json",
    "licenses/NOTICE.md",
    "NOTICE.md",
];
#[derive(Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub schema: u32,
    pub version: String,
    #[serde(default = "default_data_schema", rename = "dataSchema")]
    pub data_schema: u32,
    pub files: Vec<FileRecord>,
}
fn default_data_schema() -> u32 {
    1
}

/// Windows 文件锁可能在进程退出后短暂保留；仅重试共享冲突，不掩盖路径和权限错误。
pub fn retry_io<T>(mut operation: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        match operation() {
            Err(e)
                if matches!(e.raw_os_error(), Some(32 | 33))
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(250))
            }
            result => return result,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Job {
    pub schema: u32,
    pub id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub backup: PathBuf,
    pub pid: u32,
    pub version: String,
    pub files: Vec<FileRecord>,
    pub existed: Vec<String>,
    pub status: String,
    pub message: String,
    #[serde(default = "portable_kind")]
    pub kind: String,
    #[serde(default)]
    pub data_directory: Option<PathBuf>,
    #[serde(default)]
    pub installer: Option<FileRecord>,
}
fn portable_kind() -> String {
    "portable".into()
}
impl Job {
    pub fn data(&self) -> PathBuf {
        self.data_directory
            .clone()
            .unwrap_or_else(|| self.destination.join("data"))
    }
    pub fn result_path(&self) -> PathBuf {
        self.data().join("updates/last-result.json")
    }
}
pub fn hash(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buffer[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
pub fn verify_file(root: &Path, f: &FileRecord) -> Result<(), String> {
    if !ALLOWED.contains(&f.path.as_str()) {
        return Err("发行清单包含非程序文件".into());
    }
    check_path(root, &f.path)?;
    let path = root.join(&f.path);
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() != f.bytes || hash(&path)? != f.sha256
    {
        return Err(format!("发行文件校验失败：{}", f.path));
    }
    Ok(())
}
/// 只接受明确列出的发行路径，并拒绝父目录链接，避免许可子目录越出程序目录。
fn check_path(root: &Path, relative: &str) -> Result<(), String> {
    if !ALLOWED.contains(&relative) {
        return Err("发行路径不在白名单中".into());
    }
    let mut current = root.to_path_buf();
    for part in relative.split('/') {
        current.push(part);
        if let Ok(meta) = fs::symlink_metadata(&current) {
            let mut linked = meta.file_type().is_symlink();
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                linked |= meta.file_attributes() & 0x400 != 0;
            }
            if linked {
                return Err("发行文件及父目录不能是链接".into());
            }
        }
    }
    Ok(())
}
pub fn manifest(root: &Path, version: &str) -> Result<ReleaseManifest, String> {
    let m: ReleaseManifest = serde_json::from_slice(
        &fs::read(root.join("release-manifest.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if m.schema != 1
        || m.version != version
        || !m.files.iter().any(|f| f.path == "astral-mod-manager.exe")
    {
        return Err("发行清单版本或程序入口不匹配".into());
    }
    let mut seen = std::collections::HashSet::new();
    for f in &m.files {
        if !seen.insert(f.path.to_lowercase()) {
            return Err("发行清单包含重复文件".into());
        }
        verify_file(root, f)?;
    }
    Ok(m)
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let pending = path.with_extension("json.pending");
    let mut file = fs::File::create(&pending).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(pending, path).map_err(|e| e.to_string())
}
pub fn load_job(path: &Path) -> Result<Job, String> {
    let job: Job = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let stage = crate::staging::root()
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let source = job.source.canonicalize().map_err(|e| e.to_string())?;
    let dest = job.destination.canonicalize().map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .ok_or("更新日志无父目录")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !parent.starts_with(&stage)
        || parent == stage
        || job.backup != path.parent().unwrap().join("backup")
        || !source.starts_with(&parent)
    {
        return Err("更新计划不在专用暂存目录内".into());
    }
    if job.schema != 1
        || !matches!(job.kind.as_str(), "portable" | "msi")
        || !source.starts_with(&stage)
        || dest.starts_with(&source)
        || source.starts_with(&dest)
        || !job
            .backup
            .starts_with(path.parent().ok_or("更新日志无父目录")?)
    {
        return Err("更新路径边界无效".into());
    }
    if uuid::Uuid::parse_str(&job.id).is_err()
        || semver::Version::parse(job.version.trim_start_matches('v')).is_err()
    {
        return Err("更新任务身份或版本无效".into());
    }
    let expected_data = if job.kind == "msi" {
        crate::installation::installed_data()
    } else {
        job.destination.join("data")
    };
    if job.data().canonicalize().map_err(|e| e.to_string())?
        != expected_data.canonicalize().map_err(|e| e.to_string())?
    {
        return Err("更新数据目录与安装身份不匹配".into());
    }
    if job.kind == "msi"
        && (!job.files.is_empty()
            || !job
                .installer
                .as_ref()
                .is_some_and(|f| f.path == "installer.msi"))
    {
        return Err("MSI 更新计划无效".into());
    }
    let mut seen = std::collections::HashSet::new();
    for f in &job.files {
        if !seen.insert(f.path.to_lowercase()) {
            return Err("更新计划包含重复文件".into());
        }
        if !ALLOWED.contains(&f.path.as_str()) {
            return Err("更新计划越出发行文件范围".into());
        }
        check_path(&job.destination, &f.path)?;
    }
    if job.existed.iter().any(|p| !ALLOWED.contains(&p.as_str())) {
        return Err("备份记录越出发行范围".into());
    }
    Ok(job)
}
pub fn backup(path: &Path, job: &mut Job) -> Result<(), String> {
    if job.kind != "portable" {
        return Err("MSI 不能使用便携文件备份".into());
    }
    if job.status != "new" {
        return Err("拒绝覆盖既有更新备份，请使用恢复模式".into());
    }
    fs::create_dir_all(&job.backup).map_err(|e| e.to_string())?;
    for f in &job.files {
        verify_file(&job.source, f)?;
        check_path(&job.destination, &f.path)?;
        let current = job.destination.join(&f.path);
        if current.is_symlink() {
            return Err("发行文件不能是链接".into());
        }
        if current.is_file() {
            let saved = job.backup.join(&f.path);
            fs::create_dir_all(saved.parent().ok_or("备份文件缺少目录")?)
                .map_err(|e| e.to_string())?;
            fs::copy(&current, saved).map_err(|e| e.to_string())?;
            job.existed.push(f.path.clone());
        }
    }
    job.status = "prepared".into();
    write_json(path, job)
}
pub fn apply(path: &Path, job: &mut Job) -> Result<(), String> {
    if job.kind != "portable" {
        return Err("MSI 不能使用便携文件覆盖".into());
    }
    job.status = "applying".into();
    write_json(path, job)?;
    for f in &job.files {
        verify_file(&job.source, f)?;
        check_path(&job.destination, &f.path)?;
        let target = job.destination.join(&f.path);
        fs::create_dir_all(target.parent().ok_or("更新文件缺少目录")?)
            .map_err(|e| e.to_string())?;
        let pending = target.with_extension("apmm-update-pending");
        fs::copy(job.source.join(&f.path), &pending).map_err(|e| e.to_string())?;
        retry_io(|| fs::rename(&pending, &target)).map_err(|e| e.to_string())?;
        verify_file(&job.destination, f)?;
    }
    job.status = "awaiting-startup".into();
    write_json(path, job)
}
pub fn rollback(path: &Path, job: &mut Job, error: &str) -> Result<(), String> {
    if job.kind != "portable" {
        return Err("MSI 恢复由 Windows Installer 管理".into());
    }
    let mut errors = Vec::new();
    for f in &job.files {
        check_path(&job.destination, &f.path)?;
        let target = job.destination.join(&f.path);
        let result = if job.existed.contains(&f.path) {
            retry_io(|| fs::copy(job.backup.join(&f.path), &target)).map(|_| ())
        } else if target.exists() {
            retry_io(|| fs::remove_file(&target))
        } else {
            Ok(())
        };
        if let Err(e) = result {
            errors.push(format!("{}: {e}", f.path));
        }
    }
    job.message = error.into();
    job.status = if errors.is_empty() {
        "rolled-back"
    } else {
        "recovery-required"
    }
    .into();
    write_json(path, job)?;
    write_json(&job.result_path(), job)?;
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn licenses_and_legacy_paths_use_an_exact_allowlist() {
        let root = crate::staging::create("license-test").unwrap();
        for name in ["licenses/THIRD_PARTY_NOTICES.md", "THIRD_PARTY_NOTICES.md"] {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"notices").unwrap();
            let f = FileRecord {
                path: name.into(),
                bytes: 7,
                sha256: hash(&path).unwrap(),
            };
            verify_file(&root, &f).unwrap();
        }
        for path in [
            "licenses/other.md",
            "licenses/../data/library.json",
            "licenses\\THIRD_PARTY_NOTICES.md",
            "data/library.json",
        ] {
            assert!(check_path(&root, path).is_err());
        }
        crate::staging::remove(root).unwrap();
    }
    #[test]
    fn interrupted_batch_restores_old_version_and_user_data() {
        let root = crate::staging::create("update-test").unwrap();
        let source = root.join("source");
        let destination = root.join("app");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(destination.join("data")).unwrap();
        fs::write(destination.join("data/user.json"), b"user-data").unwrap();
        fs::write(destination.join("astral-mod-manager.exe"), b"old").unwrap();
        fs::write(source.join("astral-mod-manager.exe"), b"new").unwrap();
        let record = FileRecord {
            path: "astral-mod-manager.exe".into(),
            bytes: 3,
            sha256: hash(&source.join("astral-mod-manager.exe")).unwrap(),
        };
        fs::create_dir_all(source.join("licenses")).unwrap();
        fs::create_dir_all(destination.join("licenses")).unwrap();
        fs::write(source.join("licenses/THIRD_PARTY_NOTICES.md"), b"new").unwrap();
        fs::write(destination.join("licenses/THIRD_PARTY_NOTICES.md"), b"old").unwrap();
        let license = FileRecord {
            path: "licenses/THIRD_PARTY_NOTICES.md".into(),
            bytes: 3,
            sha256: hash(&source.join("licenses/THIRD_PARTY_NOTICES.md")).unwrap(),
        };
        let mut job = Job {
            schema: 1,
            id: "test".into(),
            source,
            destination: destination.clone(),
            backup: root.join("backup"),
            pid: 0,
            version: "1.0.0".into(),
            files: vec![record, license],
            existed: vec![],
            status: "new".into(),
            message: String::new(),
            kind: "portable".into(),
            data_directory: None,
            installer: None,
        };
        let path = root.join("job.json");
        backup(&path, &mut job).unwrap();
        apply(&path, &mut job).unwrap();
        rollback(&path, &mut job, "模拟重启失败").unwrap();
        assert_eq!(
            fs::read(destination.join("astral-mod-manager.exe")).unwrap(),
            b"old"
        );
        assert_eq!(
            fs::read(destination.join("data/user.json")).unwrap(),
            b"user-data"
        );
        assert_eq!(
            fs::read(destination.join("licenses/THIRD_PARTY_NOTICES.md")).unwrap(),
            b"old"
        );
        crate::staging::remove(root).unwrap();
    }
}
