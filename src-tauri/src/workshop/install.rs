//! 工坊库导入与应用事务，保留文件归属和失败回滚边界。
use super::*;
pub(super) fn all_rels(lib: &Library) -> HashSet<String> {
    lib.heroes
        .values()
        .chain(lib.monsters.values())
        .flat_map(|hero| hero.sections.values())
        .flat_map(|section| section.resources.values())
        .flat_map(|resource| resource.files.iter().map(|f| f.rel_path.clone()))
        .collect()
}

pub(super) fn resource_mut<'a>(
    lib: &'a mut Library,
    owned: &OwnedFile,
) -> Option<&'a mut ResourceEntry> {
    let cat = category_of(&owned.section_key);
    lib_map_mut(lib, &cat)
        .get_mut(&owned.hero_id)?
        .sections
        .get_mut(&owned.section_key)?
        .resources
        .get_mut(&owned.canonical)
}

pub(super) fn remove_owned(lib: &mut Library, owned: &OwnedFile, allow_active: bool) -> bool {
    let Some(res) = resource_mut(lib, owned) else {
        return false;
    };
    if res.disabled || (!allow_active && res.active_hash.as_deref() == Some(owned.hash.as_str())) {
        return false;
    }
    let Some(index) = res.files.iter().position(|f| {
        f.hash == owned.hash && f.source_name == owned.source_name && f.rel_path == owned.rel_path
    }) else {
        return false;
    };
    let path = vault_dir().join(&owned.rel_path);
    if !path.exists() || sha256_file(&path).ok().as_deref() != Some(&owned.hash) {
        return false;
    }
    res.files.remove(index);
    if res.active_hash.as_deref() == Some(&owned.hash) {
        res.active_hash = None;
    }
    true
}

pub(super) fn apply_transaction(
    work: &Work,
    paths: &[PathBuf],
    old: &Library,
    stage: &Path,
    apply_files: &[String],
) -> Result<(Library, usize), String> {
    let mut lib = old.clone();
    let previous = old.workshop.get(&work.id).cloned();
    let mut created = Vec::new();
    let mut owned_files = Vec::new();
    let mut touched = HashSet::new();
    for path in paths {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let meta = work
            .files
            .iter()
            .find(|f| f.name == name)
            .ok_or("资源清单不匹配")?;
        let parsed = meta.parsed()?;
        touched.insert(parsed.canonical.clone());
    }
    if let Some(prev) = &previous {
        for f in &prev.files {
            touched.insert(f.canonical.clone());
        }
    }
    let modimg = PathBuf::from(lib.settings.modimg_path.as_str());
    let backup = if !apply_files.is_empty() && !lib.settings.modimg_path.is_empty() {
        backup_modimg_for_queue(&modimg, &touched, stage)?
    } else {
        Vec::new()
    };
    let result = (|| -> Result<usize, String> {
        for path in paths {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let meta = work
                .files
                .iter()
                .find(|f| f.name == name)
                .ok_or("资源清单不匹配")?;
            let parsed = meta.parsed()?;
            let expected_hash = sha256_file(path)?;
            let (target, _) = place_in_vault(
                &vault_dir(),
                &category_of(&parsed.section_key),
                &parsed.hero_id,
                &parsed.section_key,
                meta.source(),
                &expected_hash,
                ".png",
            )?;
            let target_existed = target.exists();
            if target_existed && sha256_file(&target)? != expected_hash {
                return Err("本地同名候选已修改，请先保留或重新分类后再获取".into());
            }
            let (imported, _) = import_single(
                &mut lib,
                &vault_dir(),
                &path.to_string_lossy(),
                &parsed.hero_id,
                &parsed.section_key,
                &parsed.canonical,
                Some(meta.source().strip_suffix(".png").unwrap_or_else(|| {
                    meta.source()
                        .rsplit_once('.')
                        .map(|v| v.0)
                        .unwrap_or(meta.source())
                })),
            )
            .map_err(|r| r.reason.unwrap_or_else(|| "导入失败".into()))?;
            if imported.duplicate {
                let resource = lib_map(&lib, &category_of(&parsed.section_key))
                    .get(&parsed.hero_id)
                    .and_then(|hero| hero.sections.get(&parsed.section_key))
                    .and_then(|section| section.resources.get(&parsed.canonical))
                    .ok_or("重复候选已丢失")?;
                let candidates: Vec<_> = resource
                    .files
                    .iter()
                    .filter(|file| Some(&file.hash) == imported.hash.as_ref())
                    .collect();
                if candidates.is_empty()
                    || candidates.iter().any(|file| {
                        sha256_file(&vault_path(&vault_dir(), file)).ok().as_deref()
                            != Some(file.hash.as_str())
                    })
                {
                    return Err("已存在的候选发生修改，请刷新并保留本地文件后重试".into());
                }
            }
            if !imported.duplicate {
                let rel = imported.rel_path.clone().ok_or("缺少资源路径")?;
                if !target_existed {
                    created.push(rel.clone());
                }
                owned_files.push(OwnedFile {
                    hero_id: parsed.hero_id.clone(),
                    section_key: parsed.section_key.clone(),
                    canonical: parsed.canonical.clone(),
                    hash: imported.hash.clone().unwrap_or_default(),
                    source_name: meta.source().into(),
                    rel_path: rel,
                });
            }
            if imported.duplicate && !target_existed {
                let _ = fs::remove_file(&target);
            }
            if apply_files.contains(&name) {
                activate_resource(
                    &mut lib,
                    &parsed.hero_id,
                    &parsed.section_key,
                    &parsed.canonical,
                    imported.hash.as_deref().unwrap_or(""),
                )?;
            }
        }
        let mut retained = Vec::new();
        let incoming: HashSet<(String, String)> = paths
            .iter()
            .filter_map(|path| {
                let name = path.file_name()?.to_string_lossy().to_string();
                let meta = work.files.iter().find(|f| f.name == name)?;
                let parsed = meta.parsed().ok()?;
                let hash = meta.sha256.clone();
                Some((parsed.canonical, hash.to_ascii_lowercase()))
            })
            .collect();
        if let Some(prev) = previous {
            for file in prev.files {
                if incoming.contains(&(file.canonical.clone(), file.hash.clone())) {
                    retained.push(file);
                    continue;
                }
                if !remove_owned(&mut lib, &file, false) {
                    retained.push(file);
                }
            }
        }
        let count = owned_files.len();
        owned_files.extend(retained);
        lib.workshop.insert(
            work.id.clone(),
            InstalledWork {
                version: work.version.clone(),
                archive_hash: work.sha256.clone(),
                files: owned_files,
            },
        );
        save_library(&lib)?;
        Ok(count)
    })();
    if result.is_err() {
        if !apply_files.is_empty() && !lib.settings.modimg_path.is_empty() {
            restore_modimg_queue_backup(&modimg, &touched, &backup)
                .map_err(|e| format!("回滚失败：{e}；备份保留于 {}", stage.display()))?;
        }
        for rel in created {
            let _ = fs::remove_file(vault_dir().join(rel));
        }
    }
    result.map(|count| (lib, count))
}

/// 只有通过快照复核的请求可以写库；进入提交区间后不再响应取消。
#[tauri::command]
pub(crate) async fn workshop_install(
    request: plan::InstallRequest,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || {
        let base = base_url().ok_or("工坊未配置")?;
        let work = catalog(&base)?
            .into_iter()
            .find(|w| w.id == request.id)
            .ok_or("作品已下架或不存在")?;
        if plan::fingerprint(&work)? != request.work_fingerprint {
            return Err("作品已更新，请重新检查".into());
        }
        let archive = cache_archive(&base, &work, &app)?;
        let stage = crate::staging::create("workshop-install")?;
        let result = (|| {
            let paths = unpack_checked(&work, &archive, &stage)?;
            crate::tasks::checkpoint(&app)?;
            let current = catalog(&base)?
                .into_iter()
                .find(|w| w.id == request.id)
                .ok_or("作品已下架")?;
            let _guard = mutation.lock().map_err(|_| "库锁不可用")?;
            let original = load_library();
            plan::validate(&request, &plan::build(current, &original)?)?;
            crate::tasks::checkpoint(&app)?;
            crate::tasks::commit_phase(&app)?;
            let (updated, count) =
                apply_transaction(&work, &paths, &original, &stage, &request.apply_files)?;
            let remaining = all_rels(&updated);
            for rel in all_rels(&original).difference(&remaining) {
                let _ = fs::remove_file(vault_dir().join(rel));
            }
            Ok(count)
        })();
        if result.is_ok() {
            let _ = crate::staging::remove(&stage);
        }
        result
    })
    .await
    .map_err(|_| "工坊安装任务中断")?
}

#[tauri::command]
pub(crate) async fn workshop_uninstall(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
        let mut lib = load_library();
        let original = lib.clone();
        let installed = lib.workshop.get(&id).cloned().ok_or("作品尚未安装")?;
        let touched: HashSet<String> = installed
            .files
            .iter()
            .map(|f| f.canonical.clone())
            .collect();
        let stage = crate::staging::create("workshop-uninstall")?;
        ensure_dir(&stage).map_err(|e| e.to_string())?;
        let modimg = PathBuf::from(lib.settings.modimg_path.as_str());
        let backup = if !lib.settings.modimg_path.is_empty() {
            backup_modimg_for_queue(&modimg, &touched, &stage)?
        } else {
            Vec::new()
        };
        let mut removed = Vec::new();
        let mut retained = Vec::new();
        let result = (|| -> Result<(), String> {
            for file in installed.files {
                let active = resource_mut(&mut lib, &file)
                    .is_some_and(|r| r.active_hash.as_deref() == Some(file.hash.as_str()));
                if active {
                    if lib.settings.modimg_path.is_empty() {
                        retained.push(file);
                        continue;
                    }
                    let game_files = canonical_modimg_files(&modimg, &file.canonical)?;
                    if game_files
                        .iter()
                        .any(|path| sha256_file(path).ok().as_deref() != Some(file.hash.as_str()))
                    {
                        retained.push(file);
                        continue;
                    }
                }
                if remove_owned(&mut lib, &file, true) {
                    if active && !lib.settings.modimg_path.is_empty() {
                        purge_modimg(&modimg, &file.canonical)?;
                    }
                    removed.push(file.rel_path);
                } else {
                    retained.push(file);
                }
            }
            if retained.is_empty() {
                lib.workshop.remove(&id);
            } else {
                lib.workshop.insert(
                    id.clone(),
                    InstalledWork {
                        version: installed.version,
                        archive_hash: installed.archive_hash,
                        files: retained.clone(),
                    },
                );
            }
            save_library(&lib)?;
            Ok(())
        })();
        if result.is_err() {
            if !original.settings.modimg_path.is_empty() {
                restore_modimg_queue_backup(&modimg, &touched, &backup)
                    .map_err(|e| format!("回滚失败：{e}；备份保留于 {}", stage.display()))?;
            }
        } else {
            let remaining = all_rels(&lib);
            for rel in &removed {
                if !remaining.contains(rel) {
                    let _ = fs::remove_file(vault_dir().join(rel));
                }
            }
        }
        if result.is_ok() {
            let _ = crate::staging::remove(&stage);
        }
        result.map(|_| serde_json::json!({"removed":removed.len(), "retained":retained.len()}))
    })
    .await
    .map_err(|e| e.to_string())?
}
