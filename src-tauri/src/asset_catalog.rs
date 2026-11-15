//! 从旧版迁入的领域实现；保留协议与行为，在明确边界内逐步替换。
use crate::*;

/* ============ manifest 缓存 ============ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ManifestEntry {
    pub(crate) files: Vec<String>,
    pub(crate) dir: String,
}

pub(crate) type ManifestMap = HashMap<String, HashMap<String, HashMap<String, ManifestEntry>>>;

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct ScanResult {
    pub(crate) heroes: Vec<String>,
    pub(crate) all_ids: Vec<String>,
    #[serde(skip)]
    pub(crate) manifest: ManifestMap,
}

pub(crate) struct AppState {
    pub(crate) scan: Mutex<Option<ScanResult>>,
    /// Serializes every library/modimg mutation so the persisted state cannot
    /// race the file that Astral Party actually reads.
    pub(crate) mutation: Arc<Mutex<()>>,
    pub(crate) scan_busy: Arc<AtomicBool>,
    pub(crate) preview_busy: Arc<AtomicBool>,
    pub(crate) preview_cancel: Arc<AtomicBool>,
    /// 插件库安装/还原的取消开关，与预览下载互不影响。
    pub(crate) plugin_cancel: Arc<AtomicBool>,
    pub(crate) apx_cache_busy: Arc<Mutex<()>>,
    /// 软件更新通道（CNB 主源）的忙/取消开关。
    pub(crate) update_busy: Arc<AtomicBool>,
    pub(crate) update_cancel: Arc<AtomicBool>,
    pub(crate) import_queue: Arc<Mutex<Option<PackageImportQueue>>>,
}

pub(crate) fn scan_assets(root: &str) -> ScanResult {
    let mut manifest: ManifestMap = HashMap::new();
    let mut all_ids: Vec<String> = Vec::new();
    let mut bust_ids: Vec<String> = Vec::new();
    let mut all_id_set = HashSet::new();
    let mut bust_id_set = HashSet::new();

    for s in section_defs() {
        /* 兼容选择 images_by_part 根目录，或其 heroes 子目录。 */
        let root_path = Path::new(root);
        let hero_root = root_path.join("heroes");
        let parent_path = root_path.parent().unwrap_or(root_path);
        let mut candidates = vec![
            (root_path.join(&s.asset_dir), s.asset_dir.clone()),
            (
                hero_root.join(&s.asset_dir),
                format!("heroes/{}", s.asset_dir),
            ),
            (
                parent_path.join(&s.asset_dir),
                format!("../{}", s.asset_dir),
            ),
        ];
        if s.category == "heroes" {
            candidates.extend([
                (hero_root.join("test"), "heroes/test".into()),
                (root_path.join("test"), "test".into()),
                (
                    parent_path.join("heroes").join("test"),
                    "../heroes/test".into(),
                ),
                (parent_path.join("test"), "../test".into()),
            ]);
        }
        let mut seen_dirs = HashSet::new();
        for (dir, manifest_dir) in candidates
            .into_iter()
            .filter(|(p, _)| p.is_dir())
            .filter(|(p, _)| seen_dirs.insert(p.clone()))
        {
            let entries = match fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if !(fname.ends_with(".png")
                    || fname.ends_with(".jpg")
                    || fname.ends_with(".jpeg")
                    || fname.ends_with(".webp"))
                {
                    continue;
                }
                if let Some(p) = parse_filename(&fname) {
                    // 只有 heroes 大类的 ID 才进角色列表；cards 等大类的 ID 是卡号，不能混进角色列表
                    // 名片的主体编号并不保证是可选角色，不能仅凭名片把它塞进角色列表。
                    if s.category == "heroes"
                        && s.key != "account_background"
                        && p.hero_id
                            .parse::<u32>()
                            .map(|id| id <= 999)
                            .unwrap_or(false)
                    {
                        if all_id_set.insert(p.hero_id.clone()) {
                            all_ids.push(p.hero_id.clone());
                        }
                        if s.key == "busts" && bust_id_set.insert(p.hero_id.clone()) {
                            bust_ids.push(p.hero_id.clone());
                        }
                    }
                    let records = manifest
                        .entry(p.hero_id.clone())
                        .or_default()
                        .entry(s.key.clone())
                        .or_default();
                    match records.entry(p.canonical.clone()) {
                        std::collections::hash_map::Entry::Vacant(slot) => {
                            slot.insert(ManifestEntry {
                                files: vec![fname],
                                dir: manifest_dir.clone(),
                            });
                        }
                        std::collections::hash_map::Entry::Occupied(mut slot) => {
                            if slot.get().dir == manifest_dir {
                                slot.get_mut().files.push(fname);
                            }
                        }
                    }
                }
            }
        }
    }

    all_ids.sort_by(|a, b| a.cmp(b));
    bust_ids.sort_by(|a, b| a.cmp(b));
    for (_h, secs) in manifest.iter_mut() {
        for (_s, cans) in secs.iter_mut() {
            for (_c, e) in cans.iter_mut() {
                e.files.sort();
            }
        }
    }

    ScanResult {
        heroes: bust_ids,
        all_ids,
        manifest,
    }
}

pub(crate) fn source_canonicals(res: &ScanResult, hero_id: &str, section: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Some(entries) = res
        .manifest
        .get(hero_id)
        .and_then(|sections| sections.get(section))
    {
        out.extend(entries.keys().cloned());
    }
    out.extend(manifest_canonicals(hero_id, section));
    out
}

pub(crate) fn source_has_canonical(
    res: &ScanResult,
    hero_id: &str,
    section: &str,
    canonical: &str,
) -> bool {
    res.manifest
        .get(hero_id)
        .and_then(|sections| sections.get(section))
        .map(|entries| entries.contains_key(canonical))
        .unwrap_or(false)
        || manifest_canonicals(hero_id, section)
            .iter()
            .any(|candidate| candidate == canonical)
}

pub(crate) fn source_template_is_complete(res: &ScanResult, hero_id: &str, variant: &str) -> bool {
    required_template_resources(hero_id, variant)
        .iter()
        .all(|(section, canonical)| source_has_canonical(res, hero_id, section, canonical))
}

pub(crate) fn library_variants(lib: &Library, hero_id: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let Some(hero) = lib.heroes.get(hero_id) else {
        return out;
    };
    for (section, resources) in &hero.sections {
        if section == "account_background" {
            continue;
        }
        for canonical in resources.resources.keys() {
            if canonicalize_for_section(canonical, section, false).as_deref()
                == Some(canonical.as_str())
            {
                out.insert(template_variant(canonical));
            }
        }
    }
    out
}

/// 官方扫描结果只有凑齐整个模板才被采纳；素材库中的条目代表用户明确导入，
/// 即使只导入了一个部位也会生成该变体的可编辑模板。
pub(crate) fn hero_template_variants(
    res: &ScanResult,
    lib: &Library,
    hero_id: &str,
    include_builtin_base: bool,
) -> HashSet<String> {
    let mut candidates = HashSet::new();
    for section in sections_for_category("heroes") {
        if section.key == "account_background" {
            continue;
        }
        candidates.extend(
            source_canonicals(res, hero_id, &section.key)
                .into_iter()
                .map(|c| template_variant(&c)),
        );
    }

    let mut accepted: HashSet<String> = candidates
        .into_iter()
        .filter(|variant| {
            is_valid_template_variant(hero_id, variant)
                && source_template_is_complete(res, hero_id, variant)
        })
        .collect();
    if include_builtin_base {
        accepted.insert("base".into());
    }
    accepted.extend(library_variants(lib, hero_id));
    accepted
}

pub(crate) fn hero_template_canonicals(
    res: &ScanResult,
    lib: &Library,
    hero_id: &str,
    include_builtin_base: bool,
) -> HashMap<String, HashSet<String>> {
    let variants = hero_template_variants(res, lib, hero_id, include_builtin_base);
    let mut by_section: HashMap<String, HashSet<String>> = HashMap::new();
    for variant in &variants {
        for (section, canonical) in required_template_resources(hero_id, variant) {
            by_section.entry(section).or_default().insert(canonical);
        }
    }

    // 局内卡是特殊额外资源；只有当前官方变体已被认可或用户明确导入时才显示。
    let lib_hero = lib.heroes.get(hero_id);
    let optional = source_canonicals(res, hero_id, "card_in_game");
    for canonical in optional {
        let variant = template_variant(&canonical);
        if variants.contains(&variant) {
            by_section
                .entry("card_in_game".into())
                .or_default()
                .insert(canonical);
        }
    }
    if let Some(hero) = lib_hero {
        if let Some(section) = hero.sections.get("card_in_game") {
            by_section.entry("card_in_game".into()).or_default().extend(
                section
                    .resources
                    .keys()
                    .filter(|canonical| {
                        canonicalize_for_section(canonical, "card_in_game", false).as_deref()
                            == Some(canonical.as_str())
                    })
                    .cloned(),
            );
        }
    }

    // 名片不属于任何皮肤，只按真实发现或用户库中已有编号动态增加。
    by_section
        .entry("account_background".into())
        .or_default()
        .extend(source_canonicals(res, hero_id, "account_background"));
    if let Some(hero) = lib_hero {
        if let Some(section) = hero.sections.get("account_background") {
            by_section
                .entry("account_background".into())
                .or_default()
                .extend(
                    section
                        .resources
                        .keys()
                        .filter(|canonical| {
                            canonicalize_for_section(canonical, "account_background", false)
                                .as_deref()
                                == Some(canonical.as_str())
                        })
                        .cloned(),
                );
        }
    }
    by_section
}
