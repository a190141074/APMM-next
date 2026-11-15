//! 被动目录同步只读取游戏文件；候选变动与实际应用分别计算，预设保持历史快照。
use crate::candidate_management::*;
use crate::directory_files::*;
use crate::management_types::*;
use crate::*;
use std::collections::BTreeMap;

#[derive(Debug, Default, Serialize)]
pub(crate) struct SyncResult {
    pub changed: bool,
    pub errors: Vec<String>,
}

/// 调用者持有库写入锁。每次事件触发都重算内容哈希，不使用秒级扫描缓存。
pub(crate) fn synchronize(
    lib: &mut Library,
    vault: &Path,
    game: &Path,
) -> Result<SyncResult, String> {
    synchronize_with(lib, vault, game, save_library)
}
pub(crate) fn synchronize_with(
    lib: &mut Library,
    vault: &Path,
    game: &Path,
    commit: impl Fn(&Library) -> Result<(), String>,
) -> Result<SyncResult, String> {
    synchronize_controlled(lib, vault, game, commit, || Ok(()))
}
pub(crate) fn synchronize_controlled(
    lib: &mut Library,
    vault: &Path,
    game: &Path,
    commit: impl Fn(&Library) -> Result<(), String>,
    checkpoint: impl Fn() -> Result<(), String>,
) -> Result<SyncResult, String> {
    let before = serde_json::to_vec(lib).map_err(|e| e.to_string())?;
    let mut result = SyncResult::default();
    result.errors.extend(crate::vault_sync::synchronize(
        lib,
        vault,
        &commit,
        &checkpoint,
    )?);
    if game.is_dir() {
        for (_, map) in [("heroes", &mut lib.heroes), ("monsters", &mut lib.monsters)] {
            for h in map.values_mut() {
                for s in h.sections.values_mut() {
                    for r in s.resources.values_mut() {
                        r.external_present = false;
                    }
                }
            }
        }
        let paths = images_under(game)?;
        let names = paths
            .iter()
            .map(|p| scan_relative_key(game, p))
            .collect::<HashSet<_>>();
        let old_bindings = lib.external_bindings.clone();
        let mut observed: BTreeMap<ResourceSlotKey, Vec<(String, bool)>> = BTreeMap::new();
        for path in paths {
            checkpoint()?;
            let (name, disabled) = image_name(&path).unwrap();
            let relative = scan_relative_key(game, &path);
            if let Err(e) = image_import::validate_png(&path) {
                result.errors.push(format!("{}：{e}", path.display()));
                continue;
            }
            let hash = sha256_file(&path)?;
            let parsed_slot = parse_filename(&name).map(|p| ResourceSlot::from_parsed(&p));
            let binding = old_bindings
                .get(&relative)
                .or_else(|| old_bindings.get(&name))
                .filter(|b| b.hash == hash);
            let slot = binding
                .map(|b| b.slot.clone())
                .or_else(|| parsed_slot.clone())
                .unwrap_or_else(|| temporary(&name, &hash));
            let old_file = candidates(lib)
                .into_iter()
                .find(|(s, f, _)| *s == slot && f.hash == hash)
                .map(|(_, f, _)| f);
            if old_file.is_none() {
                let inherited = old_bindings
                    .iter()
                    .filter(|(n, b)| !names.contains(*n) && b.hash == hash)
                    .collect::<Vec<_>>();
                let info = if inherited.len() == 1 {
                    let b = inherited[0].1;
                    candidates(lib)
                        .into_iter()
                        .find(|(s, f, _)| *s == b.slot && f.hash == hash)
                        .map(|(_, f, _)| f.info)
                        .unwrap_or_default()
                } else {
                    CandidateInfo::default()
                };
                let (dest, rel) = place_in_vault(
                    vault,
                    &slot.category,
                    &slot.hero_id,
                    &slot.section_key,
                    &name,
                    &hash,
                    ".png",
                )?;
                if !dest.exists() {
                    fs::copy(&path, &dest).map_err(|e| e.to_string())?;
                }
                if sha256_file(&dest)? != hash {
                    return Err("游戏文件在同步时发生变化，等待下一次稳定写入".into());
                }
                put_candidate(
                    lib,
                    &slot,
                    FileEntry {
                        fs_identity: crate::directory_files::file_identity(&dest)
                            .unwrap_or_default(),
                        hash: hash.clone(),
                        ext: ".png".into(),
                        source_name: name.clone(),
                        rel_path: rel,
                        added_at: unix_now(),
                        info,
                        classification: ClassificationRecord {
                            original_name: name.clone(),
                            ..Default::default()
                        },
                    },
                );
            }
            if slot.section_key == "temporary" {
                checkpoint()?;
                if let Err(e) = sync_temporary_disabled(lib, vault, &slot, &hash, disabled, &commit)
                {
                    result.errors.push(e);
                }
            }
            lib.external_bindings.insert(
                relative,
                ExternalBinding {
                    hash: hash.clone(),
                    slot: slot.clone(),
                },
            );
            if let Some(actual) = parsed_slot {
                // 手动归属与文件规则不同仅产生提示，不能据此把库候选应用到目标槽位。
                if actual == slot {
                    observed
                        .entry(ResourceSlotKey::from(&actual))
                        .or_default()
                        .push((hash, disabled));
                }
            } else {
                get_resource(
                    lib,
                    &slot.category,
                    &slot.hero_id,
                    &slot.section_key,
                    &slot.canonical,
                )
                .external_present = true;
            }
        }
        for (category, map) in [("heroes", &mut lib.heroes), ("monsters", &mut lib.monsters)] {
            for (id, h) in map {
                for (section, s) in &mut h.sections {
                    for (cn, r) in &mut s.resources {
                        if section == "temporary" {
                            r.active_hash = None;
                            continue;
                        }
                        let key = ResourceSlotKey(
                            category_of(section),
                            id.clone(),
                            section.clone(),
                            cn.clone(),
                        );
                        match observed.get(&key).map(Vec::as_slice) {
                            Some([(hash, disabled)]) => {
                                r.active_hash = Some(hash.clone());
                                r.disabled = *disabled;
                            }
                            _ => {
                                r.active_hash = None;
                                r.disabled = false;
                            }
                        }
                        let _ = category;
                    }
                }
            }
        }
    }
    result.changed = before != serde_json::to_vec(lib).map_err(|e| e.to_string())?;
    Ok(result)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ResourceSlotKey(String, String, String, String);
impl From<&ResourceSlot> for ResourceSlotKey {
    fn from(s: &ResourceSlot) -> Self {
        Self(
            s.category.clone(),
            s.hero_id.clone(),
            s.section_key.clone(),
            s.canonical.clone(),
        )
    }
}
