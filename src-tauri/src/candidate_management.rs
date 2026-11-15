//! 候选信息和归属事务；只移动资源库文件，不应用或改写游戏槽位，不修改预设。
use crate::management_types::*;
use crate::*;

pub(crate) fn file_mut<'a>(
    lib: &'a mut Library,
    slot: &ResourceSlot,
    hash: &str,
) -> Result<&'a mut FileEntry, String> {
    get_resource(
        lib,
        &slot.category,
        &slot.hero_id,
        &slot.section_key,
        &slot.canonical,
    )
    .files
    .iter_mut()
    .find(|f| f.hash == hash)
    .ok_or_else(|| "候选不存在".into())
}
fn slot_from(args: &ActiveArgs) -> ResourceSlot {
    ResourceSlot {
        category: category_of(&args.section_key),
        hero_id: args.hero_id.clone(),
        section_key: args.section_key.clone(),
        canonical: args.canonical.clone(),
    }
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn candidate_info_set(
    args: ActiveArgs,
    mut info: CandidateInfo,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    for links in [
        &mut info.provenance.author_urls,
        &mut info.provenance.artist_urls,
    ] {
        links.retain(|url| !url.trim().is_empty());
        for url in links {
            *url = url.trim().to_string();
        }
    }
    info.provenance.source_url = info.provenance.source_url.trim().to_string();
    for link in std::iter::once(&info.provenance.source_url)
        .chain(&info.provenance.author_urls)
        .chain(&info.provenance.artist_urls)
        .filter(|link| !link.trim().is_empty())
    {
        let url = reqwest::Url::parse(link).map_err(|_| "链接无效")?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("链接须为 HTTPS 网址".into());
        }
    }
    if [
        info.provenance.author.len(),
        info.provenance.artist.len(),
        info.provenance.source_url.len(),
        info.provenance.version.len(),
        info.note.len(),
    ]
    .iter()
    .any(|n| *n > 16_384)
        || info
            .provenance
            .author_urls
            .iter()
            .chain(&info.provenance.artist_urls)
            .any(|url| url.len() > 16_384)
    {
        return Err("信息内容过长".into());
    }
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    file_mut(
        &mut lib,
        &slot_from(&args),
        args.hash.as_deref().ok_or("请选择候选")?,
    )?
    .info = info;
    save_library(&lib)
}
/// 名称只作为展示信息保存，不受文件名字符规则限制，也不移动 PNG 或改写内容哈希。
pub(crate) fn rename_display(
    lib: &mut Library,
    slot: &ResourceSlot,
    hash: &str,
    name: String,
) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().any(char::is_control) {
        return Err("请输入有效的候选名称".into());
    }
    let file = file_mut(lib, slot, hash)?;
    if file.classification.original_name.is_empty() {
        file.classification.original_name = file.source_name.clone();
    }
    file.source_name = name.clone();
    Ok(name)
}
pub(crate) fn rule_key(slot: &ResourceSlot) -> String {
    format!(
        "{}/{}/{}/{}",
        slot.category, slot.hero_id, slot.section_key, slot.canonical
    )
}
pub(crate) fn check_rule(slot: &ResourceSlot, file: &mut FileEntry) {
    let name = if file.classification.original_name.is_empty() {
        &file.source_name
    } else {
        &file.classification.original_name
    };
    let rule = parse_filename(name).map(|p| ResourceSlot::from_parsed(&p));
    file.classification.pending = rule.filter(|r| {
        file.classification.manual
            && r != slot
            && file.classification.confirmed_rule.as_deref() != Some(rule_key(r).as_str())
    });
}

/// 游戏临时文件的启禁变化只同步库副本；未变化的外部状态不撤销用户的库内选择。
pub(crate) fn sync_temporary_disabled(
    lib: &mut Library,
    vault: &Path,
    slot: &ResourceSlot,
    hash: &str,
    disabled: bool,
    commit: impl FnOnce(&Library) -> Result<(), String>,
) -> Result<(), String> {
    let file = file_mut(lib, slot, hash)?.clone();
    if file.classification.external_disabled == Some(disabled) {
        return Ok(());
    }
    let before = lib.clone();
    let source = vault_path(vault, &file);
    let local_disabled = source.to_string_lossy().ends_with(".disabled");
    let keep_local = file.classification.external_disabled.is_none() && !disabled && local_disabled;
    let target = if local_disabled == disabled || keep_local {
        source.clone()
    } else if disabled {
        PathBuf::from(format!("{}.disabled", source.to_string_lossy()))
    } else {
        source.with_file_name(
            source
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".disabled"))
                .ok_or("临时文件禁用后缀无效")?,
        )
    };
    if source != target {
        if target.exists() {
            return Err("临时启禁目标文件已存在".into());
        }
        fs::rename(&source, &target).map_err(|e| e.to_string())?;
    }
    let updated = file_mut(lib, slot, hash)?;
    updated.rel_path = target
        .strip_prefix(vault)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .replace('\\', "/");
    updated.classification.external_disabled = Some(disabled);
    get_resource(
        lib,
        &slot.category,
        &slot.hero_id,
        &slot.section_key,
        &slot.canonical,
    )
    .disabled = if keep_local { true } else { disabled };
    if let Err(e) = commit(lib) {
        if source != target {
            fs::rename(&target, &source).map_err(|r| {
                format!(
                    "{e}；临时启禁回滚失败：{r}；文件保留在 {}",
                    target.display()
                )
            })?;
        }
        *lib = before;
        return Err(e);
    }
    Ok(())
}
/// 目标已存在时补空信息；索引提交失败时回滚两个库文件。预设哈希和历史名称保持不变。
pub(crate) fn move_candidate(
    lib: &mut Library,
    vault: &Path,
    from: &ResourceSlot,
    hash: &str,
    to: &ResourceSlot,
    manual: bool,
    commit: impl FnOnce(&Library) -> Result<(), String>,
) -> Result<(), String> {
    if !to.valid() {
        return Err("请选择现有有效槽位".into());
    }
    if from.section_key == "temporary"
        && get_resource(
            lib,
            &from.category,
            &from.hero_id,
            &from.section_key,
            &from.canonical,
        )
        .disabled
    {
        return Err("请先启用临时候选再归类".into());
    }
    let before = lib.clone();
    let mut entry = file_mut(lib, from, hash)?.clone();
    let old = vault_path(vault, &entry)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let root = vault.canonicalize().map_err(|e| e.to_string())?;
    if !old.starts_with(&root) || sha256_file(&old)? != hash {
        return Err("候选路径或内容校验失败".into());
    }
    image_import::validate_png(&old)?;
    entry.classification.manual = manual;
    if entry.classification.original_name.is_empty() {
        entry.classification.original_name = entry.source_name.clone();
    }
    entry.classification.confirmed_rule = parse_filename(&entry.classification.original_name)
        .map(|p| rule_key(&ResourceSlot::from_parsed(&p)));
    entry.classification.pending = None;
    if from == to {
        *file_mut(lib, from, hash)? = entry;
        if let Err(e) = commit(lib) {
            *lib = before;
            return Err(e);
        }
        return Ok(());
    }
    let duplicate = get_resource(
        lib,
        &to.category,
        &to.hero_id,
        &to.section_key,
        &to.canonical,
    )
    .files
    .iter()
    .find(|f| f.hash == hash)
    .cloned();
    let (dest, rel) = if let Some(f) = &duplicate {
        (vault_path(vault, f), f.rel_path.clone())
    } else {
        place_in_vault(
            vault,
            &to.category,
            &to.hero_id,
            &to.section_key,
            &entry.source_name,
            hash,
            &entry.ext,
        )?
    };
    let stage = staging::create("classify")?;
    let backup = stage.join("source.png");
    let mut created = false;
    let mut source_removed = false;
    let result = (|| {
        if duplicate.is_some() && sha256_file(&dest)? != hash {
            return Err("目标候选损坏，请先修复".into());
        }
        if old != dest {
            // 暂存位于工作盘，资源库可能在另一卷；校验复制后再移除源，不能跨盘 rename。
            staging::copy_new(&old, &backup)?;
            if sha256_file(&backup)? != hash || sha256_file(&old)? != hash {
                return Err("候选在归类前发生变化".into());
            }
            fs::remove_file(&old).map_err(|e| e.to_string())?;
            source_removed = true;
            if duplicate.is_none() {
                if dest.exists() {
                    return Err("目标文件已存在，请重新同步".into());
                }
                staging::copy_new(&backup, &dest)?;
                created = true;
            }
        }
        get_resource(
            lib,
            &from.category,
            &from.hero_id,
            &from.section_key,
            &from.canonical,
        )
        .files
        .retain(|f| f.hash != hash);
        if let Some(target) = get_resource(
            lib,
            &to.category,
            &to.hero_id,
            &to.section_key,
            &to.canonical,
        )
        .files
        .iter_mut()
        .find(|f| f.hash == hash)
        {
            target.info.fill_empty(&entry.info);
            target.classification = entry.classification.clone();
        } else {
            entry.rel_path = rel;
            entry.fs_identity = crate::directory_files::file_identity(&dest).unwrap_or_default();
            get_resource(
                lib,
                &to.category,
                &to.hero_id,
                &to.section_key,
                &to.canonical,
            )
            .files
            .push(entry);
        }
        for binding in lib.external_bindings.values_mut() {
            if binding.hash == hash && binding.slot == *from {
                binding.slot = to.clone();
            }
        }
        // 未识别游戏文件的原始名称绑定归属，重复扫描只同步存在状态，不将归属误判为应用。
        if from.section_key == "temporary" {
            let name = file_mut(lib, to, hash)?
                .classification
                .original_name
                .clone();
            lib.external_bindings.insert(
                name,
                ExternalBinding {
                    hash: hash.into(),
                    slot: to.clone(),
                },
            );
        }
        commit(lib)
    })();
    if result.is_err() {
        if created {
            let _ = fs::remove_file(&dest);
        }
        if source_removed {
            staging::copy_new(&backup, &old)
                .map_err(|e| format!("归类回滚失败：{e}；备份 {}", backup.display()))?;
        }
        *lib = before;
    }
    let _ = staging::remove(stage);
    result
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn candidate_classify(
    args: ActiveArgs,
    target: ResourceSlot,
    keep_current: bool,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    let from = slot_from(&args);
    let hash = args.hash.as_deref().ok_or("请选择候选")?;
    if keep_current {
        let file = file_mut(&mut lib, &from, hash)?;
        file.classification.confirmed_rule = file.classification.pending.as_ref().map(rule_key);
        file.classification.pending = None;
        save_library(&lib)
    } else {
        move_candidate(
            &mut lib,
            &vault_dir(),
            &from,
            hash,
            &target,
            true,
            save_library,
        )?;
        let game = PathBuf::from(&lib.settings.modimg_path);
        crate::directory_sync::synchronize(&mut lib, &vault_dir(), &game)?;
        save_library(&lib)
    }
}

pub(crate) fn candidates(lib: &Library) -> Vec<(ResourceSlot, FileEntry, bool)> {
    let mut result = Vec::new();
    for (category, map) in [("heroes", &lib.heroes), ("monsters", &lib.monsters)] {
        for (id, h) in map {
            for (section, s) in &h.sections {
                for (canonical, r) in &s.resources {
                    for f in &r.files {
                        result.push((
                            ResourceSlot {
                                category: if section == "temporary" {
                                    "heroes".into()
                                } else {
                                    category_of(section)
                                },
                                hero_id: id.clone(),
                                section_key: section.clone(),
                                canonical: canonical.clone(),
                            },
                            f.clone(),
                            r.disabled,
                        ));
                    }
                }
            }
        }
        let _ = category;
    }
    result.sort_by(|a, b| (&a.0.canonical, &a.1.hash).cmp(&(&b.0.canonical, &b.1.hash)));
    result
}
pub(crate) fn temporary(_name: &str, hash: &str) -> ResourceSlot {
    ResourceSlot {
        category: "heroes".into(),
        hero_id: format!("temporary:{hash}"),
        section_key: "temporary".into(),
        canonical: format!("unmatched:{hash}"),
    }
}
pub(crate) fn remove_candidate(lib: &mut Library, slot: &ResourceSlot, hash: &str) {
    get_resource(
        lib,
        &slot.category,
        &slot.hero_id,
        &slot.section_key,
        &slot.canonical,
    )
    .files
    .retain(|f| f.hash != hash);
}
pub(crate) fn put_candidate(lib: &mut Library, slot: &ResourceSlot, file: FileEntry) {
    let r = get_resource(
        lib,
        &slot.category,
        &slot.hero_id,
        &slot.section_key,
        &slot.canonical,
    );
    if let Some(existing) = r.files.iter_mut().find(|f| f.hash == file.hash) {
        existing.info.fill_empty(&file.info);
    } else {
        r.files.push(file);
    }
}
