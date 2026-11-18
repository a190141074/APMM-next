//! 用隔离 PNG、库和 modimg 验证真实导入、首个应用与提交失败回滚。
use super::*;
use crate::management_tests::{add, fixture, png, slot};
use serde_json::json;
use std::cell::{Cell, RefCell};

#[test]
fn drop_names_win_over_wrong_landing_and_match_without_reading_files() {
    let target = json!({"heroId":"cursor","sectionKey":"cursor","canonical":"cursor_down"});
    let (parsed, manual) = resolve("cursor_up.png", "hash", Some(&target), true).unwrap();
    assert_eq!(parsed.canonical, "cursor_up");
    assert!(!manual);
    assert_eq!(
        resolve("cursor_up.png", "hash", Some(&target), false)
            .unwrap()
            .0
            .canonical,
        "cursor_down"
    );
    let matches = images_match_drop(vec![
        "B:/does-not-exist/cursor_up.png".into(),
        "unknown.png".into(),
        "cursor_up.jpg".into(),
    ]);
    assert_eq!(matches[0].target.as_ref().unwrap().canonical, "cursor_up");
    assert!(matches[1].target.is_none() && matches[2].target.is_none());
    assert!(resolve("unknown.png", "hash", None, true).is_err());
}

#[test]
fn drop_first_new_candidate_applies_once_and_keeps_same_slot_candidates() {
    let (root, game, vault, lib) = fixture();
    let a = root.join("a");
    let b = root.join("b");
    ensure_dir(&a).unwrap();
    ensure_dir(&b).unwrap();
    let first = a.join("cursor_up.png");
    let second = b.join("cursor_up.png");
    png(&first, 1);
    png(&second, 2);
    let saved = RefCell::new(lib.clone());
    let results = crate::image_import::import_into(
        json!({"paths":[first,second,first],"dropMode":true}),
        lib,
        &vault,
        |next| {
            *saved.borrow_mut() = next.clone();
            Ok(())
        },
    )
    .unwrap();
    assert!(results[0].applied);
    assert!(!results[1].applied && !results[1].duplicate);
    assert!(results[2].duplicate && !results[2].applied);
    assert_eq!(
        sha256_file(&game.join("cursor_up.png")).unwrap(),
        results[0].hash.clone().unwrap()
    );
    let lib = saved.borrow();
    let r = &lib_map(&lib, "cursors")["cursor"].sections["cursor"].resources["cursor_up"];
    assert_eq!(r.files.len(), 2);
    assert_eq!(r.active_hash, results[0].hash);
    assert_ne!(r.files[0].rel_path, r.files[1].rel_path);
    staging::remove(root).unwrap();
}

#[test]
fn drop_landing_renames_only_vault_copy_and_records_original_name() {
    let (root, _, vault, lib) = fixture();
    let source = root.join("我的图片.png");
    png(&source, 1);
    let hash = sha256_file(&source).unwrap();
    let saved = RefCell::new(lib.clone());
    let results = crate::image_import::import_into(json!({"paths":[source],"dropMode":true,"target":{"heroId":"cursor","sectionKey":"cursor","canonical":"cursor_up"}}), lib, &vault, |next| { *saved.borrow_mut() = next.clone(); Ok(()) }).unwrap();
    assert!(results[0].applied);
    assert_eq!(sha256_file(&source).unwrap(), hash);
    let lib = saved.borrow();
    let entry =
        &lib_map(&lib, "cursors")["cursor"].sections["cursor"].resources["cursor_up"].files[0];
    assert_eq!(entry.source_name, "cursor_up.png");
    assert_eq!(entry.classification.original_name, "我的图片.png");
    assert!(entry.classification.manual);
    staging::remove(root).unwrap();
}

#[test]
fn drop_existing_candidates_and_temporary_images_do_not_apply() {
    let (root, game, vault, mut lib) = fixture();
    add(&mut lib, &vault, &slot("cursor_up"), "old.png", 1);
    let known = root.join("cursor_up.png");
    let unknown = root.join("unknown.png");
    png(&known, 2);
    png(&unknown, 3);
    let results = crate::image_import::import_into(json!({"paths":[known,unknown],"dropMode":true,"targets":{unknown.to_string_lossy().as_ref():{"temporary":true}}}), lib, &vault, |_| Ok(())).unwrap();
    assert!(results
        .iter()
        .all(|r| r.ok && !r.applied && r.apply_error.is_none()));
    assert_eq!(results[1].section_key.as_deref(), Some("temporary"));
    assert!(!game.join("cursor_up.png").exists());
    staging::remove(root).unwrap();
}

#[test]
fn drop_application_commit_failure_keeps_import_and_rolls_back_game() {
    let (root, game, vault, lib) = fixture();
    let source = root.join("cursor_up.png");
    png(&source, 1);
    let original = game.join("cursor_up.png.disabled");
    fs::write(&original, b"official").unwrap();
    let calls = Cell::new(0);
    let saved = RefCell::new(lib.clone());
    let results = crate::image_import::import_into(
        json!({"paths":[source],"dropMode":true}),
        lib,
        &vault,
        |next| {
            calls.set(calls.get() + 1);
            if calls.get() == 2 {
                return Err("模拟应用状态提交失败".into());
            }
            *saved.borrow_mut() = next.clone();
            Ok(())
        },
    )
    .unwrap();
    assert!(results[0].ok && !results[0].applied);
    assert!(results[0]
        .apply_error
        .as_ref()
        .unwrap()
        .contains("模拟应用状态提交失败"));
    assert_eq!(fs::read(original).unwrap(), b"official");
    assert!(!game.join("cursor_up.png").exists());
    let lib = saved.borrow();
    let r = &lib_map(&lib, "cursors")["cursor"].sections["cursor"].resources["cursor_up"];
    assert_eq!(r.files.len(), 1);
    assert!(r.active_hash.is_none());
    assert!(vault_path(&vault, &r.files[0]).exists());
    staging::remove(root).unwrap();
}

#[test]
fn drop_missing_directory_retains_import_and_regular_picker_never_applies() {
    let (root, _, vault, mut lib) = fixture();
    lib.settings.modimg_path.clear();
    let source = root.join("cursor_up.png");
    png(&source, 1);
    let results = crate::image_import::import_into(
        json!({"paths":[source],"dropMode":true}),
        lib.clone(),
        &vault,
        |_| Ok(()),
    )
    .unwrap();
    assert!(results[0].ok && results[0].apply_error.is_some());
    let results =
        crate::image_import::import_into(json!({"paths":[source]}), lib, &vault, |_| Ok(()))
            .unwrap();
    assert!(results[0].ok && !results[0].applied && results[0].apply_error.is_none());
    staging::remove(root).unwrap();
}

#[test]
fn drop_rejects_changed_or_broken_png_before_library_commit() {
    let (root, _, vault, lib) = fixture();
    let source = root.join("cursor_up.png");
    png(&source, 1);
    let broken = root.join("cursor_down.png");
    fs::write(&broken, b"\x89PNG\r\n\x1a\n").unwrap();
    let results = crate::image_import::import_into(json!({"paths":[source,broken],"dropMode":true,"expectedHashes":{source.to_string_lossy().as_ref():"changed"}}), lib, &vault, |_| Ok(())).unwrap();
    assert!(results.iter().all(|r| !r.ok));
    assert!(results[0].reason.as_ref().unwrap().contains("发生变化"));
    staging::remove(root).unwrap();
}
