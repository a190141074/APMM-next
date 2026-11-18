//! 草稿、不可变快照和来源协议的隔离文件验证，不调用真实系统凭据库。
use super::*;
#[test]
#[ignore = "需要独立 ASTRAL_DEV_DATA_DIR，使用 --include-ignored 验收"]
fn author_draft_snapshot_preserves_bytes_and_rejects_stale_save() {
    assert!(
        std::env::var("ASTRAL_DEV_DATA_DIR").is_ok(),
        "必须指定隔离数据目录"
    );
    let draft = store::author_new(None).unwrap();
    let root = crate::staging::create("author-test").unwrap();
    let source = root.join("UT_HandCard_10001.png");
    crate::management_tests::png(&source, 74);
    let original = fs::read(&source).unwrap();
    let mut draft = tauri::async_runtime::block_on(sources::author_add_source(
        draft.id.clone(),
        source.to_string_lossy().into(),
        String::new(),
        false,
    ))
    .unwrap();
    draft.metadata = AuthorMetadata {
        title: "测试".into(),
        description: "保留原始字节".into(),
        version: "1".into(),
        ..Default::default()
    };
    let stale = draft.clone();
    let draft = store::author_save(draft).unwrap();
    assert!(store::author_save(stale).is_err());
    let work = AuthorWork {
        id: uuid::Uuid::new_v4().to_string(),
        ..Default::default()
    };
    let snapshot = snapshot::create(draft.clone(), &work).unwrap();
    let retry = snapshot::create(draft.clone(), &work).unwrap();
    assert_eq!(snapshot.preview.id, retry.preview.id);
    assert_eq!(snapshot.archive_hash, retry.archive_hash);
    let mut zip = zip::ZipArchive::new(
        fs::File::open(
            snapshot::path(&snapshot.preview.id)
                .unwrap()
                .join("upload.zip"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut stored = Vec::new();
    zip.by_name(&format!("{}.png", draft.files[0].id))
        .unwrap()
        .read_to_end(&mut stored)
        .unwrap();
    assert_eq!(stored, original);
    assert_eq!(snapshot.preview.added.len(), 1);
    let mut changed = draft.clone();
    changed.metadata.title = "后续修改".into();
    let next = snapshot::create(changed, &work).unwrap();
    assert_ne!(snapshot.preview.id, next.preview.id);
    assert_eq!(
        snapshot::read(&snapshot.preview.id)
            .unwrap()
            .preview
            .draft
            .metadata
            .title,
        "测试"
    );
    assert!(crate::task_history::retry_args(
        "author-upload",
        &serde_json::json!({"authorization":"secret"})
    )
    .is_none());
    assert!(crate::task_history::retry_args(
        "author-upload",
        &serde_json::json!({"id":snapshot.preview.id})
    )
    .is_some());
}

#[test]
fn author_credential_backup_crypto_rejects_wrong_password() {
    let secret = b"isolated synthetic credential";
    let encrypted = crate::packages::encrypt_package(secret, "password-12345").unwrap();
    assert!(!encrypted
        .windows(secret.len())
        .any(|window| window == secret));
    assert_eq!(
        crate::packages::decrypt_package(&encrypted, "password-12345").unwrap(),
        secret
    );
    assert!(crate::packages::decrypt_package(&encrypted, "wrong").is_err());
}

#[test]
#[ignore = "需要独立 ASTRAL_DEV_DATA_DIR，使用 --include-ignored 验收"]
fn author_offline_export_restore_without_credentials() {
    assert!(std::env::var("ASTRAL_DEV_DATA_DIR").is_ok());
    let root = crate::staging::create("author-offline").unwrap();
    let source = root.join("UT_HandCard_10001.png");
    crate::management_tests::png(&source, 82);
    let draft = store::author_new(None).unwrap();
    let mut draft = tauri::async_runtime::block_on(sources::author_add_source(
        draft.id,
        source.to_string_lossy().into(),
        String::new(),
        false,
    ))
    .unwrap();
    draft.metadata = AuthorMetadata {
        title: "离线往返".into(),
        author: "作者第一行\n作者第二行".into(),
        artist: "画师测试".into(),
        author_urls: vec!["https://example.com/author".into()],
        artist_urls: vec!["https://example.com/artist".into()],
        source_url: "https://example.com/source".into(),
        description: "不需要服务或凭据".into(),
        version: "1.0".into(),
        ..Default::default()
    };
    let draft = store::author_save(draft).unwrap();
    let draft = tauri::async_runtime::block_on(cover::author_set_cover(
        draft.id.clone(),
        draft.revision,
        source.to_string_lossy().into(),
    ))
    .unwrap();
    let preview =
        tauri::async_runtime::block_on(snapshot::author_preflight(draft.id.clone())).unwrap();
    let export = root.join("submission.zip");
    snapshot::author_export(preview.id.clone(), export.to_string_lossy().into()).unwrap();
    let receipts = store::author_state().unwrap().receipts;
    assert_eq!(
        receipts
            .iter()
            .find(|item| item.id == preview.id)
            .unwrap()
            .status,
        "exported"
    );
    assert!(snapshot::author_export(preview.id.clone(), export.to_string_lossy().into()).is_err());
    let empty = store::author_new(None).unwrap();
    let restored = offline::import(&empty.id, &export).unwrap().unwrap();
    assert_eq!(restored.metadata, draft.metadata);
    assert_eq!(restored.files[0].sha256, draft.files[0].sha256);
    assert_eq!(restored.images[0].sha256, draft.images[0].sha256);
    assert_eq!(restored.work_id, preview.draft.work_id);
    assert!(offline::import(&restored.id, &export).is_err());
    if let Ok(destination) = std::env::var("APMM_WORKSHOP_EXPORT_FIXTURE") {
        fs::copy(export, destination).unwrap();
    }
}
