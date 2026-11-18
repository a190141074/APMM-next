//! 检测只依赖隔离安装标记和候选元数据，验证同目录接续与独立安装的补齐提示。
use super::*;

fn write_library(root: &Path, count: usize) {
    ensure_dir(root).unwrap();
    let mut library = Library::default();
    get_resource(&mut library, "heroes", "101", "card", "UT_Hero_Card_101").files = (0..count)
        .map(|n| FileEntry {
            hash: format!("{n:064x}"),
            source_name: "同部位.png".into(),
            ..Default::default()
        })
        .collect();
    fs::write(
        root.join("library.json"),
        serde_json::to_vec(&library).unwrap(),
    )
    .unwrap();
}

#[test]
fn in_place_data_is_inherited_and_never_offered_as_its_own_source() {
    let root = crate::staging::create("migration-detect-inplace").unwrap();
    let data = root.join("data");
    write_library(&data, 2);
    fs::write(root.join("astral-updater.exe"), b"legacy marker").unwrap();
    let before = sha256_file(&data.join("library.json")).unwrap();
    let status = detect(&data, &root);
    assert_eq!(status.mode, "inherited");
    assert_eq!((status.mods, status.multiple_slots), (2, 1));
    assert!(status
        .sources
        .iter()
        .all(|s| !same_path(Path::new(&s.source), &data)));
    assert!(same_path(&data, &data.canonicalize().unwrap()));
    assert_eq!(sha256_file(&data.join("library.json")).unwrap(), before);
    crate::staging::remove(root).unwrap();
}

#[test]
fn separate_install_detects_missing_backups_even_after_migration_was_marked() {
    let root = crate::staging::create("migration-detect-separate").unwrap();
    let old = root.join("old-0.3/data");
    let current = root.join("APMM-next/data");
    write_library(&old, 3);
    write_library(&current, 1);
    fs::write(root.join("old-0.3/astral-updater.exe"), b"legacy marker").unwrap();
    fs::write(
        current.join("migration.json"),
        serde_json::json!({"source":old,"completedAt":1}).to_string(),
    )
    .unwrap();
    let status = detect(&current, current.parent().unwrap());
    assert_eq!(status.mode, "migrated");
    let source = status
        .sources
        .iter()
        .find(|s| same_path(Path::new(&s.source), &old))
        .unwrap();
    assert_eq!((source.mods, source.new_mods), (3, 2));
    // 补齐后不再提示缺失，原先的迁移标记也不充当候选完整性依据。
    write_library(&current, 3);
    assert_eq!(
        detect(&current, current.parent().unwrap())
            .sources
            .iter()
            .find(|s| same_path(Path::new(&s.source), &old))
            .unwrap()
            .new_mods,
        0
    );
    crate::staging::remove(root).unwrap();
}

#[test]
fn fresh_install_detects_neighboring_legacy_install_without_migration_marker() {
    let root = crate::staging::create("migration-detect-fresh-neighbor").unwrap();
    let old = root.join("AstralPartyModManager-OldVersion/data");
    let old_install = old.parent().unwrap();
    let current = root.join("APMM-next/data");
    write_library(&old, 3);
    write_library(&current, 1);
    fs::write(old_install.join("astral-mod-manager.exe"), b"0.3.36").unwrap();
    fs::write(
        old_install.join("release-manifest.json"),
        r#"{"version":"0.3.36"}"#,
    )
    .unwrap();

    let status = detect(&current, current.parent().unwrap());
    let source = status
        .sources
        .iter()
        .find(|s| same_path(Path::new(&s.source), &old))
        .expect("独立的新安装应发现相邻的 0.3.x data");
    assert_eq!(source.version.as_deref(), Some("0.3.36"));
    assert_eq!((source.mods, source.new_mods), (3, 2));
    assert!(!current.join("migration.json").exists());
    crate::staging::remove(root).unwrap();
}
