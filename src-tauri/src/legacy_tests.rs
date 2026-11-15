use super::*;

fn test_dir(label: &str) -> PathBuf {
    let p = crate::staging::root().join(format!(
        "astral-mod-manager-{}-{}",
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn update_payload_requires_the_complete_runtime_set() {
    let root = test_dir("update-payload");
    let payload = root.join("payload");
    fs::create_dir_all(&payload).unwrap();
    fs::write(payload.join(UPDATE_RUNTIME_FILES[0]), b"exe").unwrap();
    assert!(find_update_payload(&root).is_none());

    for name in UPDATE_RUNTIME_FILES.iter().skip(1) {
        fs::write(payload.join(name), b"runtime").unwrap();
    }
    assert_eq!(
        find_update_payload(&root).as_deref(),
        Some(payload.as_path())
    );
    let _ = crate::staging::remove(root);
}

#[test]
fn preview_proxy_candidates_keep_the_official_url_and_order() {
    let candidates = preview_source_candidates(PREVIEW_MANIFEST_URL);
    // CNB 已托管预览资源并启用：候选为 5 个（CNB + 3 镜像 + GitHub 官方）。
    assert_eq!(candidates.len(), 5);
    assert_eq!(candidates[0].0, CNB_SOURCE_NAME);
    let cfg = cnb_preview_config();
    assert_eq!(
        candidates[0].1,
        format!("{}{}", cfg.raw_prefix, cfg.manifest_path)
    );
    assert_eq!(candidates[1].0, "加速源 ghproxy.net");
    assert_eq!(
        candidates[1].1,
        format!("https://ghproxy.net/{}", PREVIEW_MANIFEST_URL)
    );
    assert_eq!(candidates[2].0, "加速源 cors.isteed.cc");
    assert_eq!(candidates[3].0, "加速源 gh-proxy.com");
    assert_eq!(candidates[4].0, "GitHub 官方");
    assert_eq!(candidates[4].1, PREVIEW_MANIFEST_URL);
    // 官方地址始终保留在候选中（兜底源）。
    assert!(candidates
        .iter()
        .any(|(n, u)| *n == "GitHub 官方" && u == PREVIEW_MANIFEST_URL));
}

#[test]
fn cnb_preview_bundle_url_keeps_the_official_filename() {
    // CNB 直链由官方 URL 的资产名推导，保证两边文件名一致（校验 sha 相同）。
    let official = "https://github.com/a190141074/AstralPartyModManager-Previews/releases/download/preview-v3/astral-party-previews-heroes-preview-v3.zip";
    let cfg = cnb_preview_config();
    assert!(cfg.enabled, "config/cnb.json 应启用 CNB 预览源");
    let url = cnb_preview_bundle_url(official).expect("启用后必须产出 CNB 直链");
    assert_eq!(
        url,
        format!(
            "{}bundles/astral-party-previews-heroes-preview-v3.zip",
            cfg.raw_prefix
        )
    );
    assert!(is_allowed_preview_bundle_url(&url));
}

#[test]
fn preview_bundle_url_must_be_an_allowed_host() {
    // GitHub Release 资产合法。
    assert!(is_allowed_preview_bundle_url(
            "https://github.com/a190141074/AstralPartyModManager-Previews/releases/download/preview-v2/preview.zip"
        ));
    // CNB 预览源已启用，直链在允许列表内（取自 config 的 raw 前缀）。
    let cfg = cnb_preview_config();
    assert!(is_allowed_preview_bundle_url(&format!(
        "{}bundles/preview.zip",
        cfg.raw_prefix
    )));
    // 非允许主机被拒绝。
    assert!(!is_allowed_preview_bundle_url(
        "https://example.com/preview.zip"
    ));
    assert!(!is_allowed_preview_bundle_url(
        "https://github.com/other/repo/releases/download/v1/preview.zip"
    ));
    // 未托管的 CNB 仓库（前缀不匹配）被拒绝。
    assert!(!is_allowed_preview_bundle_url(
        "https://cnb.cool/other/repo/-/git/raw/main/bundles/preview.zip"
    ));
    // 带查询参数（可能重定向）的地址被拒绝。
    assert!(!is_allowed_preview_bundle_url(
            "https://github.com/a190141074/AstralPartyModManager-Previews/releases/download/v1/preview.zip?redirect=1"
        ));
}

#[test]
fn replacement_is_written_before_old_mod_is_removed() {
    let root = test_dir("apply");
    let vault = root.join("vault");
    let modimg = root.join("modimg");
    fs::create_dir_all(&vault).unwrap();
    fs::create_dir_all(&modimg).unwrap();
    let source = vault.join("candidate.png");
    let canonical = "UT_Hero_Card_103";
    let previous = modimg.join(format!("{}.png", canonical));
    fs::write(&source, b"new-image").unwrap();
    fs::write(&previous, b"old-image").unwrap();

    write_entry_to_modimg(&modimg, canonical, &source, ".png").unwrap();
    assert_eq!(fs::read(&previous).unwrap(), b"new-image");

    let missing = vault.join("missing.png");
    assert!(write_entry_to_modimg(&modimg, canonical, &missing, ".png").is_err());
    assert_eq!(fs::read(&previous).unwrap(), b"new-image");
    let _ = crate::staging::remove(root);
}

#[test]
fn reconcile_accepts_the_same_user_mod_extensions_as_import() {
    assert!(supported_mod_extension("UT_Hero_Card_103.png"));
    assert!(!supported_mod_extension("UT_Hero_Card_103.jpg"));
    assert!(!supported_mod_extension("UT_Hero_Card_103.webp"));
    assert!(!supported_mod_extension("UT_Hero_Card_103.txt"));
}

/// 模拟 Tauri invoke('set_active', { args: { heroId, sectionKey, canonical, hash } })
/// 之前缺 rename_all="camelCase" 导致 heroId/sectionKey 匹配不上 → 命令静默失败
#[test]
fn active_args_accepts_camelcase() {
    let json =
        r#"{"heroId":"103","sectionKey":"card","canonical":"UT_Hero_Card_103","hash":"abc123"}"#;
    let a: ActiveArgs = serde_json::from_str(json).expect("camelCase 反序列化失败");
    assert_eq!(a.hero_id, "103");
    assert_eq!(a.section_key, "card");
    assert_eq!(a.canonical, "UT_Hero_Card_103");
    assert_eq!(a.hash.as_deref(), Some("abc123"));
}

#[test]
fn rename_args_accepts_camelcase() {
    let json = r#"{"heroId":"103","sectionKey":"card","canonical":"UT_Hero_Card_103","hash":"abc","name":"新名字"}"#;
    let r: RenameArgs = serde_json::from_str(json).expect("camelCase 反序列化失败");
    assert_eq!(r.hero_id, "103");
    assert_eq!(r.name, "新名字");
}

/* ============ 命名规则 self-test（09-08 修正版：busts 保留战斗交互索引） ============ */

#[test]
fn parse_busts_index_kept() {
    // UT_Hero_Bust_103_0 = 原皮战斗交互1，canonical 保留完整名（modimg 按此文件名替换）
    let p = parse_filename("UT_Hero_Bust_103_0.png").unwrap();
    assert_eq!(p.section_key, "busts");
    assert_eq!(p.canonical, "UT_Hero_Bust_103_0");
}

#[test]
fn parse_busts_variant_labels() {
    assert_eq!(variant_label("UT_Hero_Bust_103_0"), "原皮");
    assert_eq!(variant_label("UT_Hero_Bust_103_1"), "原皮");
    assert_eq!(variant_label("UT_Hero_Bust_103_Max_0"), "契约皮");
    assert_eq!(variant_label("UT_Hero_Bust_103_01_0"), "皮肤一");
    assert_eq!(variant_label("UT_Hero_Bust_103_02_1"), "皮肤二");
    // 非 busts 不受影响
    assert_eq!(variant_label("UT_Hero_Card_103_01"), "皮肤一");
    assert_eq!(variant_label("UT_Hero_Card_103_Max"), "契约皮");
    assert_eq!(variant_label("UT_Hero_Card_103"), "原皮");
}

#[test]
fn parse_busts_dup_suffix_stripped() {
    // 末尾两位数字 = 解包重复副本 → 剥离
    let p = parse_filename("UT_Hero_Bust_101_01_0_02.png").unwrap();
    assert_eq!(p.canonical, "UT_Hero_Bust_101_01_0");
    let p = parse_filename("UT_Hero_Bust_101_Max_1_02.png").unwrap();
    assert_eq!(p.canonical, "UT_Hero_Bust_101_Max_1");
}

#[test]
fn parse_busts_max_kept_with_index() {
    let p = parse_filename("UT_Hero_Bust_103_Max_1.png").unwrap();
    assert_eq!(p.section_key, "busts");
    assert_eq!(p.canonical, "UT_Hero_Bust_103_Max_1");
}

#[test]
fn parse_busts_skin_with_index() {
    let p = parse_filename("UT_Hero_Bust_103_02_0.png").unwrap();
    assert_eq!(p.section_key, "busts");
    assert_eq!(p.canonical, "UT_Hero_Bust_103_02_0");
}

#[test]
fn parse_busts_no_index_rejected() {
    // 没有战斗交互索引的裸 UT_Hero_Bust_101 不是有效 busts 条目
    assert!(parse_filename("UT_Hero_Bust_101.png").is_none());
    assert!(parse_filename("UT_Hero_Bust_101_Max.png").is_none());
}

#[test]
fn parse_item_icons_strip_skin() {
    // UT_Item_Hero_101_02 不是皮肤二，item_icons 无皮肤概念
    let p = parse_filename("UT_Item_Hero_101_02.png").unwrap();
    assert_eq!(p.section_key, "item_icons");
    assert_eq!(p.canonical, "UT_Item_Hero_101");
}

#[test]
fn parse_land_platform_and_account_background() {
    let land = parse_filename("UT_Land_301.png").unwrap();
    assert_eq!(land.section_key, "land");
    assert_eq!(land.canonical, "UT_Land_301");

    let platform = parse_filename("UT_Platform_301.webp").unwrap();
    assert_eq!(platform.section_key, "platform");
    assert_eq!(platform.canonical, "UT_Platform_301");

    let card = parse_filename("UT_AccountBackground_108_03.png").unwrap();
    assert_eq!(card.section_key, "account_background");
    assert_eq!(card.canonical, "UT_AccountBackground_108_03");
}

#[test]
fn template_parser_strips_unpack_duplicates_without_creating_fake_parts() {
    assert_eq!(
        parse_filename("UT_Hero_ThinCard_101_02_02.png")
            .unwrap()
            .canonical,
        "UT_Hero_ThinCard_101_02"
    );
    assert_eq!(
        parse_filename("UT_Hero_ProfilePhoto_101_02_02.png")
            .unwrap()
            .canonical,
        "UT_Hero_ProfilePhoto_101_02"
    );
    assert_eq!(
        parse_filename("UT_Hero_RolePhoto_101_02_03.png")
            .unwrap()
            .canonical,
        "UT_Hero_RolePhoto_101_02"
    );
    assert_eq!(
        parse_filename("UT_Land_101_02.png").unwrap().canonical,
        "UT_Land_101"
    );
    assert_eq!(
        parse_filename("UT_Platform_101_02.png").unwrap().canonical,
        "UT_Platform_101"
    );
    // Collaboration heroes use the same supported id range; only ids
    // above 999 must be kept out of the hero category.
    assert!(parse_filename("UT_Hero_Card_1000_Max.png").is_none());
}

#[test]
fn hero_ids_over_999_do_not_enter_heroes() {
    assert!(parse_filename("UT_Hero_Card_1000.png").is_none());
    assert!(parse_filename("UT_Hero_Bust_1000_0.png").is_none());

    assert_eq!(
        parse_filename("UT_Hero_ProfilePhoto_1000.png")
            .unwrap()
            .section_key,
        "monster_profile"
    );

    // Non-character assets deliberately retain their own ids.
    assert_eq!(
        parse_filename("UT_Land_1000.png").unwrap().section_key,
        "land"
    );
    assert_eq!(
        parse_filename("UT_AccountBackground_71013_02.png")
            .unwrap()
            .section_key,
        "account_background"
    );
}

#[test]
fn recognised_temporary_mod_moves_its_vault_file_to_the_stable_resource_path() {
    let root = test_dir("temporary-migration");
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(2, 2, image::Rgba([12, 20, 30, 255]))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let image_bytes = encoded.into_inner();
    let hash = sha256_bytes(&image_bytes);
    let temp_id = format!("temporary:{}", hash);
    let temp_canonical = format!("unmatched:{}", hash);
    let (old_path, old_rel) = place_in_vault(
        &root,
        "heroes",
        &temp_id,
        "temporary",
        "UT_Hero_Card_101.png",
        &hash,
        ".png",
    )
    .unwrap();
    fs::write(&old_path, &image_bytes).unwrap();

    let mut lib = Library::default();
    get_resource(&mut lib, "heroes", &temp_id, "temporary", &temp_canonical)
        .files
        .push(FileEntry {
            hash: hash.clone(),
            ext: ".png".into(),
            source_name: "UT_Hero_Card_101.png".into(),
            rel_path: old_rel,
            added_at: 0,
            info: Default::default(),
            classification: Default::default(),
            fs_identity: String::new(),
        });

    assert_eq!(migrate_temporary_entries_in_vault(&mut lib, &root), 1);
    let migrated = lib.heroes["101"].sections["card"].resources["UT_Hero_Card_101"]
        .files
        .first()
        .unwrap();
    let migrated_path = root.join(&migrated.rel_path);
    assert!(migrated_path.exists());
    assert_eq!(fs::read(&migrated_path).unwrap(), image_bytes);
    assert!(!old_path.exists());
    let _ = crate::staging::remove(root);
}

#[test]
fn disabled_temporary_mod_is_not_auto_migrated() {
    let root = test_dir("temporary-disabled");
    let hash = sha256_bytes(b"disabled-temporary-image");
    let temp_id = format!("temporary:{}", hash);
    let temp_canonical = format!("unmatched:{}", hash);
    let (old_path, old_rel) = place_in_vault(
        &root,
        "heroes",
        &temp_id,
        "temporary",
        "UT_Event_30009.png.disabled",
        &hash,
        ".png",
    )
    .unwrap();
    fs::write(&old_path, b"disabled-temporary-image").unwrap();

    let mut lib = Library::default();
    let resource = get_resource(&mut lib, "heroes", &temp_id, "temporary", &temp_canonical);
    resource.disabled = true;
    resource.files.push(FileEntry {
        hash: hash.clone(),
        ext: ".png".into(),
        source_name: "UT_Event_30009.png".into(),
        rel_path: old_rel,
        added_at: 0,
        info: Default::default(),
        classification: Default::default(),
        fs_identity: String::new(),
    });

    assert_eq!(migrate_temporary_entries_in_vault(&mut lib, &root), 0);
    assert!(lib.heroes.contains_key(&temp_id));
    assert!(old_path.exists());
    assert!(!lib.heroes.contains_key("30009"));
    let _ = crate::staging::remove(root);
}

#[test]
fn temporary_mods_are_placed_in_the_top_level_temporary_dir() {
    let root = test_dir("temporary-top-dir");
    let hash = sha256_bytes(b"top-dir-image");
    let (_, rel) = place_in_vault(
        &root,
        "heroes",
        &format!("temporary:{}", hash),
        "temporary",
        "top-dir.png",
        &hash,
        ".png",
    )
    .unwrap();
    assert!(rel.starts_with("temporary/"), "rel = {rel}");
    assert!(!rel.starts_with("heroes/"));
    let _ = crate::staging::remove(root);
}

#[test]
fn layout_migration_moves_legacy_heroes_temporary_files() {
    let root = test_dir("temporary-layout-migration");
    let hash = sha256_bytes(b"layout-migration-image");
    let temp_id = format!("temporary:{}", hash);
    let temp_canonical = format!("unmatched:{}", hash);
    let old_rel = format!("heroes/{}/temporary/legacy.png", sanitize_name(&temp_id));
    let from = root.join(&old_rel);
    ensure_dir(from.parent().unwrap()).unwrap();
    fs::write(&from, b"layout-migration-image").unwrap();

    let mut lib = Library::default();
    get_resource(&mut lib, "heroes", &temp_id, "temporary", &temp_canonical)
        .files
        .push(FileEntry {
            hash: hash.clone(),
            ext: ".png".into(),
            source_name: "legacy.png".into(),
            rel_path: old_rel.clone(),
            added_at: 0,
            info: Default::default(),
            classification: Default::default(),
            fs_identity: String::new(),
        });

    assert_eq!(migrate_temporary_vault_layout(&mut lib, &root), 1);
    let entry = &lib.heroes[&temp_id].sections["temporary"].resources[&temp_canonical].files[0];
    assert!(
        entry.rel_path.starts_with("temporary/"),
        "rel = {}",
        entry.rel_path
    );
    assert!(root.join(&entry.rel_path).exists());
    assert!(!from.exists());
    // 幂等：再次迁移不再有动作。
    assert_eq!(migrate_temporary_vault_layout(&mut lib, &root), 0);
    let _ = crate::staging::remove(root);
}

#[test]
fn template_requires_complete_scanned_variant_but_accepts_user_import() {
    let id = "133";
    let mut res = ScanResult::default();
    let mut lib = Library::default();

    // 单独扫描到一张 Card 不能凭空生成皮肤三。
    res.manifest
        .entry(id.into())
        .or_default()
        .entry("card".into())
        .or_default()
        .insert(
            "UT_Hero_Card_133_03".into(),
            ManifestEntry {
                files: vec![],
                dir: String::new(),
            },
        );
    assert!(!hero_template_variants(&res, &lib, id, false).contains("03"));

    // 用户明确导入同一个 canonical 后，立即生成可编辑的动态模板。
    get_resource(&mut lib, "heroes", id, "card", "UT_Hero_Card_133_03");
    assert!(hero_template_variants(&res, &lib, id, false).contains("03"));
    let rows = hero_template_canonicals(&res, &lib, id, false);
    assert!(rows["busts"].contains("UT_Hero_Bust_133_03_0"));
}

#[test]
fn account_background_does_not_create_hero_template() {
    let id = "71013";
    let mut res = ScanResult::default();
    res.manifest
        .entry(id.into())
        .or_default()
        .entry("account_background".into())
        .or_default()
        .insert(
            "UT_AccountBackground_71013_02".into(),
            ManifestEntry {
                files: vec![],
                dir: String::new(),
            },
        );
    assert!(hero_template_variants(&res, &Library::default(), id, false).is_empty());
}

#[test]
fn parse_thin_card_uses_new_key() {
    let p = parse_filename("UT_Hero_ThinCard_101_01.png").unwrap();
    assert_eq!(p.section_key, "thin_card");
    assert_eq!(p.canonical, "UT_Hero_ThinCard_101_01");
}

#[test]
fn parse_thin_card_legacy_key_absent() {
    // 已改名为 thin_card，card_thin 不应再出现
    for s in section_defs() {
        assert_ne!(s.key, "card_thin", "card_thin 已重命名为 thin_card");
    }
}

#[test]
fn card_assets_are_classified_into_cards_sections() {
    for (name, section, id) in [
        ("UT_Event_30009.png", "event_card", "30009"),
        ("UT_Event_30014.png", "event_card", "30014"),
        ("UT_HandCard_20005.png", "hand_card", "20005"),
        ("UT_HandCard_20006.png", "hand_card", "20006"),
        ("UT_HandCard_20030.png", "hand_card", "20030"),
        ("UT_HandCard_21003.png", "hand_card", "21003"),
        ("UT_MapEvent_32001.png", "map_event", "32001"),
    ] {
        let parsed = parse_filename(name).unwrap_or_else(|| panic!("{} should parse", name));
        assert_eq!(parsed.section_key, section, "{} section", name);
        assert_eq!(parsed.hero_id, id, "{} id", name);
    }
}

#[test]
fn encrypted_package_rejects_wrong_password_and_tampering() {
    let encrypted = encrypt_package(b"package payload", "correct horse battery staple").unwrap();
    assert_eq!(&encrypted[..8], PACKAGE_MAGIC_V2);
    assert_eq!(
        decrypt_package(&encrypted, "correct horse battery staple").unwrap(),
        b"package payload"
    );
    assert!(decrypt_package(&encrypted, "wrong password").is_err());
    let mut tampered = encrypted;
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    assert!(decrypt_package(&tampered, "correct horse battery staple").is_err());

    let mut header_tampered =
        encrypt_package(b"package payload", "correct horse battery staple").unwrap();
    header_tampered[21] ^= 1; // salt is authenticated as v2 AAD
    assert!(decrypt_package(&header_tampered, "correct horse battery staple").is_err());
}

#[test]
fn package_password_profiles_have_expected_defaults_and_bounds() {
    assert_eq!(package_kdf_profile(Some("compat")).unwrap(), (19_456, 2, 1));
    assert_eq!(package_kdf_profile(None).unwrap(), (65_536, 3, 1));
    assert_eq!(package_kdf_profile(Some("high")).unwrap(), (131_072, 4, 1));
    assert!(package_kdf_profile(Some("unsupported")).is_err());

    for profile in ["compat", "standard", "high"] {
        let (memory, iterations, parallelism) = package_kdf_profile(Some(profile)).unwrap();
        let encrypted = encrypt_package_with_params(
            b"profile payload",
            "password",
            memory,
            iterations,
            parallelism,
        )
        .unwrap();
        assert_eq!(
            decrypt_package(&encrypted, "password").unwrap(),
            b"profile payload"
        );
    }
}

#[test]
fn quick_export_paths_are_classified_and_collision_safe() {
    let hash = "0123456789abcdef";
    let rel = quick_export_relative_path("heroes", "101", "busts", "My Mod.png", hash);
    assert_eq!(
        rel,
        PathBuf::from("heroes")
            .join("101")
            .join("busts")
            .join("My Mod_01234567.png")
    );
    let root = test_dir("quick-export-path");
    let first = unique_quick_export_path(&root, &rel);
    assert_eq!(first, root.join(&rel));
    fs::create_dir_all(first.parent().unwrap()).unwrap();
    fs::write(&first, b"old").unwrap();
    assert_ne!(unique_quick_export_path(&root, &rel), first);
    let _ = crate::staging::remove(root);
}

#[test]
fn mod_scan_cache_reuses_unchanged_file_and_refreshes_changed_file() {
    let root = test_dir("scan-cache");
    let source = root.join("mod.png");
    fs::write(&source, b"first").unwrap();
    let mut cache = ModScanCache::default();
    let first = cached_file_hash(&source, &mut cache).unwrap();
    let cached = cached_file_hash(&source, &mut cache).unwrap();
    assert_eq!(first, cached);
    // A size change is part of the cache fingerprint and must force a
    // fresh hash even on file systems with coarse timestamp resolution.
    fs::write(&source, b"changed-content").unwrap();
    let changed = cached_file_hash(&source, &mut cache).unwrap();
    assert_ne!(first, changed);
    let _ = crate::staging::remove(root);
}

#[test]
fn scan_cache_migrates_absolute_keys_to_modimg_relative_keys() {
    let root = test_dir("scan-cache-key");
    let modimg = root.join("modimg");
    fs::create_dir_all(&modimg).unwrap();
    let file = modimg.join("UT_Hero_Card_101.png");
    fs::write(&file, b"image").unwrap();
    let mut cache = ModScanCache::default();
    let old_key = file.to_string_lossy().to_string();
    cached_file_hash(&file, &mut cache).unwrap();
    migrate_scan_cache_keys(&mut cache, &modimg);
    assert!(!cache.entries.contains_key(&old_key));
    assert!(cache.entries.contains_key("UT_Hero_Card_101.png"));
    let _ = crate::staging::remove(root);
}

#[test]
fn invalid_package_mapping_is_previewed_as_temporary() {
    let meta = PackageFileMeta {
        category: "unknown".into(),
        entity_id: String::new(),
        section_key: "missing".into(),
        canonical: String::new(),
        hash: "a".repeat(64),
        ext: ".png".into(),
        source_name: "custom.png".into(),
        was_enabled: false,
        storage_name: None,
        entity_name: String::new(),
        section_name: String::new(),
        resource_name: String::new(),
        category_name: String::new(),
        category_order: 4,
        preview_url: None,
        provenance: None,
        manual_classification: false,
    };
    let normalized = normalized_package_meta(&meta);
    assert_eq!(normalized.category, "heroes");
    assert_eq!(normalized.section_key, "temporary");
    assert!(normalized.entity_id.starts_with("temporary:"));
}

#[test]
fn package_import_uses_current_filename_rules_before_legacy_mapping() {
    for (source_name, section_key, entity_id, canonical) in [
        (
            "UT_Event_30009.png",
            "event_card",
            "30009",
            "UT_Event_30009",
        ),
        (
            "UT_HandCard_20005.png",
            "hand_card",
            "20005",
            "UT_HandCard_20005",
        ),
        (
            "UT_MapEvent_32001.png",
            "map_event",
            "32001",
            "UT_MapEvent_32001",
        ),
    ] {
        let legacy = PackageFileMeta {
            // This deliberately looks valid, proving sourceName takes
            // precedence over stale but structurally valid package keys.
            category: "heroes".into(),
            entity_id: "101".into(),
            section_key: "card".into(),
            canonical: "UT_Hero_Card_101".into(),
            hash: "f".repeat(64),
            ext: ".png".into(),
            source_name: source_name.into(),
            was_enabled: false,
            storage_name: None,
            entity_name: String::new(),
            section_name: String::new(),
            resource_name: String::new(),
            category_name: String::new(),
            category_order: 0,
            preview_url: None,
            provenance: None,
            manual_classification: false,
        };
        let normalized = normalized_package_meta(&legacy);
        assert_eq!(normalized.category, "cards", "{} category", source_name);
        assert_eq!(
            normalized.section_key, section_key,
            "{} section",
            source_name
        );
        assert_eq!(normalized.entity_id, entity_id, "{} id", source_name);
        assert_eq!(normalized.canonical, canonical, "{} canonical", source_name);
    }
}

#[test]
fn import_preview_uses_current_local_names_not_manifest_labels() {
    let mut lib = Library::default();
    lib.settings
        .display_names
        .insert("101".into(), "本机自定义角色".into());
    let package_meta = PackageFileMeta {
        category: "heroes".into(),
        entity_id: "101".into(),
        section_key: "busts".into(),
        canonical: "UT_Hero_Bust_101_0".into(),
        hash: "b".repeat(64),
        ext: ".png".into(),
        source_name: "exported.png".into(),
        was_enabled: false,
        storage_name: None,
        entity_name: "旧包角色名".into(),
        section_name: "旧包部位名".into(),
        resource_name: "旧包资源名".into(),
        category_name: String::new(),
        category_order: 0,
        preview_url: Some("C:/not-exposed.png".into()),
        provenance: None,
        manual_classification: false,
    };
    let localized = localize_package_meta(&lib, normalized_package_meta(&package_meta));
    assert_eq!(localized.entity_name, "本机自定义角色");
    assert_eq!(localized.section_name, "战斗交互");
    assert_eq!(localized.resource_name, "原皮");
    assert!(localized.preview_url.is_none());
}

#[test]
fn import_preview_localizes_cards_monsters_and_temporary_items() {
    let lib = Library::default();
    let card = PackageFileMeta {
        category: "cards".into(),
        entity_id: "20005".into(),
        section_key: "hand_card".into(),
        canonical: "UT_HandCard_20005".into(),
        hash: "c".repeat(64),
        ext: ".png".into(),
        source_name: "card.png".into(),
        was_enabled: false,
        storage_name: None,
        entity_name: String::new(),
        section_name: String::new(),
        resource_name: String::new(),
        category_name: String::new(),
        category_order: 1,
        preview_url: None,
        provenance: None,
        manual_classification: false,
    };
    let card = localize_package_meta(&lib, normalized_package_meta(&card));
    assert_eq!(card.entity_name, "遥控骰子");
    assert_eq!(card.section_name, "手牌");
    assert_eq!(card.resource_name, "遥控骰子");

    let monster = PackageFileMeta {
        category: "monsters".into(),
        entity_id: "1001".into(),
        section_key: "monster_card".into(),
        canonical: "UT_Monster_Card_1001".into(),
        hash: "e".repeat(64),
        ext: ".png".into(),
        source_name: "monster.png".into(),
        was_enabled: false,
        storage_name: None,
        entity_name: "错误怪物名".into(),
        section_name: String::new(),
        resource_name: String::new(),
        category_name: String::new(),
        category_order: 0,
        preview_url: None,
        provenance: None,
        manual_classification: false,
    };
    let monster = localize_package_meta(&lib, normalized_package_meta(&monster));
    assert_eq!(monster.entity_name, "小偷");
    assert_eq!(monster.section_name, "大立绘");

    let temporary = PackageFileMeta {
        category: "unknown".into(),
        entity_id: String::new(),
        section_key: "missing".into(),
        canonical: String::new(),
        hash: "d".repeat(64),
        ext: ".png".into(),
        source_name: "unknown.png".into(),
        was_enabled: false,
        storage_name: None,
        entity_name: "错误名称".into(),
        section_name: "错误部位".into(),
        resource_name: "错误资源".into(),
        category_name: String::new(),
        category_order: 0,
        preview_url: None,
        provenance: None,
        manual_classification: false,
    };
    let temporary = localize_package_meta(&lib, normalized_package_meta(&temporary));
    assert_eq!(temporary.entity_name, "临时");
    assert_eq!(temporary.section_name, "临时");
    assert_eq!(temporary.resource_name, "临时资源");
    assert!(temporary.entity_id.starts_with("temporary:"));
}

#[test]
fn disabled_state_is_rebuilt_from_the_parked_game_file() {
    let root = test_dir("disabled-sync");
    let mut lib = Library::default();
    lib.settings.modimg_path = root.to_string_lossy().to_string();
    let resource = get_resource(&mut lib, "heroes", "101", "card", "UT_Hero_Card_101");
    resource.active_hash = Some("hash".into());
    resource.files.push(FileEntry {
        hash: "hash".into(),
        ext: ".png".into(),
        source_name: "a.png".into(),
        rel_path: String::new(),
        added_at: 0,
        info: Default::default(),
        classification: Default::default(),
        fs_identity: String::new(),
    });
    fs::write(root.join("UT_Hero_Card_101.png.disabled"), b"parked").unwrap();
    sync_disabled_state_from_modimg(&mut lib);
    assert!(lib.heroes["101"].sections["card"].resources["UT_Hero_Card_101"].disabled);
    let _ = crate::staging::remove(root);
}

#[test]
fn temporary_external_presence_is_separate_from_active_slot() {
    let mut resource = ResourceEntry::default();
    assert!(!resource.external_present);
    assert!(resource.active_hash.is_none());
    resource.external_present = true;
    assert!(resource.external_present);
    assert!(resource.active_hash.is_none());
}

/* ============ 插件库（APX 补丁） ============ */

#[test]
fn plugin_paths_are_rejected_when_they_escape_the_target_root() {
    assert!(safe_relative_path("com.unity.addressables/x/__data").is_some());
    assert_eq!(
        safe_relative_path("dependence\\sub\\file.dll"),
        Some(PathBuf::from("dependence").join("sub").join("file.dll"))
    );
    assert!(safe_relative_path("").is_none());
    assert!(safe_relative_path("   ").is_none());
    assert!(safe_relative_path("/etc/passwd").is_none());
    assert!(safe_relative_path("C:/Windows/System32/x.dll").is_none());
    assert!(safe_relative_path("../../outside.dll").is_none());
    assert!(safe_relative_path("a/../../b.dll").is_none());
    assert!(safe_relative_path("./a/./b.dll").is_some());
}

#[test]
fn plugin_paths_trim_launcher_whitespace_artifacts() {
    assert_eq!(
        safe_relative_path("Plugins\\x86_64\\APXLive2DBridge.dll&#x20; \n"),
        Some(
            PathBuf::from("Plugins")
                .join("x86_64")
                .join("APXLive2DBridge.dll")
        )
    );
    assert_eq!(
        normalize_windows_path("E:\\game\\Astral Party\\8vJXn6CN\\&#x20; \n"),
        PathBuf::from("E:\\game\\Astral Party\\8vJXn6CN\\")
    );
}

#[test]
fn permission_denied_plugin_writes_have_actionable_message() {
    let error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    let message = friendly_plugin_write_error(Path::new("C:\\game\\Plugins\\bridge.dll"), &error);
    assert!(message.contains("关闭游戏后重试"));
    assert!(message.contains("写入权限"));
}

#[test]
fn plugin_url_follows_the_repository_layout() {
    assert_eq!(
        plugin_manifest_url("CN_STEAM"),
        "https://cnb.cool/erinilis/apx-plugin-releases/-/git/raw/main/CN_STEAM/manifest.json"
    );
    assert_eq!(
            plugin_file_url("CN_TAPTAP", "dependence/APXLive2DBridge.dll"),
            "https://cnb.cool/erinilis/apx-plugin-releases/-/git/raw/main/CN_TAPTAP/dependence/APXLive2DBridge.dll"
        );
    assert_eq!(
        plugin_manifest_url("INT_STEAM"),
        plugin_manifest_url("CN_STEAM")
    );
    assert_eq!(
        plugin_file_url("INT_STEAM", "dependence/x.dll"),
        plugin_file_url("CN_STEAM", "dependence/x.dll")
    );
    // 路径分隔符与需要转义的字符都必须编码，斜杠保持不变。
    assert_eq!(encode_uri_path("a b/c\\d.dll"), "a%20b/c/d.dll");
    assert_eq!(encode_uri_path("中文/x.bin"), "%E4%B8%AD%E6%96%87/x.bin");
}

#[test]
fn plugin_platforms_map_locallow_and_data_directory_names() {
    let steam = plugin_platform_info("CN_STEAM").unwrap();
    assert_eq!(steam.2, "AstralParty_CN");
    assert_eq!(steam.3, "AstralParty_CN_Data");
    for key in ["CN_TAPTAP", "CN_BILIBILI", "CN_WEGAME"] {
        let info = plugin_platform_info(key).unwrap();
        assert_eq!(info.2, "吉星派对", "{} LocalLow 目录", key);
        assert_eq!(info.3, "吉星派对_Data", "{} Data 目录", key);
    }
    assert!(plugin_platform_info("CN_UNKNOWN").is_none());
    assert_eq!(plugin_platform_options().len(), 6);
}

#[test]
fn plugin_plugins_directory_decides_between_unity_data_and_game_root() {
    assert!(plugin_uses_unity_data_dir(
        "Plugins/x86_64/APXLive2DBridge.dll"
    ));
    assert!(plugin_uses_unity_data_dir(
        "Plugins\\x86_64\\Live2DCubismCore.dll"
    ));
    assert!(plugin_uses_unity_data_dir("a/Plugins/x86_64/x.dll"));
    assert!(plugin_uses_unity_data_dir(
        "com.unity.addressables/AssetBundles/a/__data"
    ));
    assert!(plugin_uses_unity_data_dir(
        "com.unity.addressables\\AssetBundles\\a\\__data"
    ));
    assert!(!plugin_uses_unity_data_dir(
        "dependence/apx_modern_assist_ui.bundle"
    ));
    assert!(!plugin_uses_unity_data_dir(
        "live2d/live2d_runtime_resources.bundle"
    ));
}

#[test]
fn plugin_addressables_path_is_distinguished_from_other_game_root_files() {
    assert!(plugin_is_addressables_path(
        "com.unity.addressables/AssetBundles/a/__data"
    ));
    assert!(plugin_is_addressables_path(
        "com.unity.addressables\\AssetBundles\\a\\__info"
    ));
    assert!(!plugin_is_addressables_path(
        "dependence/com.unity.addressables.bundle"
    ));
    assert!(!plugin_is_addressables_path(
        "game/com.unity.addressables/AssetBundles/a"
    ));
}

#[test]
fn plugin_game_version_is_read_from_the_newest_catalog_hash() {
    let root = test_dir("plugin-catalog");
    let addressables = root.join("com.unity.addressables");
    fs::create_dir_all(&addressables).unwrap();
    fs::write(addressables.join("catalog_2026-08-01-000000.hash"), "3.1.0").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(addressables.join("catalog_2026-09-16-133113.hash"), "3.2.0").unwrap();
    // 名称不是 catalog_*.hash 的文件不能参与判定
    fs::write(addressables.join("catalog_backup.txt"), "9.9.9").unwrap();

    let mut newest: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in fs::read_dir(&addressables).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.starts_with("catalog_") && name.ends_with(".hash") {
            newest.push((entry.metadata().unwrap().modified().unwrap(), entry.path()));
        }
    }
    newest.sort_by(|a, b| b.0.cmp(&a.0));
    assert_eq!(newest.len(), 2);
    let text = fs::read_to_string(&newest[0].1).unwrap();
    assert_eq!(extract_version(text.trim()).as_deref(), Some("3.2.0"));

    // 内容不是版本号时，退回到文件名里的版本号
    assert_eq!(
        extract_version("catalog_3.2.0.hash").as_deref(),
        Some("3.2.0")
    );
    assert_eq!(extract_version("4f2a9c1e8b7d6a5f"), None);
    let _ = crate::staging::remove(root);
}

#[test]
fn plugin_manifest_matches_the_published_repository_schema() {
    let json = r#"{
          "format_version": "2.0",
          "platform": "CN_STEAM",
          "version": "3.2.0",
          "game_version": "3.2.0",
          "generated_at": "2026-09-16T13:31:13+08:00",
          "total_patch_files": 3,
          "total_patch_size": 30,
          "total_restore_files": 1,
          "total_restore_size": 10,
          "files": [
            {"relative_path":"com.unity.addressables/AssetBundles/a/b/__data","target_type":"locallow","target_path":"com.unity.addressables/AssetBundles/a/b/__data","sha256":"aa","size":10,"description":"缓存"},
            {"relative_path":"dependence/APXLive2DBridge.dll","target_type":"game_root","target_path":"Plugins/x86_64/APXLive2DBridge.dll","sha256":"bb","size":10,"description":"原生插件","locallow_target_path":"Plugins/x86_64/APXLive2DBridge.dll"},
            {"relative_path":"dependence/apx_modern_assist_ui.bundle","target_type":"game_root","target_path":"dependence/apx_modern_assist_ui.bundle","sha256":"cc","size":10,"description":"依赖包"}
          ],
          "restore_files": [
            {"relative_path":"restore/com.unity.addressables/AssetBundles/a/b/__data","target_type":"locallow","target_path":"com.unity.addressables/AssetBundles/a/b/__data","sha256":"dd","size":10,"description":"官方原版"}
          ]
        }"#;
    let manifest: PluginManifest = serde_json::from_str(json).expect("清单反序列化失败");
    assert_eq!(manifest.format_version, "2.0");
    assert_eq!(manifest.platform, "CN_STEAM");
    assert_eq!(manifest.game_version, "3.2.0");
    assert_eq!(manifest.files.len(), manifest.total_patch_files);
    assert_eq!(manifest.restore_files.len(), manifest.total_restore_files);
    assert_eq!(
        manifest.files.iter().map(|f| f.size).sum::<u64>(),
        manifest.total_patch_size
    );
    assert_eq!(
        manifest.files[1].target_path,
        "Plugins/x86_64/APXLive2DBridge.dll"
    );
    assert_eq!(manifest.files[0].target_type, "locallow");
    // restore 条目的 target_path 已去掉 restore/ 前缀，落点与补丁文件一致。
    assert_eq!(
        manifest.restore_files[0].target_path,
        manifest.files[0].target_path
    );
    assert_eq!(manifest.files.len(), 3);
}

#[test]
fn runtime_dependencies_json_accepts_semantic_manifest_when_hash_is_stale() {
    let json = br#"{
          "format_version":"1.7",
          "platform":"CN_STEAM",
          "main_assembly":"AstralParty.Runtime.dll",
          "managed_assemblies_embedded_in_main_assembly":[],
          "managed_assemblies_embedded_in_assembly_bundle":["AstralParty.Runtime.dll"],
          "native_plugins_require_manual_deployment":["dependence/APXLive2DBridge.dll"],
          "native_plugin_directories":["dependence"],
          "native_plugin_directory":"dependence",
          "dynamic_driver":{"file":"APX.dynamic.portraits.dll","source_sha256":"a","sha256":"b","size":1}
        }"#;
    assert!(validate_runtime_dependencies_json(json, "CN_STEAM").is_ok());
}

#[test]
fn runtime_dependencies_json_rejects_wrong_platform_and_unsafe_paths() {
    let wrong_platform =
        br#"{"format_version":"1","main_assembly":"x.dll","platform":"CN_TAPTAP"}"#;
    assert!(validate_runtime_dependencies_json(wrong_platform, "CN_STEAM").is_err());
    let unsafe_path = br#"{"format_version":"1","main_assembly":"x.dll","native_plugin_directories":["../outside"]}"#;
    assert!(validate_runtime_dependencies_json(unsafe_path, "CN_STEAM").is_err());
}

#[test]
fn runtime_dependencies_json_allows_cn_steam_metadata_for_int_steam_only() {
    let json = br#"{"format_version":"1","main_assembly":"x.dll","platform":"CN_STEAM"}"#;
    assert!(validate_runtime_dependencies_json(json, "INT_STEAM").is_ok());
    assert!(validate_runtime_dependencies_json(json, "CN_TAPTAP").is_err());
}

#[test]
fn plugin_status_reports_installed_and_pending_files() {
    let status = plugin_status_inner("CN_STEAM");
    assert_eq!(status.platform, "CN_STEAM");
    assert_eq!(status.platform_label, "Steam 国服");
    assert_eq!(status.platforms.len(), 6);
    // 没有缓存清单时不应崩溃，也不应伪造远程版本
    assert_eq!(status.total_files, status.files.len());
    assert!(status.installed_files <= status.total_files);
    assert!(status.warning.is_none() || status.version_match == Some(false));
}
