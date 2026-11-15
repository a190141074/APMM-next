//! 被动同步和归属确认使用独立文件夹，验证重命名身份、内容编辑和取消。
use crate::management_tests::{add, fixture, png, slot};
use crate::management_types::*;
use crate::*;

#[test]
fn management_passive_sync_rename_edit_delete_no_game_writes_or_temp_duplication() {
    let (root, game, vault, mut lib) = fixture();
    let a = slot("cursor_up");
    let mut f = add(&mut lib, &vault, &a, "name.png", 25);
    f.info.provenance.author = "库作者".into();
    *candidate_management::file_mut(&mut lib, &a, &f.hash).unwrap() = f.clone();
    let path = vault_path(&vault, &f);
    let renamed = path.with_file_name("renamed.png");
    fs::rename(&path, &renamed).unwrap();
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    assert_eq!(
        candidate_management::file_mut(&mut lib, &a, &f.hash)
            .unwrap()
            .info
            .provenance
            .author,
        "库作者"
    );
    let len = fs::metadata(&renamed).unwrap().len();
    png(&renamed, 26);
    assert_eq!(fs::metadata(&renamed).unwrap().len(), len);
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    let new_hash = sha256_file(&renamed).unwrap();
    assert_ne!(new_hash, f.hash);
    assert_eq!(
        candidate_management::file_mut(&mut lib, &a, &new_hash)
            .unwrap()
            .info
            .provenance
            .author,
        "库作者"
    );
    png(&game.join("cursor_up.png"), 27);
    let game_hash = sha256_file(&game.join("cursor_up.png")).unwrap();
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    assert!(candidate_management::file_mut(&mut lib, &a, &game_hash)
        .unwrap()
        .info
        .provenance
        .author
        .is_empty());
    assert_eq!(sha256_file(&game.join("cursor_up.png")).unwrap(), game_hash);
    png(&game.join("unrecognized.png"), 28);
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    let hash = sha256_file(&game.join("unrecognized.png")).unwrap();
    let temp = ResourceSlot {
        category: "heroes".into(),
        hero_id: format!("temporary:{hash}"),
        section_key: "temporary".into(),
        canonical: format!("unmatched:{hash}"),
    };
    candidate_management::move_candidate(&mut lib, &vault, &temp, &hash, &a, true, |_| Ok(()))
        .unwrap();
    for _ in 0..3 {
        directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    }
    assert!(candidate_management::candidates(&lib)
        .iter()
        .all(|(s, f, _)| !(s.section_key == "temporary" && f.hash == hash)));
    assert_eq!(
        get_resource(
            &mut lib,
            &a.category,
            &a.hero_id,
            &a.section_key,
            &a.canonical
        )
        .active_hash
        .as_deref(),
        Some(game_hash.as_str())
    );
    fs::remove_file(renamed).unwrap();
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    assert!(candidate_management::file_mut(&mut lib, &a, &new_hash).is_err());
    staging::remove(root).unwrap();
}
#[test]
fn management_manual_rule_confirmation_and_disabled_temporary() {
    let (root, game, vault, mut lib) = fixture();
    let a = slot("cursor_up");
    let b = slot("cursor_down");
    let mut f = add(&mut lib, &vault, &a, "cursor_down.png", 31);
    f.classification.manual = true;
    *candidate_management::file_mut(&mut lib, &a, &f.hash).unwrap() = f.clone();
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    let file = candidate_management::file_mut(&mut lib, &a, &f.hash).unwrap();
    assert_eq!(file.classification.pending, Some(b.clone()));
    file.classification.confirmed_rule = Some(candidate_management::rule_key(&b));
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    assert!(candidate_management::file_mut(&mut lib, &a, &f.hash)
        .unwrap()
        .classification
        .pending
        .is_none());
    let temp = ResourceSlot {
        category: "heroes".into(),
        hero_id: "temporary:test".into(),
        section_key: "temporary".into(),
        canonical: "unmatched:test".into(),
    };
    let mut g = add(&mut lib, &vault, &temp, "cursor_down.png", 32);
    let path = vault_path(&vault, &g);
    let disabled = path.with_file_name("cursor_down.png.disabled");
    fs::rename(path, &disabled).unwrap();
    g.rel_path = disabled
        .strip_prefix(&vault)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    *candidate_management::file_mut(&mut lib, &temp, &g.hash).unwrap() = g.clone();
    get_resource(
        &mut lib,
        &temp.category,
        &temp.hero_id,
        &temp.section_key,
        &temp.canonical,
    )
    .disabled = true;
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    assert!(candidate_management::file_mut(&mut lib, &temp, &g.hash).is_ok());
    assert!(disabled.exists());
    staging::remove(root).unwrap();
}
#[test]
fn management_png_validation_rejects_fake_and_truncated() {
    let root = staging::create("management-png").unwrap();
    let path = root.join("fake.png");
    fs::write(&path, b"not png").unwrap();
    assert!(image_import::validate_png(&path).is_err());
    fs::write(&path, b"\x89PNG\r\n\x1a\n").unwrap();
    assert!(image_import::validate_png(&path).is_err());
    staging::remove(root).unwrap();
}

#[test]
fn management_cross_directory_rename_with_identical_content_keeps_independent_information() {
    let (root, game, vault, mut lib) = fixture();
    let a = slot("cursor_up");
    let b = slot("cursor_down");
    let f = add(&mut lib, &vault, &a, "first.png", 66);
    let g = add(&mut lib, &vault, &b, "second.png", 66);
    candidate_management::file_mut(&mut lib, &a, &f.hash)
        .unwrap()
        .info
        .provenance
        .author = "作者 A".into();
    candidate_management::file_mut(&mut lib, &b, &g.hash)
        .unwrap()
        .info
        .provenance
        .author = "作者 B".into();
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    let moved = vault.join("moved");
    ensure_dir(&moved).unwrap();
    let new_path = moved.join("edited.png");
    fs::rename(vault_path(&vault, &f), &new_path).unwrap();
    png(&new_path, 67);
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    let hash = sha256_file(&new_path).unwrap();
    assert_eq!(
        candidate_management::file_mut(&mut lib, &a, &hash)
            .unwrap()
            .info
            .provenance
            .author,
        "作者 A"
    );
    assert_eq!(
        candidate_management::file_mut(&mut lib, &b, &g.hash)
            .unwrap()
            .info
            .provenance
            .author,
        "作者 B"
    );
    let before = serde_json::to_value(&lib).unwrap();
    assert!(directory_sync::synchronize_controlled(
        &mut lib,
        &vault,
        &game,
        |_| Ok(()),
        || Err("已取消".into())
    )
    .is_err());
    assert_eq!(serde_json::to_value(&lib).unwrap(), before);
    staging::remove(root).unwrap();
}

#[test]
fn management_external_temporary_disabled_changes_preserve_manual_enable_and_rollback() {
    let (root, game, vault, mut lib) = fixture();
    let file = game.join("unknown.png.disabled");
    png(&file, 71);
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    let (slot, entry, disabled) = candidate_management::candidates(&lib)
        .into_iter()
        .next()
        .unwrap();
    assert!(disabled);
    assert!(vault_path(&vault, &entry)
        .to_string_lossy()
        .ends_with(".disabled"));
    let before = serde_json::to_value(&lib).unwrap();
    assert!(candidate_management::sync_temporary_disabled(
        &mut lib,
        &vault,
        &slot,
        &entry.hash,
        false,
        |_| Err("索引失败".into())
    )
    .is_err());
    assert_eq!(serde_json::to_value(&lib).unwrap(), before);
    assert!(vault_path(&vault, &entry).exists());
    let source = vault_path(&vault, &entry);
    let enabled = source.with_file_name("unknown.png");
    fs::rename(&source, &enabled).unwrap();
    candidate_management::file_mut(&mut lib, &slot, &entry.hash)
        .unwrap()
        .rel_path = enabled
        .strip_prefix(&vault)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    get_resource(
        &mut lib,
        &slot.category,
        &slot.hero_id,
        &slot.section_key,
        &slot.canonical,
    )
    .disabled = false;
    directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    assert!(
        !get_resource(
            &mut lib,
            &slot.category,
            &slot.hero_id,
            &slot.section_key,
            &slot.canonical
        )
        .disabled
    );
    assert!(enabled.exists());
    staging::remove(root).unwrap();
}
