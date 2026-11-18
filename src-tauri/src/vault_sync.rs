//! 资源库外部编辑与规则迁移保持候选信息，文件归类通过事务提交。
use crate::candidate_management::*;
use crate::directory_files::*;
use crate::management_types::*;
use crate::*;
use std::collections::BTreeMap;
pub(crate) fn synchronize(
    lib: &mut Library,
    vault: &Path,
    commit: impl Fn(&Library) -> Result<(), String>,
    checkpoint: impl Fn() -> Result<(), String>,
) -> Result<Vec<String>, String> {
    let observation = crate::directory_observation::full(vault, Path::new(""), &checkpoint)?;
    synchronize_observed(lib, vault, observation.root(vault)?, commit, checkpoint)
}

/// 失败文件仍保留在路径集合中，防止暂时无法读取时删除库记录。
pub(crate) fn synchronize_observed(
    lib: &mut Library,
    vault: &Path,
    observation: &crate::directory_observation::RootObservation,
    commit: impl Fn(&Library) -> Result<(), String>,
    checkpoint: impl Fn() -> Result<(), String>,
) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    let paths = observation.files.keys().cloned().collect::<Vec<_>>();
    let mut hashes = BTreeMap::new();
    for p in &paths {
        checkpoint()?;
        match &observation.files[p] {
            Ok(value) => {
                hashes.insert(p.clone(), value.hash.clone());
            }
            Err(e) => errors.push(format!("{}：{e}", p.display())),
        }
    }
    let old = candidates(lib);
    let identities = paths
        .iter()
        .filter_map(|p| {
            observation.files[p]
                .as_ref()
                .ok()
                .map(|v| (p.clone(), v.stamp.identity.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let known_paths = old
        .iter()
        .map(|(_, f, _)| vault_path(vault, f))
        .collect::<HashSet<_>>();
    let missing = old
        .iter()
        .filter(|(_, f, _)| !observation.files.contains_key(&vault_path(vault, f)))
        .map(|(_, f, _)| f.hash.clone())
        .collect::<Vec<_>>();
    let mut used = HashSet::new();
    for (slot, mut file, disabled) in old {
        checkpoint()?;
        let original_path = vault_path(vault, &file);
        let mut path = observation
            .files
            .contains_key(&original_path)
            .then(|| original_path.clone());
        if path.is_none() {
            if !file.fs_identity.is_empty() {
                path = identities
                    .iter()
                    .find(|(p, id)| !used.contains(*p) && *id == &file.fs_identity)
                    .map(|(p, _)| p.clone());
            }
        }
        if path.is_none() {
            let same_folder = hashes
                .iter()
                .filter(|(p, h)| {
                    !used.contains(*p) && *h == &file.hash && p.parent() == original_path.parent()
                })
                .map(|(p, _)| p.clone())
                .collect::<Vec<_>>();
            if same_folder.len() == 1 {
                path = same_folder.first().cloned();
            }
        }
        if path.is_none() && missing.iter().filter(|h| *h == &file.hash).count() == 1 {
            let matches = hashes
                .iter()
                .filter(|(p, h)| {
                    !used.contains(*p) && !known_paths.contains(*p) && *h == &file.hash
                })
                .map(|(p, _)| p.clone())
                .collect::<Vec<_>>();
            if matches.len() == 1 {
                path = matches.first().cloned();
            }
        }
        let Some(path) = path else {
            remove_candidate(lib, &slot, &file.hash);
            continue;
        };
        used.insert(path.clone());
        if observation.files[&path].is_err() {
            continue;
        }
        let hash = hashes[&path].clone();
        file.fs_identity = identities.get(&path).cloned().unwrap_or_default();
        remove_candidate(lib, &slot, &file.hash);
        if file.hash != hash {
            for binding in lib.external_bindings.values_mut() {
                if binding.hash == file.hash && binding.slot == slot {
                    binding.hash = hash.clone();
                }
            }
            file.hash = hash;
        }
        if path != original_path {
            file.rel_path = path
                .strip_prefix(vault)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            file.source_name = image_name(&path).unwrap().0;
            file.classification.original_name = file.source_name.clone();
        }
        candidate_management::check_rule(&slot, &mut file);
        let rule = parse_filename(if file.classification.original_name.is_empty() {
            &file.source_name
        } else {
            &file.classification.original_name
        })
        .map(|p| ResourceSlot::from_parsed(&p));
        put_candidate(lib, &slot, file.clone());
        if slot.section_key == "temporary" {
            get_resource(
                lib,
                &slot.category,
                &slot.hero_id,
                &slot.section_key,
                &slot.canonical,
            )
            .disabled = image_name(&path).unwrap().1;
            if !disabled && !image_name(&path).unwrap().1 && !file.classification.manual {
                if let Some(target) = rule {
                    if let Err(e) = candidate_management::move_candidate(
                        lib, vault, &slot, &file.hash, &target, false, &commit,
                    ) {
                        errors.push(e);
                    }
                }
            }
        }
    }
    for path in paths.into_iter().filter(|p| !used.contains(p)) {
        checkpoint()?;
        if observation.files[&path].is_err() {
            continue;
        }
        let Some(hash) = hashes.get(&path) else {
            continue;
        };
        let (name, disabled) = image_name(&path).unwrap();
        let slot = parse_filename(&name)
            .map(|p| ResourceSlot::from_parsed(&p))
            .unwrap_or_else(|| temporary(&name, hash));
        let file = FileEntry {
            fs_identity: file_identity(&path).unwrap_or_default(),
            hash: hash.clone(),
            source_name: name.clone(),
            rel_path: path
                .strip_prefix(vault)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/"),
            ext: ".png".into(),
            added_at: unix_now(),
            classification: ClassificationRecord {
                original_name: name,
                ..Default::default()
            },
            ..Default::default()
        };
        put_candidate(lib, &slot, file);
        if slot.section_key == "temporary" {
            get_resource(
                lib,
                &slot.category,
                &slot.hero_id,
                &slot.section_key,
                &slot.canonical,
            )
            .disabled = disabled;
        }
    }
    Ok(errors)
}
