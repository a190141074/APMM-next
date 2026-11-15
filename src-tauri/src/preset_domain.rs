//! 预设保存固定内容与作用范围；预览和提交都核对实际文件，整批应用复用资源事务。
use crate::management_types::*;
use crate::*;
use std::collections::BTreeMap;

pub(crate) fn find<'a>(lib: &'a Library, id: &str) -> Result<&'a GlobalPreset, String> {
    lib.settings
        .presets
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "预设不存在".into())
}
pub(crate) fn editable(p: &GlobalPreset) -> Result<(), String> {
    if p.locked {
        Err("预设已锁定，请先解锁".into())
    } else {
        Ok(())
    }
}
pub(crate) fn name_checked(name: String) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().count() > 120 {
        Err("预设名称须为 1–120 个字符".into())
    } else {
        Ok(name)
    }
}
fn slots(lib: &Library) -> Vec<ResourceSlot> {
    workspace::snapshot_from_library(lib)
        .resources
        .iter()
        .filter_map(|r| parse_filename(&r.canonical).map(|p| ResourceSlot::from_parsed(&p)))
        .collect()
}
fn resource<'a>(lib: &'a Library, slot: &ResourceSlot) -> Option<&'a ResourceEntry> {
    lib_map(lib, &slot.category)
        .get(&slot.hero_id)?
        .sections
        .get(&slot.section_key)?
        .resources
        .get(&slot.canonical)
}
/// 一次枚举得到实际槽位状态；禁用文件参与冲突检查，但有效搭配按原版表示。
fn observed(game: &Path) -> BTreeMap<String, Vec<(PathBuf, String, bool)>> {
    let mut result: BTreeMap<String, Vec<(PathBuf, String, bool)>> = BTreeMap::new();
    if let Ok(paths) = scanner::sorted_entries(game) {
        for path in paths {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let disabled = name.to_ascii_lowercase().ends_with(".png.disabled");
            let image_name = if disabled {
                &name[..name.len() - 9]
            } else {
                name
            };
            if !image_name.to_ascii_lowercase().ends_with(".png") {
                continue;
            }
            if let Some(parsed) = parse_filename(image_name) {
                result.entry(parsed.canonical).or_default().push((
                    path.clone(),
                    sha256_file(&path).unwrap_or_default(),
                    disabled,
                ));
            }
        }
    }
    result
}
fn item_for(slot: &ResourceSlot, hash: String, display_name: String) -> PresetItem {
    PresetItem {
        category: slot.category.clone(),
        hero_id: slot.hero_id.clone(),
        section_key: slot.section_key.clone(),
        canonical: slot.canonical.clone(),
        hash,
        display_name,
    }
}
pub(crate) fn capture(
    lib: &Library,
    scope: String,
    selected: Vec<ResourceSlot>,
    name: String,
) -> Result<GlobalPreset, String> {
    if !matches!(scope.as_str(), "global" | "local") {
        return Err("预设范围无效".into());
    }
    let selected = if scope == "global" {
        slots(lib)
    } else {
        selected
    };
    if selected.is_empty() {
        return Err("请选择预设槽位".into());
    }
    let actual = observed(Path::new(&lib.settings.modimg_path));
    let mut items = BTreeMap::new();
    for slot in selected {
        if !slot.valid() {
            return Err(format!("无效槽位：{}", slot.canonical));
        }
        let hash = match actual.get(&slot.canonical).map(Vec::as_slice) {
            Some([(_, hash, false)]) => hash.clone(),
            _ => String::new(),
        };
        let display_name = resource(lib, &slot)
            .and_then(|r| r.files.iter().find(|f| f.hash == hash))
            .map(|f| f.source_name.clone())
            .unwrap_or_else(|| slot.canonical.clone());
        items.insert(slot.canonical.clone(), item_for(&slot, hash, display_name));
    }
    Ok(GlobalPreset {
        id: uuid::Uuid::new_v4().to_string(),
        name: name_checked(name)?,
        items: items.into_values().collect(),
        scope,
        locked: false,
    })
}
/// 全局旧预设隐含范围内其余槽位恢复原版；局部预设仅使用保存项的固定集合。
pub(crate) fn preview_with(
    lib: &Library,
    preset: &GlobalPreset,
    vault: &Path,
    game: &Path,
    known: Vec<ResourceSlot>,
) -> PresetPreview {
    let mut items: BTreeMap<String, PresetItem> = BTreeMap::new();
    if preset.scope == "global" {
        for slot in known {
            items.insert(
                slot.canonical.clone(),
                item_for(&slot, String::new(), slot.canonical.clone()),
            );
        }
    }
    let mut issues = Vec::new();
    let mut seen = HashSet::new();
    for item in &preset.items {
        if !seen.insert(item.canonical.clone()) {
            issues.push(format!("重复槽位：{}", item.canonical));
        }
        items.insert(item.canonical.clone(), item.clone());
    }
    let actual = observed(game);
    let vault_root = vault.canonicalize().ok();
    let mut changes = Vec::new();
    for item in items.into_values() {
        let slot = item.slot();
        let mut row_issues = Vec::new();
        if !slot.valid() {
            row_issues.push("槽位已无效".into());
        }
        let current = actual.get(&item.canonical);
        if current.is_some_and(|v| v.len() > 1) {
            row_issues.push("游戏槽位存在多个文件，请先处理冲突".into());
        }
        let current_path = current
            .and_then(|v| v.first())
            .map(|(p, _, _)| p.to_string_lossy().into_owned());
        let current_hash = current
            .and_then(|v| v.first())
            .and_then(|(_, h, disabled)| if *disabled { None } else { Some(h.clone()) });
        let mut saved_path = None;
        if !item.hash.is_empty() {
            match resource(lib, &slot)
                .and_then(|r| r.files.iter().find(|f| f.hash == item.hash).map(|f| (r, f)))
            {
                None => row_issues.push("保存的候选已缺失或移动".into()),
                Some((r, file)) => {
                    let path = vault_path(vault, file);
                    saved_path = Some(path.to_string_lossy().into_owned());
                    if r.disabled || path.to_string_lossy().ends_with(".disabled") {
                        row_issues.push("候选已禁用".into());
                    }
                    match path.canonicalize() {
                        Ok(p) if vault_root.as_ref().is_some_and(|root| p.starts_with(root)) => {
                            if sha256_file(&p).ok().as_deref() != Some(item.hash.as_str())
                                || crate::image_import::validate_png(&p).is_err()
                            {
                                row_issues.push("候选内容已损坏或被修改".into());
                            }
                        }
                        _ => row_issues.push("候选文件缺失或路径无效".into()),
                    }
                }
            }
        }
        let desired = if item.hash.is_empty() {
            None
        } else {
            Some(item.hash.clone())
        };
        // 清除禁用替换文件同样属于变更，不依赖原图提取状态。
        let changed = current_hash != desired || (item.hash.is_empty() && current.is_some());
        issues.extend(
            row_issues
                .iter()
                .map(|e| format!("{}：{e}", item.display_name)),
        );
        changes.push(PresetChange {
            item,
            saved_path,
            current_path,
            current_hash,
            changed,
            issues: row_issues,
        });
    }
    if lib.settings.modimg_path.is_empty() || !game.is_dir() {
        issues.push("请先连接有效游戏 Mod 目录".into());
    }
    if preset.scope != "global" && preset.scope != "local" {
        issues.push("预设范围无效".into());
    }
    PresetPreview {
        preset: preset.clone(),
        changes,
        can_apply: issues.is_empty(),
        issues,
    }
}
pub(crate) fn preview(lib: &Library, preset: &GlobalPreset) -> PresetPreview {
    preview_with(
        lib,
        preset,
        &vault_dir(),
        Path::new(&lib.settings.modimg_path),
        slots(lib),
    )
}
pub(crate) fn apply(lib: &mut Library, p: &GlobalPreset) -> Result<(), String> {
    let known = slots(lib);
    let game = PathBuf::from(&lib.settings.modimg_path);
    apply_with(lib, p, &vault_dir(), &game, known, save_library)
}
/// 先固定候选字节，再重新核对作用范围和游戏状态；失败时回滚文件及内存索引。
pub(crate) fn apply_with(
    lib: &mut Library,
    p: &GlobalPreset,
    vault: &Path,
    game: &Path,
    known: Vec<ResourceSlot>,
    commit: impl FnOnce(&Library) -> Result<(), String>,
) -> Result<(), String> {
    let preview = preview_with(lib, p, vault, game, known.clone());
    if !preview.can_apply {
        return Err(preview.issues.join("\n"));
    }
    let before = lib.clone();
    let stage = staging::create("preset-sources")?;
    let result = (|| {
        let mut sources = HashMap::new();
        for (index, row) in preview.changes.iter().enumerate() {
            let source = if let Some(path) = &row.saved_path {
                let dest = stage.join(format!("{index}.png"));
                fs::copy(path, &dest).map_err(|e| e.to_string())?;
                if sha256_file(&dest)? != row.item.hash {
                    return Err("候选在提交前被修改，请重新预览".into());
                }
                Some(dest)
            } else {
                None
            };
            sources.insert(row.item.canonical.clone(), source);
        }
        let fresh = preview_with(lib, p, vault, game, known);
        if !fresh.can_apply {
            return Err(fresh.issues.join("\n"));
        }
        let mut plan = Vec::new();
        for row in fresh.changes {
            if row.changed {
                plan.push((
                    row.item.canonical.clone(),
                    sources
                        .remove(&row.item.canonical)
                        .ok_or("预设范围在提交前变化")?,
                ));
            }
            let slot = row.item.slot();
            let r = get_resource(
                lib,
                &slot.category,
                &slot.hero_id,
                &slot.section_key,
                &slot.canonical,
            );
            r.active_hash = if row.item.hash.is_empty() {
                None
            } else {
                Some(row.item.hash)
            };
            r.disabled = false;
        }
        resource_transaction::apply_batch(game, &plan, || commit(lib))
    })();
    if result.is_err() {
        *lib = before;
    }
    let _ = staging::remove(stage);
    result
}
