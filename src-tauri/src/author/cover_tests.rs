//! 封面事务和公开署名兼容测试，文件操作全部使用独立验收目录。
use super::*;
use crate::workshop::attribution;

#[test]
#[ignore = "需要独立 ASTRAL_DEV_DATA_DIR，使用 --include-ignored 验收"]
fn author_cover_replaces_at_capacity_and_preserves_on_failure() {
    assert!(std::env::var("ASTRAL_DEV_DATA_DIR").is_ok());
    let root = crate::staging::create("author-cover").unwrap();
    let source = root.join("cover.png");
    crate::management_tests::png(&source, 30);
    let mut draft = store::author_new(None).unwrap();
    for _ in 0..7 {
        draft = tauri::async_runtime::block_on(sources::author_add_source(
            draft.id.clone(),
            source.to_string_lossy().into(),
            String::new(),
            true,
        ))
        .unwrap();
    }
    let before = serde_json::to_value(&draft).unwrap();
    let original = fs::read(&source).unwrap();
    let bad = root.join("bad.png");
    fs::write(&bad, b"invalid PNG").unwrap();
    assert!(tauri::async_runtime::block_on(cover::author_set_cover(
        draft.id.clone(),
        draft.revision,
        bad.to_string_lossy().into(),
    ))
    .is_err());
    assert!(tauri::async_runtime::block_on(cover::author_set_cover(
        draft.id.clone(),
        draft.revision - 1,
        source.to_string_lossy().into(),
    ))
    .is_err());
    let stored = store::author_state()
        .unwrap()
        .drafts
        .into_iter()
        .find(|d| d.id == draft.id)
        .unwrap();
    assert_eq!(serde_json::to_value(stored).unwrap(), before);
    let updated = tauri::async_runtime::block_on(cover::author_set_cover(
        draft.id.clone(),
        draft.revision,
        source.to_string_lossy().into(),
    ))
    .unwrap();
    assert_eq!(updated.images.len(), 7);
    assert_ne!(updated.images[0].id, draft.images[0].id);
    assert_eq!(
        serde_json::to_value(&updated.images[1..]).unwrap(),
        serde_json::to_value(&draft.images[1..]).unwrap()
    );
    assert_eq!(updated.revision, draft.revision + 1);
    assert_eq!(fs::read(source).unwrap(), original);
}

#[test]
fn author_attribution_compatibility_and_link_validation() {
    let old = serde_json::json!({"title":"旧投稿","author":"旧作者","description":"简介","category":"","tags":[],"version":"1","gameVersion":"","changeNote":""});
    let old: AuthorMetadata = serde_json::from_value(old).unwrap();
    assert!(old.artist.is_empty() && old.author_urls.is_empty() && old.source_url.is_empty());
    let mut work = serde_json::to_value(crate::workshop::Work::default()).unwrap();
    for key in ["artist", "authorUrls", "artistUrls", "sourceUrl"] {
        work.as_object_mut().unwrap().remove(key);
    }
    let old_work: crate::workshop::Work = serde_json::from_value(work).unwrap();
    assert!(old_work.artist_urls.is_empty());
    assert!(attribution::validate(
        "甲\n乙",
        "画师",
        &["https://example.com/a".into()],
        &[],
        "https://example.com/mod"
    )
    .is_ok());
    for invalid in [
        "http://example.com",
        "javascript:alert(1)",
        "https://user:pass@example.com",
        "https://example.com/\npath",
        "",
    ] {
        assert!(
            attribution::validate("", "", &[invalid.into()], &[], "").is_err(),
            "{invalid}"
        );
    }
    assert!(
        attribution::validate("", "", &vec!["https://example.com".into(); 9], &[], "").is_err()
    );
}
