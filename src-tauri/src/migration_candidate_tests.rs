//! 迁移验收覆盖多个同名候选、非空库补齐、取消、校验失败和真实旧库的只读来源。
use super::*;
use crate::management_tests::png;

const SLOT: &str = "UT_Hero_Card_102";

fn library(path: &Path) -> Library {
    serde_json::from_slice(&fs::read(path.join("library.json")).unwrap()).unwrap()
}

fn save(path: &Path, value: &Library) {
    ensure_dir(path).unwrap();
    fs::write(
        path.join("library.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}

fn entries(lib: &Library) -> Vec<(&str, &str, &str, bool, &ResourceEntry)> {
    let mut result = Vec::new();
    for (monster, map) in [(false, &lib.heroes), (true, &lib.monsters)] {
        for (id, entity) in map {
            for (section, value) in &entity.sections {
                for (canonical, resource) in &value.resources {
                    result.push((
                        id.as_str(),
                        section.as_str(),
                        canonical.as_str(),
                        monster,
                        resource,
                    ));
                }
            }
        }
    }
    result
}

fn fixture() -> (PathBuf, PathBuf, PathBuf) {
    let root = crate::staging::create("migration-candidates-test").unwrap();
    let source = root.join("old");
    let destination = root.join("new");
    let mut old = Library::default();
    for (index, relative) in ["one.png", "heroes/102/card/two.png"].iter().enumerate() {
        let file = source.join("vault").join(relative);
        ensure_dir(file.parent().unwrap()).unwrap();
        png(&file, 30 + index as u8);
        let hash = sha256_file(&file).unwrap();
        get_resource(&mut old, "heroes", "102", "card", SLOT)
            .files
            .push(FileEntry {
                hash,
                ext: ".png".into(),
                source_name: format!("{SLOT}.png"),
                rel_path: relative.to_string(),
                ..Default::default()
            });
    }
    let resource = get_resource(&mut old, "heroes", "102", "card", SLOT);
    resource.active_hash = Some(resource.files[1].hash.clone());
    save(&source, &old);
    let mut current = old.clone();
    let resource = get_resource(&mut current, "heroes", "102", "card", SLOT);
    resource.files.truncate(1);
    resource.active_hash = Some(resource.files[0].hash.clone());
    resource.files[0].info.note = "保留新版备注".into();
    current.settings.update_auto_check = false;
    let copied = destination.join("vault/one.png");
    ensure_dir(copied.parent().unwrap()).unwrap();
    fs::copy(source.join("vault/one.png"), copied).unwrap();
    save(&destination, &current);
    (root, source, destination)
}

#[test]
fn nonempty_migration_keeps_all_candidates_and_current_application() {
    let (root, source, destination) = fixture();
    let before = library(&destination);
    let old_hash = sha256_file(&source.join("library.json")).unwrap();
    let result = copy_to(&source, &destination, &AtomicBool::new(false), |_, _, _| {}).unwrap();
    assert_eq!(result["mode"], "merge");
    assert_eq!(result["modsAdded"], 1);
    let mut current = library(&destination);
    let slot = &current.heroes["102"].sections["card"].resources[SLOT];
    assert_eq!(slot.files.len(), 2);
    assert_eq!(
        slot.active_hash,
        before.heroes["102"].sections["card"].resources[SLOT].active_hash
    );
    assert_eq!(slot.files[0].info.note, "保留新版备注");
    assert_eq!(
        serde_json::to_value(&current.settings).unwrap(),
        serde_json::to_value(&before.settings).unwrap()
    );
    assert_ne!(slot.files[0].rel_path, slot.files[1].rel_path);
    for file in &slot.files {
        assert_eq!(
            sha256_file(&vault_path(&destination.join("vault"), file)).unwrap(),
            file.hash
        );
    }
    // 重新扫描和重复迁移都保留两个同名候选，只允许一个应用记录。
    crate::vault_sync::synchronize(
        &mut current,
        &destination.join("vault"),
        |_| Ok(()),
        || Ok(()),
    )
    .unwrap();
    assert_eq!(
        current.heroes["102"].sections["card"].resources[SLOT]
            .files
            .len(),
        2
    );
    let repeated = copy_to(&source, &destination, &AtomicBool::new(false), |_, _, _| {}).unwrap();
    assert_eq!(repeated["modsAdded"], 0);
    assert_eq!(
        library(&destination).heroes["102"].sections["card"].resources[SLOT]
            .files
            .len(),
        2
    );
    assert_eq!(sha256_file(&source.join("library.json")).unwrap(), old_hash);
    crate::staging::remove(root).unwrap();
}

#[test]
fn cancelled_or_invalid_candidate_leaves_current_library_unchanged() {
    for corrupt in [false, true] {
        let (root, source, destination) = fixture();
        let before = sha256_file(&destination.join("library.json")).unwrap();
        let cancel = AtomicBool::new(false);
        if corrupt {
            fs::write(source.join("vault/heroes/102/card/two.png"), b"changed").unwrap();
        }
        assert!(copy_to(&source, &destination, &cancel, |_, _, _| {
            cancel.store(true, Ordering::Release);
        })
        .is_err());
        assert_eq!(
            sha256_file(&destination.join("library.json")).unwrap(),
            before
        );
        assert_eq!(walk(&destination.join("vault")).unwrap().len(), 1);
        assert!(!destination.join("migration.json").exists());
        assert!(!destination.join("migration-pending.json").exists());
        crate::staging::remove(root).unwrap();
    }
}

#[test]
fn failed_merge_publication_restores_library_and_new_payloads() {
    let (root, source, destination) = fixture();
    let before = sha256_file(&destination.join("library.json")).unwrap();
    // 完成标记被目录占用，制造库已发布而标记尚未发布的提交失败。
    fs::create_dir(destination.join("migration.json")).unwrap();
    assert!(copy_to(&source, &destination, &AtomicBool::new(false), |_, _, _| {}).is_err());
    assert_eq!(
        sha256_file(&destination.join("library.json")).unwrap(),
        before
    );
    assert_eq!(walk(&destination.join("vault")).unwrap().len(), 1);
    assert!(destination.join("migration.json").is_dir());
    assert!(!destination.join("migration-pending.json").exists());
    crate::staging::remove(root).unwrap();
}

#[test]
#[ignore = "只用于核对 B 盘隔离迁移副本，需 APMM_OLD_DATA 和 ASTRAL_DEV_DATA_DIR"]
fn user_legacy_rescan_preserves_all_candidates() {
    let source = PathBuf::from(std::env::var("APMM_OLD_DATA").unwrap());
    let destination = PathBuf::from(std::env::var("ASTRAL_DEV_DATA_DIR").unwrap())
        .canonicalize()
        .unwrap();
    let work = crate::staging::work_root().canonicalize().unwrap();
    assert!(destination.starts_with(&work) && destination != work);
    let old = library(&source);
    let mut current = library(&destination);
    let before = current.clone();
    let errors = crate::vault_sync::synchronize(
        &mut current,
        &destination.join("vault"),
        |_| Ok(()),
        || Ok(()),
    )
    .unwrap();
    let mut absent = Vec::new();
    for (id, section, canonical, monster, resource) in entries(&before) {
        let map = if monster {
            &current.monsters
        } else {
            &current.heroes
        };
        let actual = map
            .get(id)
            .and_then(|e| e.sections.get(section))
            .and_then(|s| s.resources.get(canonical));
        for file in &resource.files {
            // 临时副本可按已有规则归入正式槽位，同摘要合并后仍须可见。
            let reclassified = section == "temporary"
                && entries(&current)
                    .iter()
                    .any(|entry| entry.4.files.iter().any(|f| f.hash == file.hash));
            if !actual.is_some_and(|r| r.files.iter().any(|f| f.hash == file.hash)) && !reclassified
            {
                absent.push(json!({"id":id,"section":section,"canonical":canonical,"hash":file.hash,"name":file.source_name}));
            }
        }
    }
    let snapshot = crate::workspace::snapshot_from_library(&current);
    let count = entries(&current)
        .iter()
        .map(|e| e.4.files.len())
        .sum::<usize>();
    let visible = snapshot
        .resources
        .iter()
        .map(|r| r.files.len())
        .sum::<usize>();
    let report = json!({"errors":errors,"absentAfterRescan":absent,"libraryCandidates":count,"visibleCandidates":visible,"oldCandidates":entries(&old).iter().map(|e| e.4.files.len()).sum::<usize>()});
    fs::write(
        destination
            .parent()
            .unwrap()
            .join("migration-rescan-result.json"),
        report.to_string(),
    )
    .unwrap();
    assert!(absent.is_empty(), "重新扫描后缺少候选：{absent:?}");
    assert_eq!(visible, count);
}

#[test]
#[ignore = "需要 APMM_OLD_DATA、APMM_MIGRATION_TARGET_FIXTURE 和全新的 B 盘 ASTRAL_DEV_DATA_DIR"]
fn user_legacy_candidates_survive_copy_merge_and_rescan() {
    let source = PathBuf::from(std::env::var("APMM_OLD_DATA").unwrap());
    let fixture = PathBuf::from(std::env::var("APMM_MIGRATION_TARGET_FIXTURE").unwrap());
    let destination = PathBuf::from(std::env::var("ASTRAL_DEV_DATA_DIR").unwrap());
    assert!(!destination.exists());
    ensure_dir(&destination).unwrap();
    let destination = destination.canonicalize().unwrap();
    let work = crate::staging::work_root().canonicalize().unwrap();
    assert!(destination.starts_with(&work) && destination != work);
    let old = library(&source);
    let mut baseline = library(&fixture);
    let old_hash = sha256_file(&source.join("library.json")).unwrap();
    let old_files = entries(&old).iter().map(|e| e.4.files.len()).sum::<usize>();
    let old_multiple = entries(&old).iter().filter(|e| e.4.files.len() > 1).count();
    // 目标取已有扫描库的隔离副本，只复制其候选文件，不操作真实游戏目录。
    for (_, _, _, _, resource) in entries(&baseline) {
        for file in &resource.files {
            let relative = if file.rel_path.is_empty() {
                format!("{}{}", file.hash, file.ext)
            } else {
                file.rel_path.clone()
            };
            let target = destination.join("vault").join(relative);
            ensure_dir(target.parent().unwrap()).unwrap();
            fs::copy(vault_path(&fixture.join("vault"), file), target).unwrap();
        }
    }
    baseline.settings.monitor_directories = false;
    save(&destination, &baseline);
    let baseline_settings = serde_json::to_value(&baseline.settings).unwrap();
    let result = copy_to(&source, &destination, &AtomicBool::new(false), |_, _, _| {}).unwrap();
    let mut current = library(&destination);
    assert_eq!(
        serde_json::to_value(&current.settings).unwrap(),
        baseline_settings
    );
    for (id, section, canonical, monster, resource) in entries(&old) {
        if resource.files.is_empty() {
            continue;
        }
        let map = if monster {
            &current.monsters
        } else {
            &current.heroes
        };
        let actual = &map[id].sections[section].resources[canonical];
        for file in &resource.files {
            let actual_file = actual.files.iter().find(|f| f.hash == file.hash).unwrap();
            assert_eq!(
                sha256_file(&vault_path(&destination.join("vault"), actual_file)).unwrap(),
                file.hash
            );
        }
    }
    for (id, section, canonical, monster, resource) in entries(&baseline) {
        let map = if monster {
            &current.monsters
        } else {
            &current.heroes
        };
        assert_eq!(
            map[id].sections[section].resources[canonical].active_hash,
            resource.active_hash
        );
    }
    crate::vault_sync::synchronize(
        &mut current,
        &destination.join("vault"),
        |_| Ok(()),
        || Ok(()),
    )
    .unwrap();
    let snapshot = crate::workspace::snapshot_from_library(&current);
    let total = snapshot
        .resources
        .iter()
        .map(|r| r.files.len())
        .sum::<usize>();
    let multiple = snapshot
        .resources
        .iter()
        .filter(|r| r.files.len() > 1)
        .count();
    assert!(total >= old_files);
    assert!(multiple >= old_multiple);
    assert_eq!(sha256_file(&source.join("library.json")).unwrap(), old_hash);
    let report = json!({"sourceCandidates":old_files,"sourceMultipleSlots":old_multiple,"result":result,
        "snapshotCandidates":total,"snapshotMultipleSlots":multiple,"sourceLibraryUnchanged":true,
        "destination":destination,"gameModsApplied":false});
    fs::write(
        destination
            .parent()
            .unwrap()
            .join("migration-candidates-result.json"),
        report.to_string(),
    )
    .unwrap();
    // 同时核验空库完整迁移，保留隔离副本作为可审查结果。
    let full = destination.parent().unwrap().join("full-copy");
    copy_to(&source, &full, &AtomicBool::new(false), |_, _, _| {}).unwrap();
    assert_eq!(
        entries(&library(&full))
            .iter()
            .map(|e| e.4.files.len())
            .sum::<usize>(),
        old_files
    );
}
