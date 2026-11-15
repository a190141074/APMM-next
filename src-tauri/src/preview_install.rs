//! 预览包只在独立目录解包，覆盖既有预览前备份；失败保留旧图和未完成的恢复材料。
use crate::*;
pub fn install(
    bundle: &[u8],
    remote: &RemotePreviewManifest,
    cancel: Option<&AtomicBool>,
    legacy: bool,
) -> Result<(), String> {
    if !remote.sha256.is_empty() && sha256_bytes(bundle) != remote.sha256.to_lowercase() {
        return Err("预览包校验失败".into());
    }
    let key = if remote.bundle_key.is_empty() {
        "all"
    } else {
        &remote.bundle_key
    };
    if !["all", "heroes", "cards", "monsters"].contains(&key) {
        return Err("未知预览分类".into());
    }
    let expected = if key == "all" {
        expected_preview_files()
    } else {
        expected_preview_files_for_bundle(key)
    };
    let stage = crate::staging::create("preview-install")?;
    let backup = stage.join("backup");
    ensure_dir(&backup).map_err(|e| e.to_string())?;
    let cache = preview_cache_dir();
    ensure_dir(&cache).map_err(|e| e.to_string())?;
    let mut changed: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let result = (|| {
        let mut archive = zip::ZipArchive::new(Cursor::new(bundle)).map_err(|e| e.to_string())?;
        if archive.len() > 20001 {
            return Err("预览包文件数量超限".into());
        }
        let mut names = HashSet::new();
        let mut files = Vec::new();
        let mut total = 0u64;
        for i in 0..archive.len() {
            if cancel.is_some_and(|f| f.load(Ordering::Acquire)) {
                return Err("预览下载已取消".into());
            }
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            if entry.is_dir() {
                continue;
            }
            let enclosed = entry.enclosed_name().ok_or("预览包路径越界")?;
            let relative = enclosed
                .strip_prefix("previews/")
                .map_err(|_| "预览包目录结构无效")?;
            let name = relative.to_string_lossy().to_string();
            if relative.components().count() != 1
                || !matches!(
                    relative.extension().and_then(|s| s.to_str()),
                    Some("webp" | "png")
                )
                || !names.insert(name.to_lowercase())
                || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            {
                return Err("预览包含无效路径、重复文件或链接".into());
            }
            if !legacy && !expected.contains(&name) {
                return Err("预览包包含分类以外的资源".into());
            }
            total = total.checked_add(entry.size()).ok_or("预览包大小溢出")?;
            if total > 512 * 1024 * 1024 || entry.size() > 16 * 1024 * 1024 {
                return Err("预览包解压大小超限".into());
            }
            let output = stage.join(&name);
            std::io::copy(
                &mut entry,
                &mut fs::File::create(&output).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            files.push(name);
        }
        if (!legacy && files.len() != remote.files)
            || (legacy
                && (files.len() < remote.files || expected.iter().any(|n| !files.contains(n))))
        {
            return Err("预览包内容不完整".into());
        }
        let state_name = if legacy {
            "state.json".into()
        } else {
            format!("state-{key}.json")
        };
        fs::write(
            stage.join(&state_name),
            serde_json::to_vec(&PreviewCacheState {
                version: remote.version.clone(),
            })
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        files.sort();
        files.push(state_name);
        for name in files {
            if cancel.is_some_and(|f| f.load(Ordering::Acquire)) {
                return Err("预览安装已取消".into());
            }
            let target = cache.join(&name);
            let old = if target.is_file() {
                let p = backup.join(&name);
                fs::copy(&target, &p).map_err(|e| e.to_string())?;
                Some(p)
            } else {
                None
            };
            changed.push((target.clone(), old));
            let pending = cache.join(format!(".install-{name}"));
            fs::copy(stage.join(name), &pending).map_err(|e| e.to_string())?;
            fs::rename(pending, target).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(error) = &result {
        let mut failures = Vec::new();
        for (target, old) in changed.into_iter().rev() {
            let restored = if let Some(p) = old {
                fs::copy(p, &target).map(|_| ())
            } else if target.exists() {
                fs::remove_file(&target)
            } else {
                Ok(())
            };
            if let Err(e) = restored {
                failures.push(e.to_string());
            }
        }
        if !failures.is_empty() {
            return Err(format!(
                "{error}；预览回滚失败：{}；备份位于 {}",
                failures.join("；"),
                stage.display()
            ));
        }
    }
    crate::staging::remove(stage)?;
    result
}
