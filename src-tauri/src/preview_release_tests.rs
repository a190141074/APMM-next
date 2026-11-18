//! 预览发布协议回归；真实包安装只在显式指定的独立数据目录执行。
use crate::*;

#[test]
fn preview_versioned_cnb_path_is_not_flattened_or_proxied() {
    assert!(PREVIEW_MANIFEST_URL.ends_with("/preview-manifest-v4.json"));
    let cfg = cnb_preview_config();
    let url = format!(
        "{}bundles/preview-v4/APMM-previews-chips-4.zip",
        cfg.raw_prefix
    );
    assert_eq!(cnb_preview_bundle_url(&url).as_deref(), Some(url.as_str()));
    let sources = preview_bundle_sources(&url);
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].url, url);
    assert_eq!(sources[0].kind, SourceKind::Cnb);
}

#[test]
fn preview_groups_cover_the_embedded_catalog_exactly_once() {
    let expected: HashSet<_> = expected_preview_files().into_iter().collect();
    let mut actual = HashSet::new();
    for key in preview_bundle_keys() {
        let group = expected_preview_files_for_bundle(key);
        assert!(!group.is_empty(), "{key}");
        for file in group {
            assert!(actual.insert(file));
        }
    }
    assert_eq!(actual, expected);
    assert!(actual.contains("cursor_up.webp"));
    assert!(actual.contains("UT_Relic_50075_1.webp"));
}

#[test]
fn preview_same_version_repairs_missing_files_without_downgrade() {
    let local = PreviewBundleStatus {
        key: "heroes".into(),
        local_version: Some("4".into()),
        cached_files: 1371,
        expected_files: 1710,
        missing_files: 339,
        state: "partial".into(),
    };
    assert!(preview_needs_update(Some(&local), "4"));
    assert!(preview_needs_update(Some(&local), "5"));
    assert!(!preview_needs_update(Some(&local), "3"));
    assert!(!preview_needs_update(
        Some(&PreviewBundleStatus {
            missing_files: 0,
            ..local
        }),
        "4"
    ));
}

#[test]
#[ignore = "需要 APMM_PREVIEW_PUBLIC 与全新的 ASTRAL_DEV_DATA_DIR。"]
fn preview_release_artifacts_install_with_real_protocol() {
    let root = PathBuf::from(std::env::var("APMM_PREVIEW_PUBLIC").unwrap());
    let isolated = PathBuf::from(std::env::var("ASTRAL_DEV_DATA_DIR").unwrap());
    assert_eq!(data_dir(), isolated);
    assert!(!preview_cache_dir().exists(), "真实包测试要求空白预览目录");
    let modern: RemotePreviewManifest =
        serde_json::from_slice(&fs::read(root.join("preview-manifest-v4.json")).unwrap()).unwrap();
    let modern = validate_preview_manifest(modern).unwrap();
    assert_eq!(modern.bundles.len(), preview_bundle_keys().len());
    let cfg = cnb_preview_config();
    for bundle in &modern.bundles {
        let relative = bundle.bundle_url.strip_prefix(&cfg.raw_prefix).unwrap();
        let bytes = fs::read(root.join(relative)).unwrap();
        assert_eq!(bytes.len() as u64, bundle.bytes);
        install_preview_bundle_section(&bytes, &remote_as_bundle(bundle), None).unwrap();
    }
    let status = preview_cache_status();
    assert_eq!(status.missing_files, 0);
    assert_eq!(status.cached_files, expected_preview_files().len());
    assert!(status
        .bundles
        .iter()
        .all(|b| b.local_version.as_deref() == Some("4")));
    // 兼容包使用旧客户端的内嵌目录核验，不以新分类规则改写旧白名单。
    let legacy_catalog: Vec<serde_json::Value> = serde_json::from_slice(
        &fs::read(std::env::var("APMM_LEGACY_PREVIEW_CATALOG").unwrap()).unwrap(),
    )
    .unwrap();
    let legacy: RemotePreviewManifest =
        serde_json::from_slice(&fs::read(root.join("preview-manifest.json")).unwrap()).unwrap();
    let legacy = validate_preview_manifest(legacy).unwrap();
    assert_eq!(legacy.bundles.len(), 3);
    for bundle in &legacy.bundles {
        let filename = bundle.bundle_url.rsplit('/').next().unwrap();
        let bytes = fs::read(root.join("bundles").join(filename)).unwrap();
        assert_eq!(sha256_bytes(&bytes), bundle.sha256);
        assert_eq!(bytes.len() as u64, bundle.bytes);
        let expected: HashSet<_> = legacy_catalog
            .iter()
            .filter(|entry| {
                entry.get("preview").is_none()
                    && category_of(entry["section"].as_str().unwrap()) == bundle.key
            })
            .map(|entry| entry["previewFile"].as_str().unwrap().to_string())
            .collect();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut actual = HashSet::new();
        for index in 0..archive.len() {
            let entry = archive.by_index(index).unwrap();
            let name = entry.name().strip_prefix("previews/").unwrap();
            assert!(!name.contains('/') && !name.contains('\\'));
            assert!(actual.insert(name.to_string()));
        }
        assert_eq!(actual.len(), bundle.files);
        assert_eq!(actual, expected);
    }
    assert_eq!(preview_cache_status().missing_files, 0);
}
