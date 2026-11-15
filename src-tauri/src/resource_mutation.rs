//! 从旧版迁入的领域实现；保留协议与行为，在明确边界内逐步替换。
use crate::*;

/* ============ 应用 / 还原 ============ */

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActiveArgs {
    pub(crate) hero_id: String,
    pub(crate) section_key: String,
    pub(crate) canonical: String,
    pub(crate) hash: Option<String>,
}

pub(crate) fn canonical_modimg_files(
    modimg: &Path,
    canonical: &str,
) -> Result<Vec<PathBuf>, String> {
    if !modimg.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for e in fs::read_dir(modimg).map_err(|e| e.to_string())?.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let n = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let visible = n.strip_suffix(".disabled").unwrap_or(&n);
        let stem = visible.rsplit_once('.').map(|(a, _)| a).unwrap_or(visible);
        if stem == canonical {
            out.push(p);
        }
    }
    Ok(out)
}

/// 删除 modimg 中该 canonical 的所有同名文件（不同扩展名）。
/// 失败必须向上返回，否则库记录会领先于游戏实际读取的文件。
pub(crate) fn purge_modimg(modimg: &Path, canonical: &str) -> Result<(), String> {
    for p in canonical_modimg_files(modimg, canonical)? {
        fs::remove_file(&p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn vault_path(vault: &Path, entry: &FileEntry) -> PathBuf {
    if entry.rel_path.is_empty() {
        vault.join(format!("{}{}", entry.hash, entry.ext))
    } else {
        vault.join(&entry.rel_path)
    }
}

#[derive(Debug)]
pub(crate) struct GameFileBackup {
    pub(crate) original: PathBuf,
    pub(crate) snapshot: PathBuf,
}

/// Package imports may apply several resources. `write_entry_to_modimg` is
/// atomic per resource, but this snapshot makes the entire queue atomic if a
/// later resource or the final library publish fails.
pub(crate) fn backup_modimg_for_queue(
    modimg: &Path,
    canonicals: &HashSet<String>,
    stage: &Path,
) -> Result<Vec<GameFileBackup>, String> {
    let backup_dir = stage.join("modimg-backup");
    ensure_dir(&backup_dir).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for canonical in canonicals {
        for (index, original) in canonical_modimg_files(modimg, canonical)?
            .into_iter()
            .enumerate()
        {
            let snapshot = backup_dir.join(format!("{}-{}", sanitize_name(canonical), index));
            fs::copy(&original, &snapshot).map_err(|e| format!("备份游戏文件失败：{}", e))?;
            out.push(GameFileBackup { original, snapshot });
        }
    }
    Ok(out)
}

pub(crate) fn restore_modimg_queue_backup(
    modimg: &Path,
    canonicals: &HashSet<String>,
    backups: &[GameFileBackup],
) -> Result<(), String> {
    for canonical in canonicals {
        purge_modimg(modimg, canonical)?;
    }
    for backup in backups {
        if let Some(parent) = backup.original.parent() {
            let _ = ensure_dir(parent);
        }
        fs::copy(&backup.snapshot, &backup.original).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Write first, then let the caller persist active_hash. The temporary name has
/// a different stem so purge_modimg never removes the prepared replacement.
pub(crate) fn write_entry_to_modimg(
    modimg: &Path,
    canonical: &str,
    src: &Path,
    ext: &str,
) -> Result<String, String> {
    ensure_dir(modimg).map_err(|e| e.to_string())?;
    if !src.exists() {
        return Err(format!("源文件丢失：{}", src.display()));
    }
    let tmp = modimg.join(format!(
        ".astral-pending-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::copy(src, &tmp).map_err(|e| e.to_string())?;
    let old_files = match canonical_modimg_files(modimg, canonical) {
        Ok(files) => files,
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
    };
    let mut backups = Vec::new();
    for (index, old) in old_files.iter().enumerate() {
        let backup = modimg.join(format!(
            ".astral-backup-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            index
        ));
        if let Err(e) = fs::rename(old, &backup) {
            for (original, saved) in old_files.iter().zip(backups.iter()) {
                let _ = fs::rename(saved, original);
            }
            let _ = fs::remove_file(&tmp);
            return Err(e.to_string());
        }
        backups.push(backup);
    }
    let dst = modimg.join(format!("{}{}", canonical, ext));
    if let Err(e) = fs::rename(&tmp, &dst) {
        for (original, saved) in old_files.iter().zip(backups.iter()) {
            let _ = fs::rename(saved, original);
        }
        let _ = fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    for backup in backups {
        let _ = fs::remove_file(backup);
    }
    Ok(dst.to_string_lossy().to_string())
}

pub(crate) fn activate_resource(
    lib: &mut Library,
    hero_id: &str,
    section_key: &str,
    canonical: &str,
    hash: &str,
) -> Result<String, String> {
    let modimg = lib.settings.modimg_path.clone();
    if modimg.is_empty() {
        return Err("未设置 modimg 目录，请先在设置中定位游戏".into());
    }
    let cat = category_of(section_key);
    let entry = lib_map(lib, &cat)
        .get(hero_id)
        .and_then(|h| h.sections.get(section_key))
        .and_then(|s| s.resources.get(canonical))
        .and_then(|r| r.files.iter().find(|f| f.hash == hash))
        .cloned()
        .ok_or("贴图不在库中")?;
    if !is_png_mod_ext(&entry.ext) {
        return Err("仅 PNG Mod 可以应用".into());
    }
    if lib_map(lib, &cat)
        .get(hero_id)
        .and_then(|h| h.sections.get(section_key))
        .and_then(|s| s.resources.get(canonical))
        .map(|r| r.disabled)
        .unwrap_or(false)
    {
        return Err("该 Mod 已禁用，请先启用后再应用".into());
    }
    if section_key == "temporary"
        || hero_id.starts_with("temporary:")
        || canonical.starts_with("unmatched:")
    {
        let source_name = sanitize_name(&entry.source_name);
        let stem = source_name
            .rsplit_once('.')
            .map(|(name, _)| name.to_string())
            .unwrap_or_else(|| source_name.clone());
        let dst = write_entry_to_modimg(
            Path::new(&modimg),
            &stem,
            &vault_path(&vault_dir(), &entry),
            &entry.ext,
        )?;
        let resource = get_resource(lib, &cat, hero_id, section_key, canonical);
        resource.external_present = true;
        resource.active_hash = None;
        return Ok(dst);
    }
    let dst = write_entry_to_modimg(
        Path::new(&modimg),
        canonical,
        &vault_path(&vault_dir(), &entry),
        &entry.ext,
    )?;
    get_resource(lib, &cat, hero_id, section_key, canonical).active_hash = Some(hash.to_string());
    Ok(dst)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn set_active(
    args: ActiveArgs,
    state: tauri::State<AppState>,
) -> Result<String, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    let hash = args.hash.clone().ok_or("缺少 hash")?;
    let dst = activate_resource(
        &mut lib,
        &args.hero_id,
        &args.section_key,
        &args.canonical,
        &hash,
    )?;
    save_library(&lib)?;
    write_log(
        "info",
        "mod.apply",
        &format!(
            "已应用 {}/{}/{}",
            args.hero_id, args.section_key, args.canonical
        ),
        None,
    );
    Ok(dst)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn clear_active(args: ActiveArgs, state: tauri::State<AppState>) -> Result<(), String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    let modimg = lib.settings.modimg_path.clone();
    let cat = category_of(&args.section_key);
    let exists = lib_map(&lib, &cat)
        .get(&args.hero_id)
        .and_then(|h| h.sections.get(&args.section_key))
        .and_then(|s| s.resources.get(&args.canonical))
        .is_some();
    if !exists {
        return Ok(());
    }
    if lib_map(&lib, &cat)
        .get(&args.hero_id)
        .and_then(|h| h.sections.get(&args.section_key))
        .and_then(|s| s.resources.get(&args.canonical))
        .map(|resource| resource.disabled)
        .unwrap_or(false)
    {
        return Err("该 Mod 已禁用，仅可启用、定位或导出".into());
    }
    if !modimg.is_empty() {
        purge_modimg(Path::new(&modimg), &args.canonical)?;
    }
    let resource = get_resource(
        &mut lib,
        &cat,
        &args.hero_id,
        &args.section_key,
        &args.canonical,
    );
    resource.active_hash = None;
    resource.disabled = false;
    save_library(&lib)
}

/// Rename only the game-facing copy.  The vault remains immutable, while the
/// `.disabled` suffix is visible to users and ignored by Astral Party.
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn mod_set_disabled(
    args: ActiveArgs,
    disabled: bool,
    state: tauri::State<AppState>,
) -> Result<bool, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    let cat = category_of(&args.section_key);

    // Temporary resources do not have a valid game-facing canonical filename,
    // so their state is represented by the vault filename itself.  Keeping the
    // same `.disabled` convention lets a later scan distinguish entries that
    // are intentionally held back from automatic reclassification.
    if args.section_key == "temporary" && args.hero_id.starts_with("temporary:") {
        let hash = args.hash.clone().ok_or("缺少 hash")?;
        let (entry, already_disabled) = {
            let resource = lib_map(&lib, "heroes")
                .get(&args.hero_id)
                .and_then(|h| h.sections.get("temporary"))
                .and_then(|s| s.resources.get(&args.canonical))
                .ok_or("临时资源不存在")?;
            let entry = resource
                .files
                .iter()
                .find(|f| f.hash == hash)
                .cloned()
                .ok_or("临时 Mod 不存在")?;
            (entry, resource.disabled)
        };
        if already_disabled == disabled {
            return Ok(disabled);
        }
        let vault = vault_dir();
        let source = vault_path(&vault, &entry);
        if !source.is_file() {
            return Err("临时 Mod 文件不存在，无法切换状态".into());
        }
        let target = if disabled {
            PathBuf::from(format!("{}.disabled", source.to_string_lossy()))
        } else {
            let name = source
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("临时 Mod 文件名无效")?;
            let normal = name
                .strip_suffix(".disabled")
                .ok_or("临时 Mod 禁用文件后缀无效")?;
            source.with_file_name(normal)
        };
        if target.exists() {
            return Err("目标状态文件已存在，拒绝覆盖".into());
        }
        fs::rename(&source, &target)
            .map_err(|e| format!("{}失败：{}", if disabled { "禁用" } else { "启用" }, e))?;
        let rel = target
            .strip_prefix(&vault)
            .unwrap_or(&target)
            .to_string_lossy()
            .replace('\\', "/");
        if let Some(resource) = lib_map_mut(&mut lib, "heroes")
            .get_mut(&args.hero_id)
            .and_then(|h| h.sections.get_mut("temporary"))
            .and_then(|s| s.resources.get_mut(&args.canonical))
        {
            if let Some(file) = resource.files.iter_mut().find(|f| f.hash == hash) {
                file.rel_path = rel;
            }
            resource.disabled = disabled;
        }
        save_library(&lib)?;
        write_log(
            "info",
            if disabled {
                "mod.disable"
            } else {
                "mod.enable"
            },
            &format!("{}/{}/{}", args.hero_id, args.section_key, args.canonical),
            None,
        );
        return Ok(disabled);
    }

    let resource = lib_map(&lib, &cat)
        .get(&args.hero_id)
        .and_then(|h| h.sections.get(&args.section_key))
        .and_then(|s| s.resources.get(&args.canonical))
        .ok_or("资源不存在")?;
    if resource.disabled == disabled {
        return Ok(disabled);
    }
    let hash = resource
        .active_hash
        .clone()
        .ok_or("只有已应用的 Mod 可以禁用")?;
    let entry = resource
        .files
        .iter()
        .find(|f| f.hash == hash)
        .cloned()
        .ok_or("已应用 Mod 的源文件不存在")?;
    let modimg = lib.settings.modimg_path.clone();
    if modimg.is_empty() {
        return Err("未设置 modimg 目录，请先定位游戏".into());
    }
    let normal = Path::new(&modimg).join(format!("{}{}", args.canonical, entry.ext));
    let parked = Path::new(&modimg).join(format!("{}{}.disabled", args.canonical, entry.ext));
    if disabled {
        if !normal.exists() {
            return Err("当前应用文件不存在，无法禁用".into());
        }
        if parked.exists() {
            return Err("已存在同名禁用文件，拒绝覆盖".into());
        }
        fs::rename(&normal, &parked).map_err(|e| format!("禁用失败：{}", e))?;
    } else {
        if !parked.exists() {
            return Err("禁用文件不存在，无法启用".into());
        }
        if normal.exists() {
            return Err("已存在同名启用文件，拒绝覆盖".into());
        }
        fs::rename(&parked, &normal).map_err(|e| format!("启用失败：{}", e))?;
    }
    get_resource(
        &mut lib,
        &cat,
        &args.hero_id,
        &args.section_key,
        &args.canonical,
    )
    .disabled = disabled;
    save_library(&lib)?;
    write_log(
        "info",
        if disabled {
            "mod.disable"
        } else {
            "mod.enable"
        },
        &format!("{}/{}/{}", args.hero_id, args.section_key, args.canonical),
        None,
    );
    Ok(disabled)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn remove_file(args: ActiveArgs, state: tauri::State<AppState>) -> Result<(), String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    let hash = args.hash.clone().ok_or("缺少 hash")?;
    let vd = vault_dir();
    let cat = category_of(&args.section_key);
    if lib_map(&lib, &cat)
        .get(&args.hero_id)
        .and_then(|h| h.sections.get(&args.section_key))
        .and_then(|s| s.resources.get(&args.canonical))
        .map(|resource| resource.disabled)
        .unwrap_or(false)
    {
        return Err("该 Mod 已禁用，仅可启用、定位或导出".into());
    }
    let mut to_delete: Option<PathBuf> = None;
    let modimg = lib.settings.modimg_path.clone();

    if let Some(res) = lib_map_mut(&mut lib, &cat)
        .get_mut(&args.hero_id)
        .and_then(|h| h.sections.get_mut(&args.section_key))
        .and_then(|s| s.resources.get_mut(&args.canonical))
    {
        let was_active = res.active_hash.as_deref() == Some(hash.as_str());
        let mut removed: Option<FileEntry> = None;
        res.files.retain(|f| {
            if f.hash == hash {
                removed = Some(f.clone());
                false
            } else {
                true
            }
        });
        if let Some(f) = removed {
            if !f.rel_path.is_empty() {
                let p = vd.join(&f.rel_path);
                if p.exists() {
                    to_delete = Some(p);
                }
            }
        }
        if was_active {
            res.active_hash = None;
        }
    }
    if !modimg.is_empty() {
        let original = load_library();
        let was_active = lib_map(&original, &cat)
            .get(&args.hero_id)
            .and_then(|h| h.sections.get(&args.section_key))
            .and_then(|s| s.resources.get(&args.canonical))
            .is_some_and(|r| r.active_hash.as_deref() == Some(hash.as_str()));
        if was_active {
            crate::resource_transaction::apply_batch(
                Path::new(&modimg),
                &[(args.canonical.clone(), None)],
                || save_library(&lib),
            )?;
        } else {
            save_library(&lib)?;
        }
    } else {
        save_library(&lib)?;
    }
    if let Some(p) = to_delete {
        let shared = [&lib.heroes, &lib.monsters]
            .into_iter()
            .flat_map(|m| m.values())
            .flat_map(|h| h.sections.values())
            .flat_map(|s| s.resources.values())
            .flat_map(|r| r.files.iter())
            .any(|f| vault_path(&vd, f) == p);
        if shared {
            return Ok(());
        }
        let real = p.canonicalize().map_err(|e| e.to_string())?;
        if !real.starts_with(vd.canonicalize().map_err(|e| e.to_string())?) {
            return Err("文件不在资源库内，拒绝删除".into());
        }
        fs::remove_file(&real).map_err(|e| format!("已移除库记录，文件清理失败：{e}"))?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenameArgs {
    pub(crate) hero_id: String,
    pub(crate) section_key: String,
    pub(crate) canonical: String,
    pub(crate) hash: String,
    pub(crate) name: String,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn rename_file(
    args: RenameArgs,
    state: tauri::State<AppState>,
) -> Result<String, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    let cat = category_of(&args.section_key);
    let out = crate::candidate_management::rename_display(
        &mut lib,
        &crate::management_types::ResourceSlot {
            category: cat,
            hero_id: args.hero_id,
            section_key: args.section_key,
            canonical: args.canonical,
        },
        &args.hash,
        args.name,
    )?;
    save_library(&lib)?;
    Ok(out)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn reveal_file(args: ActiveArgs, state: tauri::State<AppState>) -> Result<(), String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let lib = load_library();
    let hash = args.hash.ok_or("缺少 hash")?;
    let vd = vault_dir();
    let cat = category_of(&args.section_key);
    let f = lib_map(&lib, &cat)
        .get(&args.hero_id)
        .and_then(|h| h.sections.get(&args.section_key))
        .and_then(|s| s.resources.get(&args.canonical))
        .and_then(|r| r.files.iter().find(|f| f.hash == hash))
        .cloned()
        .ok_or("贴图不存在")?;
    let p = if !f.rel_path.is_empty() {
        vd.join(&f.rel_path)
    } else {
        vd.join(format!("{}{}", f.hash, f.ext))
    };
    if !p.exists() {
        return Err("图片文件已不存在，无法定位".into());
    }
    let p = p
        .canonicalize()
        .map_err(|e| format!("无法解析图片文件路径：{}", e))?;
    std::process::Command::new("explorer")
        .arg(format!("/select,{}", p.display()))
        .spawn()
        .map_err(|e| format!("无法打开资源管理器：{}", e))?;
    Ok(())
}

/// v0.2 public name. Keep `reveal_file` registered for existing frontends.
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn reveal_mod_file(
    args: ActiveArgs,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    reveal_file(args, state)
}

/// 清理 library.json 中文件已不存在的记录，并删除空 resource/section/hero
pub(crate) fn cleanup_map(map: &mut HashMap<String, HeroEntry>, vd: &Path, removed: &mut usize) {
    let ids: Vec<String> = map.keys().cloned().collect();
    for hid in ids {
        let Some(hero) = map.get_mut(&hid) else {
            continue;
        };
        let section_keys: Vec<String> = hero.sections.keys().cloned().collect();
        for skey in section_keys {
            let Some(sec) = hero.sections.get_mut(&skey) else {
                continue;
            };
            let canonicals: Vec<String> = sec.resources.keys().cloned().collect();
            for cn in canonicals {
                let Some(res) = sec.resources.get_mut(&cn) else {
                    continue;
                };
                let before = res.files.len();
                res.files.retain(|f| {
                    let p = if !f.rel_path.is_empty() {
                        vd.join(&f.rel_path)
                    } else {
                        vd.join(format!("{}{}", f.hash, f.ext))
                    };
                    let exists = p.exists();
                    if !exists {
                        *removed += 1;
                    }
                    exists
                });
                if res
                    .active_hash
                    .as_ref()
                    .is_some_and(|hash| !res.files.iter().any(|f| &f.hash == hash))
                {
                    res.active_hash = None;
                }
                // 如果该 resource 没有任何文件且未设置 active，直接删除
                if res.files.is_empty() && res.active_hash.is_none() && before > 0 {
                    sec.resources.remove(&cn);
                }
            }
            if sec.resources.is_empty() {
                hero.sections.remove(&skey);
            }
        }
        if hero.sections.is_empty() {
            map.remove(&hid);
        }
    }
}

/// 清理 library.json 中文件已不存在的记录，并删除空 resource/section/hero
pub(crate) fn cleanup_library_inner() -> Result<usize, String> {
    let mut lib = load_library();
    let vd = vault_dir();
    let mut removed = 0usize;

    cleanup_map(&mut lib.heroes, &vd, &mut removed);
    cleanup_map(&mut lib.monsters, &vd, &mut removed);

    save_library(&lib)?;
    Ok(removed)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn cleanup_library(state: tauri::State<AppState>) -> Result<usize, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    cleanup_library_inner()
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReconcileResult {
    pub(crate) added: usize,
    pub(crate) activated: usize,
    pub(crate) cleared: usize,
    pub(crate) conflicts: Vec<String>,
    pub(crate) errors: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModLibraryStatus {
    pub(crate) total_mods: usize,
    pub(crate) active_mods: usize,
    pub(crate) temporary_mods: usize,
    pub(crate) disabled_mods: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TemporaryModOut {
    pub(crate) entity_id: String,
    pub(crate) canonical: String,
    pub(crate) hash: String,
    pub(crate) source_name: String,
    pub(crate) url: String,
    pub(crate) rel_path: String,
    pub(crate) disabled: bool,
    pub(crate) external_present: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NonPngModItem {
    pub(crate) path: String,
    pub(crate) format: String,
    pub(crate) source_name: String,
    pub(crate) category: Option<String>,
    pub(crate) entity_id: Option<String>,
    pub(crate) section_key: Option<String>,
    pub(crate) canonical: Option<String>,
    pub(crate) hash: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NonPngModReport {
    pub(crate) active: Vec<NonPngModItem>,
    pub(crate) stored: Vec<NonPngModItem>,
    pub(crate) orphaned: Vec<NonPngModItem>,
}

pub(crate) fn non_png_format_for_name(name: &str) -> Option<String> {
    let base = name.strip_suffix(".disabled").unwrap_or(name);
    let ext = Path::new(base).extension()?.to_str()?.to_ascii_lowercase();
    (!ext.is_empty() && ext != "png").then_some(ext)
}

pub(crate) fn non_png_item(
    path: PathBuf,
    format: String,
    source_name: String,
    category: Option<String>,
    entity_id: Option<String>,
    section_key: Option<String>,
    canonical: Option<String>,
    hash: Option<String>,
) -> NonPngModItem {
    NonPngModItem {
        path: path.to_string_lossy().to_string(),
        format,
        source_name,
        category,
        entity_id,
        section_key,
        canonical,
        hash,
    }
}

pub(crate) fn collect_non_png_from_library(
    map: &HashMap<String, HeroEntry>,
    category: &str,
    vault: &Path,
    modimg: &Path,
    report: &mut NonPngModReport,
    known_modimg: &mut HashSet<PathBuf>,
) {
    for (entity_id, hero) in map {
        for (section_key, section) in &hero.sections {
            for (canonical, resource) in &section.resources {
                for file in &resource.files {
                    if is_png_mod_ext(&file.ext) {
                        continue;
                    }
                    let normal = modimg.join(format!("{}{}", canonical, file.ext));
                    let parked = modimg.join(format!("{}{}.disabled", canonical, file.ext));
                    known_modimg.insert(normal.clone());
                    known_modimg.insert(parked.clone());
                    let active_path = if normal.is_file() {
                        normal
                    } else if parked.is_file() {
                        parked
                    } else {
                        vault_path(vault, file)
                    };
                    let item = non_png_item(
                        active_path,
                        file.ext.trim_start_matches('.').to_ascii_lowercase(),
                        file.source_name.clone(),
                        Some(category.to_string()),
                        Some(entity_id.clone()),
                        Some(section_key.clone()),
                        Some(canonical.clone()),
                        Some(file.hash.clone()),
                    );
                    if resource.active_hash.as_deref() == Some(file.hash.as_str())
                        && !resource.disabled
                    {
                        report.active.push(item);
                    } else {
                        report.stored.push(item);
                    }
                }
            }
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn non_png_mods_check() -> NonPngModReport {
    let lib = load_library();
    let vault = vault_dir();
    let modimg = PathBuf::from(&lib.settings.modimg_path);
    let mut report = NonPngModReport::default();
    let mut known_modimg = HashSet::new();
    collect_non_png_from_library(
        &lib.heroes,
        "heroes",
        &vault,
        &modimg,
        &mut report,
        &mut known_modimg,
    );
    collect_non_png_from_library(
        &lib.monsters,
        "monsters",
        &vault,
        &modimg,
        &mut report,
        &mut known_modimg,
    );
    if modimg.is_dir() {
        if let Ok(entries) = fs::read_dir(&modimg) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() || known_modimg.contains(&path) {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(".astral-") {
                    continue;
                }
                if let Some(format) = non_png_format_for_name(&name) {
                    report.orphaned.push(non_png_item(
                        path, format, name, None, None, None, None, None,
                    ));
                }
            }
        }
    }
    let sort = |items: &mut Vec<NonPngModItem>| items.sort_by(|a, b| a.path.cmp(&b.path));
    sort(&mut report.active);
    sort(&mut report.stored);
    sort(&mut report.orphaned);
    report
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn reveal_non_png_mod(path: String) -> Result<(), String> {
    let lib = load_library();
    let candidate = PathBuf::from(path);
    if !candidate.is_file() {
        return Err("文件不存在".into());
    }
    let vault = vault_dir();
    let modimg = PathBuf::from(&lib.settings.modimg_path);
    if !(candidate.starts_with(&vault)
        || (!lib.settings.modimg_path.is_empty() && candidate.starts_with(&modimg)))
    {
        return Err("文件不在受控目录中".into());
    }
    let name = candidate
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if non_png_format_for_name(name).is_none() {
        return Err("仅可定位非 PNG Mod".into());
    }
    std::process::Command::new("explorer")
        .arg("/select,")
        .arg(&candidate)
        .spawn()
        .map_err(|error| format!("无法打开资源管理器：{}", error))?;
    Ok(())
}

/// The renamed game file is the source of truth.  Reconcile this persisted
/// state on every scan so manual renames and restarts cannot leave the UI
/// believing a Mod is enabled when Astral Party sees `.disabled` (or vice
/// versa).
pub(crate) fn sync_disabled_state_from_modimg(lib: &mut Library) {
    let modimg = PathBuf::from(&lib.settings.modimg_path);
    if lib.settings.modimg_path.is_empty() || !modimg.is_dir() {
        return;
    }
    for map in [&mut lib.heroes, &mut lib.monsters] {
        for hero in map.values_mut() {
            for section in hero.sections.values_mut() {
                for (canonical, resource) in &mut section.resources {
                    let Some(hash) = resource.active_hash.as_ref() else {
                        continue;
                    };
                    let Some(entry) = resource.files.iter().find(|file| &file.hash == hash) else {
                        continue;
                    };
                    let normal = modimg.join(format!("{}{}", canonical, entry.ext));
                    let parked = modimg.join(format!("{}{}.disabled", canonical, entry.ext));
                    resource.disabled = parked.is_file() && !normal.is_file();
                }
            }
        }
    }
}

/// Existing non-PNG entries remain in the vault for inspection, but can no
/// longer be considered active after PNG-only support is enabled.
pub(crate) fn clear_unsupported_active_mods(lib: &mut Library) -> usize {
    let mut cleared = 0usize;
    for map in [&mut lib.heroes, &mut lib.monsters] {
        for hero in map.values_mut() {
            for section in hero.sections.values_mut() {
                for resource in section.resources.values_mut() {
                    let unsupported = resource.active_hash.as_ref().is_some_and(|hash| {
                        resource
                            .files
                            .iter()
                            .any(|file| &file.hash == hash && !is_png_mod_ext(&file.ext))
                    });
                    if unsupported {
                        resource.active_hash = None;
                        resource.disabled = false;
                        cleared += 1;
                    }
                }
            }
        }
    }
    cleared
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn temporary_mods() -> Vec<TemporaryModOut> {
    let lib = load_library();
    let mut out = Vec::new();
    for (entity_id, hero) in &lib.heroes {
        if !entity_id.starts_with("temporary:") {
            continue;
        }
        let Some(section) = hero.sections.get("temporary") else {
            continue;
        };
        for (canonical, resource) in &section.resources {
            for file in &resource.files {
                let path = vault_path(&vault_dir(), file);
                out.push(TemporaryModOut {
                    entity_id: entity_id.clone(),
                    canonical: canonical.clone(),
                    hash: file.hash.clone(),
                    source_name: file.source_name.clone(),
                    url: path.to_string_lossy().to_string(),
                    rel_path: file.rel_path.clone(),
                    disabled: resource.disabled,
                    external_present: resource.external_present,
                });
            }
        }
    }
    out.sort_by(|a, b| a.source_name.cmp(&b.source_name));
    out
}

pub(crate) fn mod_library_status_inner() -> ModLibraryStatus {
    let lib = load_library();
    let mut status = ModLibraryStatus::default();
    for map in [&lib.heroes, &lib.monsters] {
        for (id, hero) in map {
            for (section, section_entry) in &hero.sections {
                for resource in section_entry.resources.values() {
                    let valid_files = resource
                        .files
                        .iter()
                        .filter(|file| is_png_mod_ext(&file.ext))
                        .count();
                    let active_is_png = resource.active_hash.as_ref().is_some_and(|hash| {
                        resource
                            .files
                            .iter()
                            .any(|file| &file.hash == hash && is_png_mod_ext(&file.ext))
                    });
                    status.total_mods += valid_files;
                    let is_temporary = section == "temporary" || id.starts_with("temporary:");
                    if active_is_png && !resource.disabled && !is_temporary {
                        status.active_mods += 1;
                    }
                    if active_is_png && resource.disabled {
                        status.disabled_mods += 1;
                    }
                    if section == "temporary" || id.starts_with("temporary:") {
                        status.temporary_mods += valid_files;
                    }
                }
            }
        }
    }
    status
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn scan_modimg_status() -> ModLibraryStatus {
    mod_library_status_inner()
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn scan_modimg_start(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<ReconcileResult, String> {
    if state.scan_busy.swap(true, Ordering::AcqRel) {
        return Err("modimg 扫描任务正在进行中".into());
    }
    let busy = Arc::clone(&state.scan_busy);
    let mutation = Arc::clone(&state.mutation);
    let _ = app.emit(
        "mod-scan-progress",
        serde_json::json!({"phase":"scan","percent":0,"message":"正在后台同步 modimg"}),
    );
    tauri::async_runtime::spawn_blocking(move || {
        let result = mutation
            .lock()
            .map_err(|_| "库写入锁已损坏".to_string())
            .and_then(|_guard| reconcile_inner(Some(&app)));
        match &result {
            Ok(summary) => {
                let _ = app.emit("mod-scan-progress", serde_json::json!({
                    "phase":"complete", "percent":100, "message":"modimg 同步完成",
                    "added":summary.added, "activated":summary.activated, "cleared":summary.cleared
                }));
                write_log("info", "modimg.scan", "后台同步完成", None);
            }
            Err(error) => {
                let _ = app.emit(
                    "mod-scan-progress",
                    serde_json::json!({"phase":"error","percent":100,"message":error}),
                );
                write_log(
                    "error",
                    "modimg.scan",
                    error,
                    Some(&format!("{:?}", std::backtrace::Backtrace::force_capture())),
                );
            }
        }
        busy.store(false, Ordering::Release);
    });
    // Work proceeds in Rust's blocking pool.  A short acknowledgement keeps
    // the WebView interactive; completion details arrive through the event.
    Ok(ReconcileResult::default())
}

#[derive(Clone)]
pub(crate) struct ObservedMod {
    pub(crate) path: PathBuf,
    pub(crate) parsed: Parsed,
    pub(crate) cache_key: String,
}

pub(crate) fn supported_mod_extension(name: &str) -> bool {
    matches!(
        Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("png")
    )
}

pub(crate) fn register_external_mod(
    lib: &mut Library,
    vault: &Path,
    observed: &ObservedMod,
    cache: &mut ModScanCache,
) -> Result<(String, bool), String> {
    let name = observed
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = ext_of(&name);
    let hash = cached_file_hash_for_key(&observed.path, cache, &observed.cache_key)?;
    let category = category_of(&observed.parsed.section_key);
    let (dest, rel) = place_in_vault(
        vault,
        &category,
        &observed.parsed.hero_id,
        &observed.parsed.section_key,
        &name,
        &hash,
        &ext,
    )?;
    if !dest.exists() {
        fs::copy(&observed.path, &dest).map_err(|e| e.to_string())?;
    }
    let cat = category_of(&observed.parsed.section_key);
    let resource = get_resource(
        lib,
        &cat,
        &observed.parsed.hero_id,
        &observed.parsed.section_key,
        &observed.parsed.canonical,
    );
    let added = !resource.files.iter().any(|f| f.hash == hash);
    if added {
        resource.files.push(FileEntry {
            hash: hash.clone(),
            ext,
            source_name: name,
            rel_path: rel,
            added_at: 0,
            info: Default::default(),
            classification: Default::default(),
            fs_identity: String::new(),
        });
    }
    Ok((hash, added))
}

/// A later section-rule update can make a formerly unknown filename
/// recognisable.  Move its existing vault entry instead of importing a second
/// copy; the hash remains stable and user-visible names are preserved.
pub(crate) fn migrate_temporary_entries_in_vault(lib: &mut Library, vault: &Path) -> usize {
    let count = |lib: &Library| {
        crate::candidate_management::candidates(lib)
            .iter()
            .filter(|(s, _, _)| s.section_key == "temporary")
            .count()
    };
    let before = count(lib);
    let _ = crate::vault_sync::synchronize(lib, vault, |_| Ok(()), || Ok(()));
    before.saturating_sub(count(lib))
}

pub(crate) fn migrate_temporary_entries(lib: &mut Library) -> usize {
    migrate_temporary_entries_in_vault(lib, &vault_dir())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn reconcile(state: tauri::State<AppState>) -> Result<ReconcileResult, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    reconcile_inner(None)
}

/// Core modimg reconciliation.  Callers must own the mutation lock.  The
/// optional AppHandle is deliberately kept out of ordinary mutations so the
/// same safe reconciliation remains usable during startup and in the worker.
pub(crate) fn reconcile_inner(app: Option<&tauri::AppHandle>) -> Result<ReconcileResult, String> {
    if let Some(app) = app {
        crate::tasks::checkpoint(app)?;
    }
    let mut lib = load_library();
    let game = PathBuf::from(&lib.settings.modimg_path);
    let before = crate::candidate_management::candidates(&lib).len();
    let result = crate::directory_sync::synchronize_controlled(
        &mut lib,
        &vault_dir(),
        &game,
        |lib| {
            if let Some(app) = app {
                crate::tasks::commit_phase(app)?;
            }
            save_library(lib)
        },
        || {
            if let Some(app) = app {
                crate::tasks::checkpoint(app)?;
            }
            Ok(())
        },
    )?;
    if let Some(app) = app {
        crate::tasks::commit_phase(app)?;
    }
    if result.changed {
        save_library(&lib)?;
    }
    Ok(ReconcileResult {
        added: crate::candidate_management::candidates(&lib)
            .len()
            .saturating_sub(before),
        errors: result.errors,
        ..Default::default()
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preview_status() -> PreviewStatus {
    preview_cache_status()
}

pub(crate) fn preview_check_update_blocking_legacy() -> PreviewStatus {
    let mut status = preview_cache_status();
    match fetch_preview_manifest(None, None) {
        Ok(remote) => {
            status.remote_version = Some(remote.version.clone());
            status.remote_bytes = Some(remote.bytes);
            status.update_available = Some(
                status.missing_files > 0
                    || status.local_version.as_deref() != Some(remote.version.as_str()),
            );
        }
        Err(error) => status.error = Some(error),
    }
    status
}

pub(crate) fn preview_check_update_blocking() -> PreviewStatus {
    let mut status = preview_cache_status();
    match fetch_preview_manifest(None, None) {
        Ok(remote) => {
            status.remote_version = remote
                .bundles
                .iter()
                .map(|bundle| bundle.version.clone())
                .max();
            status.remote_bytes = Some(remote.bundles.iter().map(|bundle| bundle.bytes).sum());
            status.update_available = Some(remote.bundles.iter().any(|bundle| {
                let local_key = if bundle.key == "all" {
                    "heroes"
                } else {
                    bundle.key.as_str()
                };
                status
                    .bundles
                    .iter()
                    .find(|local| local.key == local_key)
                    .and_then(|local| local.local_version.as_deref())
                    != Some(bundle.version.as_str())
            }));
        }
        Err(error) => status.error = Some(error),
    }
    status
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn preview_check_update() -> Result<PreviewStatus, String> {
    tauri::async_runtime::spawn_blocking(preview_check_update_blocking)
        .await
        .map_err(|error| format!("检查预览更新任务失败：{}", error))
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn emit_preview_progress(
    app: &tauri::AppHandle,
    phase: &str,
    percent: Option<u8>,
    message: &str,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
) {
    let _ = app.emit(
        "preview-download-progress",
        serde_json::json!({
            "phase": phase,
            "percent": percent,
            "downloadedBytes": downloaded_bytes,
            "totalBytes": total_bytes,
            "message": message
        }),
    );
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn bundle_display_name(key: &str) -> &'static str {
    match key {
        "heroes" => "英雄",
        "cards" => "卡面",
        "monsters" => "怪物",
        _ => "预览",
    }
}

pub(crate) fn download_one_preview_bundle(
    remote: &RemotePreviewManifest,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
) -> Result<u64, String> {
    emit_preview_progress(
        app,
        "probe",
        None,
        &format!("正在测试{}更新源", bundle_display_name(&remote.bundle_key)),
        Some(0),
        Some(remote.bytes),
    );
    let ranked_sources = ranked_preview_bundle_sources(&remote.bundle_url);
    let mut source_errors = Vec::new();
    for probe in ranked_sources {
        if cancel.load(Ordering::Relaxed) {
            return Err("用户取消下载".into());
        }
        emit_preview_progress(
            app,
            "download",
            Some(0),
            &format!(
                "正在通过{}下载{}",
                probe.source,
                bundle_display_name(&remote.bundle_key)
            ),
            Some(0),
            Some(remote.bytes),
        );
        match download_preview_bundle(probe.source, &probe.url, remote, Some(app), Some(cancel)) {
            Ok(bytes) => {
                emit_preview_progress(
                    app,
                    "verify",
                    None,
                    &format!("正在校验{}", bundle_display_name(&remote.bundle_key)),
                    Some(bytes.len() as u64),
                    Some(remote.bytes),
                );
                if sha256_bytes(&bytes) != remote.sha256.to_ascii_lowercase() {
                    clear_preview_download_part(&remote.bundle_key);
                    return Err(format!(
                        "{}包的 SHA-256 校验失败",
                        bundle_display_name(&remote.bundle_key)
                    ));
                }
                emit_preview_progress(
                    app,
                    "extract",
                    None,
                    &format!("正在安装{}", bundle_display_name(&remote.bundle_key)),
                    Some(bytes.len() as u64),
                    Some(remote.bytes),
                );
                if let Err(error) = install_preview_bundle_section(&bytes, remote, Some(cancel)) {
                    clear_preview_download_part(&remote.bundle_key);
                    return Err(error);
                }
                clear_preview_download_part(&remote.bundle_key);
                return Ok(bytes.len() as u64);
            }
            Err(error) => source_errors.push(format!("{}: {}", probe.source, error)),
        }
    }
    Err(format!(
        "{}包下载失败：{}",
        bundle_display_name(&remote.bundle_key),
        source_errors.join("；")
    ))
}

pub(crate) fn preview_download_blocking(
    force: Option<bool>,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<PreviewStatus, String> {
    let result = (|| {
        emit_preview_progress(&app, "manifest", Some(0), "正在获取更新信息", None, None);
        let manifest = fetch_preview_manifest(Some(&app), Some(&cancel))?;
        let force = force.unwrap_or(false);
        let status = preview_cache_status();
        let mut to_download: Vec<RemotePreviewManifest> = manifest
            .bundles
            .iter()
            .map(remote_as_bundle)
            .filter(|bundle| {
                if force {
                    return true;
                }
                let key = if bundle.bundle_key == "all" {
                    "heroes"
                } else {
                    bundle.bundle_key.as_str()
                };
                status
                    .bundles
                    .iter()
                    .find(|local| local.key == key)
                    .and_then(|local| local.local_version.as_deref())
                    != Some(bundle.version.as_str())
            })
            .collect();
        if to_download.is_empty() {
            emit_preview_progress(
                &app,
                "complete",
                Some(100),
                "预览已是最新版本",
                Some(0),
                Some(0),
            );
            return Ok(status);
        }
        let total = to_download.len();
        let mut installed = Vec::new();
        let mut failures = Vec::new();
        for (index, remote) in to_download.drain(..).enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Err("用户取消下载".into());
            }
            emit_preview_progress(
                &app,
                "probe",
                None,
                &format!(
                    "正在更新{}（{}/{}）",
                    bundle_display_name(&remote.bundle_key),
                    index + 1,
                    total
                ),
                Some(0),
                Some(remote.bytes),
            );
            match download_one_preview_bundle(&remote, &app, &cancel) {
                Ok(_) => installed.push(remote.bundle_key.clone()),
                Err(error) if error == "用户取消下载" => return Err(error),
                Err(error) => failures.push(error),
            }
        }
        if !failures.is_empty() {
            return Err(format!(
                "{}；已完成：{}",
                failures.join("；"),
                if installed.is_empty() {
                    "无".into()
                } else {
                    installed.join("、")
                }
            ));
        }
        emit_preview_progress(&app, "complete", Some(100), "更新完成", None, None);
        Ok(preview_cache_status())
    })();
    if let Err(error) = &result {
        emit_preview_progress(
            &app,
            if error == "用户取消下载" {
                "cancelled"
            } else {
                "error"
            },
            None,
            error,
            None,
            None,
        );
    }
    result
}

pub(crate) fn preview_download_blocking_legacy(
    force: Option<bool>,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<PreviewStatus, String> {
    let result: Result<PreviewStatus, String> = (|| {
        emit_preview_progress(&app, "manifest", Some(5), "正在获取更新信息…", None, None);
        let remote = fetch_preview_manifest(Some(&app), Some(&cancel))?;
        let status = preview_cache_status();
        if !force.unwrap_or(false)
            && status.missing_files == 0
            && status.local_version.as_deref() == Some(remote.version.as_str())
        {
            emit_preview_progress(
                &app,
                "complete",
                Some(100),
                "已是最新版本",
                Some(0),
                Some(0),
            );
            return Ok(status);
        }
        emit_preview_progress(
            &app,
            "probe",
            None,
            "正在测试各下载源速度…",
            Some(0),
            Some(remote.bytes),
        );
        let ranked_sources = ranked_preview_bundle_sources(&remote.bundle_url);
        let mut bundle = None;
        let mut source_errors = Vec::new();
        for (index, probe) in ranked_sources.into_iter().enumerate() {
            let source = probe.source;
            let url = probe.url;
            if index > 0 {
                emit_preview_progress(
                    &app,
                    "download",
                    Some(0),
                    &format!("上一下载源不可用，正在切换{}…", source),
                    Some(0),
                    Some(remote.bytes),
                );
            } else {
                emit_preview_progress(
                    &app,
                    "probe",
                    None,
                    &format!(
                        "已选择{}（测速 {:.1} MB/s）",
                        source,
                        probe.bytes_per_second as f64 / 1_048_576.0
                    ),
                    Some(0),
                    Some(remote.bytes),
                );
            }
            match download_preview_bundle(source, &url, &remote, Some(&app), Some(&cancel)) {
                Ok(bytes) => {
                    bundle = Some(bytes);
                    break;
                }
                Err(error) => source_errors.push(format!("{}：{}", source, error)),
            }
        }
        let bundle =
            bundle.ok_or_else(|| format!("预览包下载失败：{}", source_errors.join("；")))?;
        let downloaded = bundle.len() as u64;
        emit_preview_progress(
            &app,
            "download",
            Some(60),
            &format!("已下载 {:.1} MB", downloaded as f64 / 1_048_576.0),
            Some(downloaded),
            Some(remote.bytes),
        );
        emit_preview_progress(
            &app,
            "verify",
            Some(70),
            "正在校验完整性…",
            Some(downloaded),
            Some(remote.bytes),
        );
        emit_preview_progress(
            &app,
            "extract",
            Some(85),
            "正在安装更新…",
            Some(downloaded),
            Some(remote.bytes),
        );
        if !remote.sha256.is_empty() && sha256_bytes(&bundle) != remote.sha256.to_ascii_lowercase()
        {
            clear_preview_download_part("all");
            return Err("Preview bundle SHA-256 verification failed".into());
        }
        if let Err(error) = install_preview_bundle(&bundle, &remote, Some(&cancel)) {
            clear_preview_download_part("all");
            return Err(error);
        }
        clear_preview_download_part("all");
        emit_preview_progress(
            &app,
            "complete",
            Some(100),
            "更新完成",
            Some(downloaded),
            Some(remote.bytes),
        );
        Ok(preview_cache_status())
    })();
    if let Err(error) = &result {
        let phase = if error == "用户取消下载" {
            "cancelled"
        } else {
            "error"
        };
        let message = if phase == "cancelled" {
            "已取消预览下载"
        } else {
            error.as_str()
        };
        emit_preview_progress(&app, phase, None, message, None, None);
    }
    result
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn preview_download(
    force: Option<bool>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<PreviewStatus, String> {
    if state.preview_busy.swap(true, Ordering::AcqRel) {
        return Err("预览下载任务正在进行中".into());
    }
    state.preview_cancel.store(false, Ordering::Release);
    let busy = Arc::clone(&state.preview_busy);
    let cancel = Arc::clone(&state.preview_cancel);
    let result =
        tauri::async_runtime::spawn_blocking(move || preview_download_blocking(force, app, cancel))
            .await
            .map_err(|error| format!("预览下载任务失败：{}", error));
    busy.store(false, Ordering::Release);
    result?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preview_cancel(state: tauri::State<AppState>) -> Result<(), String> {
    state.preview_cancel.store(true, Ordering::Release);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preview_clear() -> Result<PreviewStatus, String> {
    let cache = preview_cache_dir();
    if cache.exists() {
        fs::remove_dir_all(cache).map_err(|e| e.to_string())?;
    }
    Ok(preview_cache_status())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn outimg_reveal(path: String) -> Result<(), String> {
    let settings = load_library().settings;
    let root = effective_outimg_dir(&settings).ok_or_else(|| "尚未设置 outimg 目录".to_string())?;
    let candidate = PathBuf::from(&path);
    let canonical_root = root
        .canonicalize()
        .map_err(|_| "outimg 目录不存在".to_string())?;
    let canonical_file = candidate
        .canonicalize()
        .map_err(|_| "outimg 文件不存在".to_string())?;
    if !canonical_file.starts_with(&canonical_root) || !canonical_file.is_file() {
        return Err("目标文件不在 outimg 目录内".into());
    }
    std::process::Command::new("explorer")
        .arg("/select,")
        .arg(&canonical_file)
        .spawn()
        .map_err(|e| format!("无法定位 outimg 文件：{}", e))?;
    Ok(())
}

pub(crate) fn browser_drop_root() -> PathBuf {
    std::env::temp_dir().join("astral-apmm-browser-drop")
}

pub(crate) fn browser_drop_dir(id: &str) -> Result<PathBuf, String> {
    if id.len() != 16 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("无效的拖入任务标识".into());
    }
    Ok(browser_drop_root().join(id))
}

pub(crate) fn browser_drop_file(id: &str) -> Result<PathBuf, String> {
    let dir = browser_drop_dir(id)?;
    let mut entries = fs::read_dir(&dir).map_err(|_| "拖入任务不存在或已结束".to_string())?;
    let entry = entries
        .next()
        .ok_or_else(|| "拖入文件不存在".to_string())?
        .map_err(|error| error.to_string())?;
    if entries.next().is_some()
        || !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
    {
        return Err("拖入任务文件异常".into());
    }
    Ok(entry.path())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn browser_drop_begin(name: String, size: u64) -> Result<String, String> {
    if size == 0 || size > 1024 * 1024 * 1024 {
        return Err("拖入文件为空或超过 1 GB，请使用文件选择导入".into());
    }
    if name.len() > 255
        || name.is_empty()
        || name == "."
        || name == ".."
        || name.chars().any(|c| {
            matches!(
                c,
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'
            )
        })
        || !["png", "apmod"].iter().any(|ext| {
            name.rsplit('.')
                .next()
                .is_some_and(|found| found.eq_ignore_ascii_case(ext))
        })
    {
        return Err("拖入文件名无效或格式不受支持".into());
    }
    let root = browser_drop_root();
    ensure_dir(&root).map_err(|error| format!("无法创建拖入暂存目录：{}", error))?;
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.len() != 16 || !name.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                continue;
            }
            if !entry
                .file_type()
                .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink())
            {
                continue;
            }
            let expired = entry
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|age| age.as_secs() > 24 * 60 * 60);
            if expired && entry.path().parent() == Some(root.as_path()) {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
    for _ in 0..8 {
        let id = format!("{:016x}", rand::random::<u64>());
        let dir = root.join(&id);
        if fs::create_dir(&dir).is_err() {
            continue;
        }
        if let Err(error) = fs::File::create(dir.join(&name)) {
            let _ = fs::remove_dir(&dir);
            return Err(format!("无法暂存拖入文件：{}", error));
        }
        return Ok(id);
    }
    Err("无法创建唯一的拖入任务".into())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn browser_drop_append(id: String, data_base64: String) -> Result<(), String> {
    use base64::Engine;
    use std::io::Write;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64)
        .map_err(|_| "拖入文件片段编码无效".to_string())?;
    if bytes.is_empty() || bytes.len() > 1024 * 1024 {
        return Err("拖入文件片段大小无效".into());
    }
    let path = browser_drop_file(&id)?;
    let current = fs::metadata(&path)
        .map_err(|error| error.to_string())?
        .len();
    if current + bytes.len() as u64 > 1024 * 1024 * 1024 {
        return Err("拖入文件超过 1 GB".into());
    }
    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut file| file.write_all(&bytes))
        .map_err(|error| format!("写入拖入文件失败：{}", error))
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn browser_drop_finish(id: String, size: u64) -> Result<String, String> {
    let path = browser_drop_file(&id)?;
    if fs::metadata(&path)
        .map_err(|error| error.to_string())?
        .len()
        != size
    {
        return Err("拖入文件传输不完整".into());
    }
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn browser_drop_cancel(id: String) -> Result<(), String> {
    let dir = browser_drop_dir(&id)?;
    if dir.is_dir() {
        fs::remove_dir_all(dir).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn get_data_dir() -> String {
    data_dir().to_string_lossy().to_string()
}

/// 兜底方案：协议不可用时直接返回 data URL，保证 PNG/JPG/WebP 一定能显示
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn file_base64(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    let data = fs::read(p).map_err(|e| e.to_string())?;
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut enc = base64_encode(&data);
    enc.shrink_to_fit();
    Ok(format!("data:{};base64,{}", mime_of(&ext), enc))
}

pub(crate) fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    let mut i = 0;
    while i + 2 < data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(T[(n >> 6) as usize & 63] as char);
        out.push(T[n as usize & 63] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = (data[i] as u32) << 16;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(T[(n >> 6) as usize & 63] as char);
        out.push('=');
    }
    out
}

/* ============ 本地图片协议（PNG/JPG/WebP 预览） ============ */

pub(crate) fn mime_of(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }
}

pub(crate) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hexval(bytes[i + 1]), hexval(bytes[i + 2])) {
                    out.push(h * 16 + l);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

pub(crate) fn hexval(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

pub(crate) const PLACEHOLDER_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
    0x42, 0x60, 0x82,
];
