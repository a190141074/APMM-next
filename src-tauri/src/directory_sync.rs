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

/// 按完整槽位和摘要读取当前候选，保留旧库跨 map 的查找顺序与独立信息。
/// 扫描每个游戏文件时不能复制、排序整个资源库；新增候选也必须立即可见。
fn matching_candidate<'a>(
    lib: &'a Library,
    slot: &ResourceSlot,
    hash: &str,
) -> Option<&'a FileEntry> {
    let category = if slot.section_key == "temporary" {
        "heroes".to_string()
    } else {
        category_of(&slot.section_key)
    };
    if category != slot.category {
        return None;
    }
    [&lib.heroes, &lib.monsters].into_iter().find_map(|map| {
        map.get(&slot.hero_id)?
            .sections
            .get(&slot.section_key)?
            .resources
            .get(&slot.canonical)?
            .files
            .iter()
            .find(|file| file.hash == hash)
    })
}

/// 兼容入口执行强制完整校验；被动监听通过预先采集的观察结果提交。
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
    let observation = crate::directory_observation::full(vault, game, &checkpoint)?;
    synchronize_observed(lib, vault, game, &observation, commit, checkpoint)
}

/// 调用者持有库写入锁；观察已在锁外完成，仍保留候选归属和文件事务。
pub(crate) fn synchronize_observed(
    lib: &mut Library,
    vault: &Path,
    game: &Path,
    observation: &crate::directory_observation::Observation,
    commit: impl Fn(&Library) -> Result<(), String>,
    checkpoint: impl Fn() -> Result<(), String>,
) -> Result<SyncResult, String> {
    checkpoint()?;
    let before = serde_json::to_value(&*lib).map_err(|e| e.to_string())?;
    let mut result = SyncResult::default();
    result
        .errors
        .extend(crate::vault_sync::synchronize_observed(
            lib,
            vault,
            observation.root(vault)?,
            &commit,
            &checkpoint,
        )?);
    if let Some(game_observation) = observation.roots.get(game) {
        // 本轮游戏文件不完整时保留上次应用状态，不能把坏图当成未应用。
        let game_errors = game_observation
            .files
            .values()
            .filter_map(|v| v.as_ref().err().cloned())
            .collect::<Vec<_>>();
        if !game_errors.is_empty() {
            result.errors.extend(game_errors);
            result.changed = before != serde_json::to_value(&*lib).map_err(|e| e.to_string())?;
            return Ok(result);
        }
        for (_, map) in [("heroes", &mut lib.heroes), ("monsters", &mut lib.monsters)] {
            for h in map.values_mut() {
                for s in h.sections.values_mut() {
                    for r in s.resources.values_mut() {
                        r.external_present = false;
                    }
                }
            }
        }
        let paths = game_observation.files.keys().cloned().collect::<Vec<_>>();
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
            let hash = game_observation.files[&path]
                .as_ref()
                .map_err(Clone::clone)?
                .hash
                .clone();
            let parsed_slot = parse_filename(&name).map(|p| ResourceSlot::from_parsed(&p));
            let binding = old_bindings
                .get(&relative)
                .or_else(|| old_bindings.get(&name))
                .filter(|b| b.hash == hash);
            let slot = binding
                .map(|b| b.slot.clone())
                .or_else(|| parsed_slot.clone())
                .unwrap_or_else(|| temporary(&name, &hash));
            let old_file = matching_candidate(lib, &slot, &hash);
            if old_file.is_none() {
                let inherited = old_bindings
                    .iter()
                    .filter(|(n, b)| !names.contains(*n) && b.hash == hash)
                    .collect::<Vec<_>>();
                let info = if inherited.len() == 1 {
                    let b = inherited[0].1;
                    matching_candidate(lib, &b.slot, &hash)
                        .map(|file| file.info.clone())
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
    result.changed = before != serde_json::to_value(lib).map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_lookup_keeps_same_hash_slots_and_observes_new_candidates() {
        let mut lib = Library::default();
        let mut slot = ResourceSlot {
            category: category_of("cursor"),
            hero_id: "cursor".into(),
            section_key: "cursor".into(),
            canonical: "cursor_up".into(),
        };
        for (name, note) in [
            ("cursor_up", "抬起独立信息"),
            ("cursor_down", "按下独立信息"),
        ] {
            slot.canonical = name.into();
            put_candidate(
                &mut lib,
                &slot,
                FileEntry {
                    hash: "same-content".into(),
                    info: CandidateInfo {
                        note: note.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
        }
        for name in ["cursor_up", "cursor_down"] {
            slot.canonical = name.into();
            let expected = candidates(&lib)
                .into_iter()
                .find(|(candidate, file, _)| candidate == &slot && file.hash == "same-content")
                .unwrap()
                .1;
            assert_eq!(
                matching_candidate(&lib, &slot, "same-content")
                    .unwrap()
                    .info,
                expected.info
            );
            assert!(matching_candidate(&lib, &slot, "not-present").is_none());
        }
        put_candidate(
            &mut lib,
            &slot,
            FileEntry {
                hash: "just-added".into(),
                ..Default::default()
            },
        );
        assert!(matching_candidate(&lib, &slot, "just-added").is_some());
        slot.category = "monsters".into();
        assert!(matching_candidate(&lib, &slot, "same-content").is_none());
    }
}
