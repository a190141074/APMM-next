//! 增量观察的回归覆盖真实文件、跨启动缓存及失败边界，不把编译视为性能验收。
use crate::directory_observation::Observation;
use crate::file_verification::{library_token, VerificationCache, VerificationStats};
use crate::management_tests::{add, fixture, png, slot};
use crate::*;

#[test]
fn incremental_warm_restart_reads_no_png_and_single_event_is_scoped() {
    let (root, game, vault, mut lib) = fixture();
    let a = add(&mut lib, &vault, &slot("cursor_up"), "cursor_up.png", 22);
    let b = add(
        &mut lib,
        &vault,
        &slot("cursor_down"),
        "cursor_down.png",
        23,
    );
    let roots = vec![vault.clone(), game.clone()];
    let path = root.join("verification.json");
    let mut cache = VerificationCache::load(&path, "fixture".into());
    let cold =
        Observation::collect(&roots, &mut cache, None, &[], false, || Ok(()), |_, _| {}).unwrap();
    assert_eq!(cold.stats.decoded, 2);
    cache.save(&path).unwrap();
    let mut cache = VerificationCache::load(&path, "fixture".into());
    let warm =
        Observation::collect(&roots, &mut cache, None, &[], false, || Ok(()), |_, _| {}).unwrap();
    assert_eq!(
        (
            warm.stats.reused,
            warm.stats.content_reads,
            warm.stats.decoded
        ),
        (2, 0, 0)
    );
    let changed = vault_path(&vault, &a);
    png(&changed, 24);
    let edited = Observation::collect(
        &roots,
        &mut cache,
        Some(&warm),
        &[changed.clone()],
        false,
        || Ok(()),
        |_, _| {},
    )
    .unwrap();
    assert_eq!(
        (
            edited.stats.checked,
            edited.stats.content_reads,
            edited.stats.decoded
        ),
        (1, 1, 1)
    );
    assert_eq!(
        edited.root(&vault).unwrap().files[&vault_path(&vault, &b)]
            .as_ref()
            .unwrap()
            .hash,
        b.hash
    );
    assert_ne!(
        edited.root(&vault).unwrap().files[&changed]
            .as_ref()
            .unwrap()
            .hash,
        a.hash
    );
    staging::remove(root).unwrap();
}

#[test]
fn incremental_notification_and_full_scan_detect_preserved_timestamp() {
    let root = staging::create("incremental-time").unwrap();
    let path = root.join("image.png");
    png(&path, 11);
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let size = fs::metadata(&path).unwrap().len();
    let mut cache = VerificationCache::default();
    let original = cache
        .verify(&path, true, false, &mut VerificationStats::default())
        .unwrap();
    png(&path, 12);
    assert_eq!(fs::metadata(&path).unwrap().len(), size);
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    // 快速启动接受这一明确边界，具体文件通知与强制检查必须绕过元数据命中。
    let fast = cache
        .verify(&path, true, false, &mut VerificationStats::default())
        .unwrap();
    assert_eq!(fast.hash, original.hash);
    let forced = cache
        .verify(&path, true, true, &mut VerificationStats::default())
        .unwrap();
    assert_ne!(forced.hash, original.hash);
    staging::remove(root).unwrap();
}

#[test]
fn incremental_cache_corruption_and_different_root_rebuild_safely() {
    let root = staging::create("incremental-cache").unwrap();
    let image = root.join("image.png");
    png(&image, 15);
    let path = root.join("cache.json");
    let mut cache = VerificationCache::load(&path, "one".into());
    cache
        .verify(&image, true, false, &mut VerificationStats::default())
        .unwrap();
    cache.save(&path).unwrap();
    assert!(VerificationCache::load(&path, "two".into())
        .entries
        .is_empty());
    fs::write(&path, b"{truncated").unwrap();
    let mut cache = VerificationCache::load(&path, "one".into());
    let mut stats = VerificationStats::default();
    cache.verify(&image, true, false, &mut stats).unwrap();
    assert_eq!(stats.decoded, 1);
    staging::remove(root).unwrap();
}

#[test]
fn incremental_invalid_image_does_not_delete_candidate_or_confirm_corrupt_cache() {
    let (root, game, vault, mut lib) = fixture();
    let candidate = add(&mut lib, &vault, &slot("cursor_up"), "cursor_up.png", 25);
    let path = vault_path(&vault, &candidate);
    let roots = vec![vault.clone(), game.clone()];
    let mut cache = VerificationCache::default();
    let before =
        Observation::collect(&roots, &mut cache, None, &[], false, || Ok(()), |_, _| {}).unwrap();
    fs::write(&path, b"broken png").unwrap();
    let observation = Observation::collect(
        &roots,
        &mut cache,
        Some(&before),
        &[path.clone()],
        false,
        || Ok(()),
        |_, _| {},
    )
    .unwrap();
    let result = crate::directory_sync::synchronize_observed(
        &mut lib,
        &vault,
        &game,
        &observation,
        |_| Ok(()),
        || Ok(()),
    )
    .unwrap();
    assert!(!result.errors.is_empty());
    assert!(crate::candidate_management::candidates(&lib)
        .iter()
        .any(|(_, f, _)| f.hash == candidate.hash));
    assert!(!cache.entries.contains_key(&path));
    staging::remove(root).unwrap();
}

#[test]
fn incremental_failed_forced_check_revokes_old_success_with_identical_metadata() {
    let root = staging::create("incremental-revoke").unwrap();
    let path = root.join("image.png");
    png(&path, 61);
    let original = fs::metadata(&path).unwrap();
    let saved = root.join("cache.json");
    let mut cache = VerificationCache::load(&saved, "revocation".into());
    cache
        .verify(&path, true, false, &mut VerificationStats::default())
        .unwrap();
    fs::write(&path, vec![0; original.len() as usize]).unwrap();
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(original.modified().unwrap()))
        .unwrap();
    assert!(cache
        .verify(&path, true, true, &mut VerificationStats::default())
        .is_err());
    cache.save(&saved).unwrap();
    let mut cache = VerificationCache::load(&saved, "revocation".into());
    assert!(cache
        .verify(&path, true, false, &mut VerificationStats::default())
        .is_err());
    assert!(!cache.entries.contains_key(&path));
    staging::remove(root).unwrap();
}

#[test]
fn incremental_rename_delete_and_unavailable_root_are_distinct() {
    let (root, game, vault, mut lib) = fixture();
    let candidate = add(&mut lib, &vault, &slot("cursor_up"), "cursor_up.png", 35);
    let source = vault_path(&vault, &candidate);
    let target = vault.join("renamed.png");
    let roots = vec![vault.clone(), game.clone()];
    let mut cache = VerificationCache::default();
    let prior =
        Observation::collect(&roots, &mut cache, None, &[], false, || Ok(()), |_, _| {}).unwrap();
    fs::rename(&source, &target).unwrap();
    let renamed = Observation::collect(
        &roots,
        &mut cache,
        Some(&prior),
        &[source.clone(), target.clone()],
        false,
        || Ok(()),
        |_, _| {},
    )
    .unwrap();
    assert!(!renamed.root(&vault).unwrap().files.contains_key(&source));
    crate::directory_sync::synchronize_observed(
        &mut lib,
        &vault,
        &game,
        &renamed,
        |_| Ok(()),
        || Ok(()),
    )
    .unwrap();
    assert_eq!(
        crate::candidate_management::candidates(&lib)[0].1.hash,
        candidate.hash
    );
    fs::remove_file(&target).unwrap();
    assert!(!renamed.stable());
    let deleted = Observation::collect(
        &roots,
        &mut cache,
        Some(&renamed),
        &[target],
        false,
        || Ok(()),
        |_, _| {},
    )
    .unwrap();
    assert!(deleted.root(&vault).unwrap().files.is_empty());
    fs::rename(&vault, root.join("offline-vault")).unwrap();
    assert!(Observation::collect(
        &roots,
        &mut cache,
        Some(&deleted),
        &[],
        false,
        || Ok(()),
        |_, _| {}
    )
    .is_err());
    staging::remove(root).unwrap();
}

#[test]
fn incremental_observation_cancellation_and_changed_library_cannot_commit() {
    let (root, game, vault, mut lib) = fixture();
    let candidate = add(&mut lib, &vault, &slot("cursor_up"), "cursor_up.png", 45);
    let roots = vec![vault.clone(), game];
    let mut cache = VerificationCache::default();
    let baseline = library_token(&lib);
    assert!(Observation::collect(
        &roots,
        &mut cache,
        None,
        &[],
        false,
        || Err("取消".into()),
        |_, _| {}
    )
    .is_err());
    assert_eq!(library_token(&lib), baseline);
    let observation =
        Observation::collect(&roots, &mut cache, None, &[], false, || Ok(()), |_, _| {}).unwrap();
    assert!(crate::directory_scan::can_commit(&lib, &lib, &observation));
    let old = lib.clone();
    lib.settings.modimg_path = "changed-game".into();
    assert!(!crate::directory_scan::can_commit(&old, &lib, &observation));
    // 完整扫描没有依赖通知时，也必须拒绝枚举后新增文件的旧观察。
    let added = vault.join("created-after-enumeration.png");
    png(&added, 47);
    assert!(!crate::directory_scan::can_commit(&old, &old, &observation));
    fs::remove_file(added).unwrap();
    png(&vault_path(&vault, &candidate), 46);
    assert!(!crate::directory_scan::can_commit(&old, &old, &observation));
    staging::remove(root).unwrap();
}

#[cfg(windows)]
#[test]
fn incremental_native_notifications_report_paths_and_release_on_drop() {
    let root = staging::create("incremental-watch").unwrap();
    let watch = crate::directory_watch::WatchGroup::start(vec![root.clone()]);
    std::thread::sleep(Duration::from_millis(200));
    let path = root.join("created.png");
    png(&path, 55);
    let event = watch.events.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!event.full);
    assert!(event.paths.contains(&path));
    let before = Instant::now();
    drop(watch);
    assert!(before.elapsed() < Duration::from_secs(2));
    staging::remove(root).unwrap();
}

#[cfg(windows)]
#[test]
fn incremental_native_root_replacement_invalidates_watch() {
    let base = staging::create("incremental-root-watch").unwrap();
    let root = base.join("watched");
    fs::create_dir(&root).unwrap();
    let watch = crate::directory_watch::WatchGroup::start(vec![root.clone()]);
    std::thread::sleep(Duration::from_millis(200));
    fs::rename(&root, base.join("moved")).unwrap();
    fs::create_dir(&root).unwrap();
    let event = watch.events.recv_timeout(Duration::from_secs(3));
    drop(watch);
    staging::remove(base).unwrap();
    assert!(event.is_ok_and(|event| event.full && event.error.is_some()));
}
