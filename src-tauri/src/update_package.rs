//! 更新包预检和安全展开；保留发行清单中的相对路径，并兼容旧版使用的平铺包。
use crate::portable_update::{self as portable, ReleaseManifest};
use std::{collections::HashSet, fs, path::Path};

/// 在退出之前展开并校验全部程序文件，不允许包中的目录改变发行文件的身份。
pub(crate) fn unpack(archive: &Path, output: &Path, version: &str) -> Result<ReleaseManifest, String> {
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if zip.len() > 64 {
        return Err("软件更新包包含过多文件".into());
    }
    let mut prefix = None;
    for index in 0..zip.len() {
        let entry = zip.by_index(index).map_err(|e| e.to_string())?;
        if entry.name().ends_with("release-manifest.json") {
            let rel = entry.enclosed_name().ok_or("更新包包含越界路径")?;
            if rel.file_name().and_then(|v| v.to_str()) == Some("release-manifest.json") {
                if prefix.is_some() { return Err("更新包包含多个发行清单".into()); }
                prefix = Some(rel.parent().unwrap_or(Path::new("")).to_path_buf());
            }
        }
    }
    let prefix = prefix.ok_or("更新包缺少发行清单")?;
    let mut seen = HashSet::new();
    let mut bytes = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
        let rel = entry.enclosed_name().ok_or("更新包包含越界路径")?;
        if entry.is_dir() { continue; }
        let relative = rel.strip_prefix(&prefix).map_err(|_| "更新文件不在同一发行目录内")?
            .to_str().ok_or("更新路径不是有效文本")?.replace('\\', "/");
        if !portable::ALLOWED.contains(&relative.as_str()) || !seen.insert(relative.to_lowercase())
            || entry.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) {
            return Err("更新包包含非发行文件、链接或重复文件".into());
        }
        bytes = bytes.checked_add(entry.size()).ok_or("更新包大小溢出")?;
        if bytes > 512 * 1024 * 1024 { return Err("更新包解压大小超限".into()); }
        let target = output.join(&relative);
        fs::create_dir_all(target.parent().ok_or("更新文件缺少目录")?).map_err(|e| e.to_string())?;
        let mut file = fs::File::create(target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    let manifest = portable::manifest(output, version)?;
    let expected: HashSet<_> = manifest.files.iter().map(|f| f.path.to_lowercase())
        .chain(["release-manifest.json".into()]).collect();
    if seen != expected { return Err("更新包文件与发行清单不一致".into()); }
    Ok(manifest)
}

/// 禁止回退版本或跨越当前尚不能读取的数据格式；旧发行清单默认数据格式为 1。
pub(crate) fn precheck(manifest: &ReleaseManifest, current: &str, directory: &Path) -> Result<(), String> {
    let parse = |v: &str| semver::Version::parse(v.trim_start_matches(['v', 'V'])).map_err(|e| e.to_string());
    if parse(&manifest.version)? < parse(current)? { return Err("安装包版本低于当前版本".into()); }
    if manifest.data_schema != 1 { return Err("更新包数据格式不兼容，请按发布说明升级".into()); }
    let probe = directory.join(format!(".apmm-write-test-{}", uuid::Uuid::new_v4()));
    fs::OpenOptions::new().create_new(true).write(true).open(&probe)
        .map_err(|e| format!("程序目录不可写，更新尚未开始：{e}"))?;
    fs::remove_file(probe).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn package(root: &Path, prefix: &str, license: &str) -> std::path::PathBuf {
        let records: Vec<_> = [("astral-mod-manager.exe", b"exe".as_slice()), (license, b"license".as_slice())]
            .into_iter().map(|(path, bytes)| {
                let file = root.join(path); fs::create_dir_all(file.parent().unwrap()).unwrap(); fs::write(&file, bytes).unwrap();
                portable::FileRecord { path: path.into(), bytes: bytes.len() as u64, sha256: portable::hash(&file).unwrap() }
            }).collect();
        let manifest = ReleaseManifest { schema: 1, version: "0.3.40".into(), data_schema: 1, files: records };
        let archive = root.join("test.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        for f in &manifest.files { zip.start_file(format!("{prefix}{}", f.path), options).unwrap(); zip.write_all(&fs::read(root.join(&f.path)).unwrap()).unwrap(); }
        zip.start_file(format!("{prefix}release-manifest.json"), options).unwrap(); zip.write_all(&serde_json::to_vec(&manifest).unwrap()).unwrap(); zip.finish().unwrap();
        archive
    }
    #[test]
    fn nested_and_legacy_flat_packages_keep_manifest_paths() {
        for (prefix, license) in [
            ("APMM-0.3.40/", "licenses/LICENSE.txt"),
            ("APMM/", "licenses/LICENSE.txt"),
            ("", "LICENSE.txt"),
        ] {
            let root = crate::staging::create("unpack-test").unwrap();
            let archive = package(&root, prefix, license); let output = root.join("output");
            let manifest = unpack(&archive, &output, "0.3.40").unwrap();
            assert_eq!(fs::read(output.join(license)).unwrap(), b"license");
            precheck(&manifest, "0.3.36", &root).unwrap();
            assert!(precheck(&manifest, "0.3.41", &root).is_err());
            let mut future = manifest; future.data_schema = 2; assert!(precheck(&future, "0.3.36", &root).is_err());
            crate::staging::remove(root).unwrap();
        }
    }
    #[test]
    fn undeclared_files_are_rejected_before_replacement() {
        let root = crate::staging::create("extra-file-test").unwrap();
        let archive = package(&root, "", "LICENSE.txt");
        let mut zip = zip::ZipWriter::new_append(fs::OpenOptions::new().read(true).write(true).open(&archive).unwrap()).unwrap();
        zip.start_file("THIRD_PARTY_NOTICES.md", zip::write::SimpleFileOptions::default()).unwrap(); zip.write_all(b"extra").unwrap(); zip.finish().unwrap();
        assert!(unpack(&archive, &root.join("output"), "0.3.40").is_err());
        crate::staging::remove(root).unwrap();
    }
}
