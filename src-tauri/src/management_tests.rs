//! 管理功能使用工作盘隔离夹具，分别验证持久兼容、预设事务和被动同步边界。
use crate::management_types::*;
use crate::*;

pub(super) fn fixture() -> (PathBuf, PathBuf, PathBuf, Library) {
    let root = staging::create("management-test").unwrap();
    let game = root.join("game");
    let vault = root.join("vault");
    ensure_dir(&game).unwrap();
    ensure_dir(&vault).unwrap();
    let mut lib = Library::default();
    lib.settings.modimg_path = game.to_string_lossy().into_owned();
    (root, game, vault, lib)
}
pub(super) fn png(path: &Path, color: u8) {
    image::RgbaImage::from_pixel(2, 2, image::Rgba([color, 20, 30, 255]))
        .save_with_format(path, image::ImageFormat::Png)
        .unwrap();
}
pub(super) fn slot(cn: &str) -> ResourceSlot {
    ResourceSlot::from_parsed(&parse_filename(cn).unwrap())
}
pub(super) fn add(
    lib: &mut Library,
    vault: &Path,
    s: &ResourceSlot,
    name: &str,
    color: u8,
) -> FileEntry {
    let temp = vault.join(format!("fixture-{color}.png"));
    png(&temp, color);
    let hash = sha256_file(&temp).unwrap();
    let (path, rel) = place_in_vault(
        vault,
        &s.category,
        &s.hero_id,
        &s.section_key,
        name,
        &hash,
        ".png",
    )
    .unwrap();
    fs::rename(temp, path).unwrap();
    let f = FileEntry {
        hash,
        source_name: name.into(),
        ext: ".png".into(),
        rel_path: rel,
        classification: ClassificationRecord {
            original_name: name.into(),
            ..Default::default()
        },
        ..Default::default()
    };
    get_resource(lib, &s.category, &s.hero_id, &s.section_key, &s.canonical)
        .files
        .push(f.clone());
    f
}
pub(super) fn item(s: &ResourceSlot, f: Option<&FileEntry>) -> PresetItem {
    PresetItem {
        category: s.category.clone(),
        hero_id: s.hero_id.clone(),
        section_key: s.section_key.clone(),
        canonical: s.canonical.clone(),
        hash: f.map(|f| f.hash.clone()).unwrap_or_default(),
        display_name: f
            .map(|f| f.source_name.clone())
            .unwrap_or_else(|| s.canonical.clone()),
    }
}

#[test]
fn management_old_data_and_shared_metadata_compatibility() {
    let p:GlobalPreset=serde_json::from_str(r#"{"id":"old","name":"旧预设","items":[{"category":"heroes","heroId":"cursor","sectionKey":"cursor","canonical":"cursor_up","hash":"abc"}]}"#).unwrap();
    assert_eq!(p.scope, "global");
    assert!(!p.locked);
    assert_eq!(p.items[0].hash, "abc");
    assert!(p.items[0].slot().valid());
    assert!(Settings::default().monitor_directories);
    let mut local = CandidateInfo {
        provenance: Provenance {
            author: "本机作者".into(),
            ..Default::default()
        },
        note: "本机备注".into(),
    };
    local.fill_empty(&CandidateInfo {
        provenance: Provenance {
            author: "包作者".into(),
            source_url: "https://example.com/mod".into(),
            version: "1.2".into(),
            author_urls: vec![
                "https://example.com/author".into(),
                "https://example.com/author2".into(),
            ],
            artist: "画师".into(),
            artist_urls: vec![
                "https://example.com/artist".into(),
                "https://example.com/artist2".into(),
            ],
        },
        note: String::new(),
    });
    assert_eq!(local.provenance.author, "本机作者");
    assert_eq!(local.note, "本机备注");
    assert_eq!(local.provenance.version, "1.2");
    assert_eq!(local.provenance.artist, "画师");
    assert_eq!(local.provenance.author_urls.len(), 2);
    assert_eq!(local.provenance.artist_urls.len(), 2);
    let meta:PackageFileMeta=serde_json::from_value(serde_json::json!({"category":"heroes","entityId":"101","sectionKey":"card","canonical":"UT_Hero_Card_101","hash":"abc","ext":".png","sourceName":"UT_Hero_Card_101.png","wasEnabled":false,"provenance":local.provenance})).unwrap();
    let shared = serde_json::to_string(&normalized_package_meta(&meta)).unwrap();
    assert!(shared.contains("本机作者"));
    assert!(!shared.contains("note"));
    assert!(!shared.contains("本机备注"));
    assert!(shared.contains("画师"));
    assert!(shared.contains("artist2"));
}

#[test]
fn management_display_name_is_free_and_does_not_move_or_modify_disabled_candidate() {
    let (root, game, vault, mut lib) = fixture();
    let slot = slot("cursor_up");
    let file = add(&mut lib, &vault, &slot, "old.png", 90);
    get_resource(
        &mut lib,
        &slot.category,
        &slot.hero_id,
        &slot.section_key,
        &slot.canonical,
    )
    .disabled = true;
    let path = vault_path(&vault, &file);
    let before = fs::read(&path).unwrap();
    let name = "我的 / 夏日：搭配 🌙";
    assert_eq!(
        candidate_management::rename_display(&mut lib, &slot, &file.hash, name.into()).unwrap(),
        name
    );
    let changed = candidate_management::file_mut(&mut lib, &slot, &file.hash).unwrap();
    assert_eq!(changed.hash, file.hash);
    assert_eq!(changed.rel_path, file.rel_path);
    assert_eq!(changed.classification.original_name, "old.png");
    assert_eq!(fs::read(path).unwrap(), before);
    assert_eq!(fs::read_dir(game).unwrap().count(), 0);
    assert!(candidate_management::rename_display(&mut lib, &slot, &file.hash, " ".into()).is_err());
    staging::remove(root).unwrap();
}

#[test]
fn management_legacy_container_category_applies_as_global_without_rewriting_identity() {
    let (root, game, vault, mut lib) = fixture();
    let a = slot("cursor_up");
    let b = slot("cursor_down");
    let file = add(&mut lib, &vault, &a, "legacy-cursor.png", 82);
    png(&game.join("cursor_down.png"), 83);
    let preset: GlobalPreset = serde_json::from_value(serde_json::json!({
        "id": "legacy-id", "name": "旧鼠标预设", "items": [{
            "category": "heroes", "heroId": "cursor", "sectionKey": "cursor",
            "canonical": "cursor_up", "hash": file.hash
        }]
    }))
    .unwrap();
    let preview =
        preset_domain::preview_with(&lib, &preset, &vault, &game, vec![a.clone(), b.clone()]);
    assert!(preview.can_apply, "{:?}", preview.issues);
    preset_domain::apply_with(&mut lib, &preset, &vault, &game, vec![a, b], |_| Ok(())).unwrap();
    assert_eq!(sha256_file(&game.join("cursor_up.png")).unwrap(), file.hash);
    assert!(!game.join("cursor_down.png").exists());
    assert_eq!(preset.id, "legacy-id");
    assert_eq!(preset.name, "旧鼠标预设");
    assert_eq!(preset.items[0].category, "heroes");
    assert_eq!(preset.items[0].hash, file.hash);
    staging::remove(root).unwrap();
}
#[test]
fn management_local_original_and_disabled_capture_global_preview() {
    let (root, game, vault, mut lib) = fixture();
    let a = slot("cursor_up");
    let b = slot("cursor_down");
    let f = add(&mut lib, &vault, &a, "custom.png", 12);
    png(&game.join("cursor_up.png.disabled"), 12);
    png(&game.join("cursor_down.png"), 20);
    get_resource(
        &mut lib,
        &a.category,
        &a.hero_id,
        &a.section_key,
        &a.canonical,
    )
    .active_hash = Some(f.hash);
    let local =
        preset_domain::capture(&lib, "local".into(), vec![a.clone()], "局部".into()).unwrap();
    assert!(local.items[0].hash.is_empty());
    let preview =
        preset_domain::preview_with(&lib, &local, &vault, &game, vec![a.clone(), b.clone()]);
    assert!(preview.can_apply);
    assert_eq!(preview.changes.len(), 1);
    assert!(preview.changes[0].changed);
    let global = GlobalPreset {
        scope: "global".into(),
        ..local.clone()
    };
    assert_eq!(
        preset_domain::preview_with(&lib, &global, &vault, &game, vec![a, b])
            .changes
            .len(),
        2
    );
    preset_domain::apply_with(&mut lib, &local, &vault, &game, vec![], |_| Ok(())).unwrap();
    assert!(!game.join("cursor_up.png.disabled").exists());
    assert!(game.join("cursor_down.png").exists());
    staging::remove(root).unwrap();
}
#[test]
fn management_preset_aggregates_missing_damaged_disabled_invalid_and_freezes_hash() {
    let (root, game, vault, mut lib) = fixture();
    let a = slot("cursor_up");
    let b = slot("cursor_down");
    let f = add(&mut lib, &vault, &a, "old-name.png", 14);
    let g = add(&mut lib, &vault, &b, "second.png", 15);
    let p = GlobalPreset {
        id: "p".into(),
        name: "p".into(),
        scope: "local".into(),
        items: vec![
            item(&a, Some(&f)),
            item(&b, Some(&g)),
            PresetItem {
                canonical: "invalid".into(),
                hash: "missing".into(),
                display_name: "历史名称".into(),
                ..Default::default()
            },
        ],
        locked: false,
    };
    lib.settings.presets.push(p.clone());
    png(&vault_path(&vault, &f), 16);
    get_resource(
        &mut lib,
        &b.category,
        &b.hero_id,
        &b.section_key,
        &b.canonical,
    )
    .disabled = true;
    let preview = preset_domain::preview_with(&lib, &p, &vault, &game, vec![]);
    assert!(!preview.can_apply);
    assert!(preview.issues.len() >= 4);
    let result = directory_sync::synchronize_with(&mut lib, &vault, &game, |_| Ok(())).unwrap();
    assert!(result.changed);
    assert_eq!(lib.settings.presets[0].items[0].hash, f.hash);
    assert_eq!(
        lib.settings.presets[0].items[0].display_name,
        "old-name.png"
    );
    staging::remove(root).unwrap();
}
#[test]
fn management_preset_commit_failure_rolls_back_all_files_and_index() {
    let (root, game, vault, mut lib) = fixture();
    let a = slot("cursor_up");
    let b = slot("cursor_down");
    let f = add(&mut lib, &vault, &a, "replacement.png", 17);
    png(&game.join("cursor_up.png"), 18);
    png(&game.join("cursor_down.png.disabled"), 19);
    let old = fs::read(game.join("cursor_up.png")).unwrap();
    let down = fs::read(game.join("cursor_down.png.disabled")).unwrap();
    let p = GlobalPreset {
        scope: "local".into(),
        items: vec![item(&a, Some(&f)), item(&b, None)],
        ..Default::default()
    };
    let before = serde_json::to_value(&lib).unwrap();
    assert!(
        preset_domain::apply_with(&mut lib, &p, &vault, &game, vec![], |_| Err(
            "索引提交失败".into()
        ))
        .is_err()
    );
    assert_eq!(fs::read(game.join("cursor_up.png")).unwrap(), old);
    assert_eq!(
        fs::read(game.join("cursor_down.png.disabled")).unwrap(),
        down
    );
    assert_eq!(serde_json::to_value(&lib).unwrap(), before);
    staging::remove(root).unwrap();
}
#[test]
fn management_classification_transaction_merge_metadata_and_failure() {
    let (root, game, vault, mut lib) = fixture();
    let a = slot("cursor_up");
    let b = slot("cursor_down");
    let mut f = add(&mut lib, &vault, &a, "source.png", 21);
    f.info.provenance.author = "作者 A".into();
    f.info.note = "备注".into();
    *candidate_management::file_mut(&mut lib, &a, &f.hash).unwrap() = f.clone();
    let old = vault_path(&vault, &f);
    let before = serde_json::to_value(&lib).unwrap();
    assert!(candidate_management::move_candidate(
        &mut lib,
        &vault,
        &a,
        &f.hash,
        &b,
        true,
        |_| Err("索引失败".into())
    )
    .is_err());
    assert!(old.exists());
    assert_eq!(serde_json::to_value(&lib).unwrap(), before);
    let mut target = add(&mut lib, &vault, &b, "target.png", 21);
    target.info.provenance.author = "目标作者".into();
    *candidate_management::file_mut(&mut lib, &b, &target.hash).unwrap() = target.clone();
    candidate_management::move_candidate(&mut lib, &vault, &a, &f.hash, &b, true, |_| Ok(()))
        .unwrap();
    let t = candidate_management::file_mut(&mut lib, &b, &f.hash).unwrap();
    assert_eq!(t.info.provenance.author, "目标作者");
    assert_eq!(t.info.note, "备注");
    assert!(t.classification.manual);
    assert!(!old.exists());
    assert_eq!(sha256_file(&vault_path(&vault, t)).unwrap(), f.hash);
    assert_eq!(fs::read_dir(game).unwrap().count(), 0);
    staging::remove(root).unwrap();
}
