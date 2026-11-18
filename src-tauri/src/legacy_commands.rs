//! 从旧版迁入的领域实现；保留协议与行为，在明确边界内逐步替换。
use crate::*;

/* ============ Commands ============ */

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn detect(state: tauri::State<AppState>) -> Result<DetectResult, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    match find_game() {
        Some((gp, _)) => {
            let (status, m) = ensure_modimg(&gp)?;
            let mut lib = load_library();
            lib.settings.modimg_path = m.to_string_lossy().to_string();
            save_library(&lib)?;
            Ok(DetectResult {
                status,
                game_path: gp.to_string_lossy().to_string(),
                modimg_path: m.to_string_lossy().to_string(),
            })
        }
        None => Ok(DetectResult {
            status: "notfound".into(),
            game_path: String::new(),
            modimg_path: String::new(),
        }),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn resolve_manual(
    dir: String,
    state: tauri::State<AppState>,
) -> Result<DetectResult, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let p = PathBuf::from(&dir);
    let gp = if p
        .file_name()
        .map(|n| n.to_string_lossy().eq_ignore_ascii_case("Astral Party"))
        .unwrap_or(false)
    {
        let subs: Vec<String> = fs::read_dir(&p)
            .map_err(|e| e.to_string())?
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        let pick = subs
            .iter()
            .find(|n| n.eq_ignore_ascii_case("8vJXn6CN"))
            .cloned()
            .unwrap_or_else(|| subs.first().cloned().unwrap_or_default());
        p.join(pick)
    } else {
        p
    };
    let (status, m) = ensure_modimg(&gp)?;
    let mut lib = load_library();
    lib.settings.modimg_path = m.to_string_lossy().to_string();
    save_library(&lib)?;
    Ok(DetectResult {
        status,
        game_path: gp.to_string_lossy().to_string(),
        modimg_path: m.to_string_lossy().to_string(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn open_modimg(state: tauri::State<AppState>) -> Result<(), String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let p = load_library().settings.modimg_path;
    if !p.is_empty() && Path::new(&p).exists() {
        let _ = std::process::Command::new("explorer").arg(&p).spawn();
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanOut {
    pub(crate) heroes: Vec<String>,
    pub(crate) all_ids: Vec<String>,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn asset_scan(
    root: Option<String>,
    state: tauri::State<AppState>,
) -> Result<ScanOut, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    if let Some(r) = root {
        if !r.is_empty() {
            lib.settings.asset_root = r;
            save_library(&lib)?;
        }
    }
    let res = scan_assets(&lib.settings.asset_root);
    let out = ScanOut {
        heroes: res.heroes.clone(),
        all_ids: res.all_ids.clone(),
    };
    *state.scan.lock().unwrap() = Some(res);
    Ok(out)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HeroItem {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) title: Option<String>,
    pub(crate) avatar: Option<String>,
    pub(crate) active_count: usize,
    pub(crate) mod_count: usize,
    pub(crate) has_mod: bool,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn heroes_list(state: tauri::State<AppState>) -> Result<Vec<HeroItem>, String> {
    let lib = load_library();
    let res = cached_scan(&state);

    // 始终包含：内置 35 个 ID ∪ 拥有完整官方原皮模板的扫描 ID ∪ 用户库 ID。
    // 不能直接使用扫描 ID，否则单独的 AccountBackground 会变成“幽灵角色”。
    let mut ids: Vec<String> = builtin_hero_ids();
    for k in res.manifest.keys() {
        if k.parse::<u32>().ok().is_some_and(|id| id <= 999)
            && source_template_is_complete(&res, k, "base")
            && !ids.contains(k)
        {
            ids.push(k.clone());
        }
    }
    for k in lib.heroes.keys() {
        if k.parse::<u32>().ok().is_some_and(|id| id <= 999) && !ids.contains(k) {
            ids.push(k.clone());
        }
    }
    ids.sort_by(|a, b| a.cmp(b));

    Ok(ids
        .into_iter()
        .map(|id| {
            // 角色列表头像必须对应游戏的物品图标资源，不能用大立绘冒充。
            let avatar_canonical = format!("UT_Item_Hero_{}", id);
            let mut avatar = manifest_preview_file(&id, "item_icons", &avatar_canonical)
                .and_then(|preview| preview_path_for_file(&preview));
            if let Some(icon) = res
                .manifest
                .get(&id)
                .and_then(|m| m.get("item_icons"))
                .and_then(|icons| icons.get(&avatar_canonical))
            {
                if let Some(f) = icon.files.first() {
                    let p = Path::new(&lib.settings.asset_root).join(&icon.dir).join(f);
                    if p.exists() {
                        if avatar.is_none() {
                            avatar = Some(p.to_string_lossy().to_string());
                        }
                    }
                }
            }

            let mut active_count = 0usize;
            let mut mod_count = 0usize;
            if let Some(h) = lib.heroes.get(&id) {
                for s in h.sections.values() {
                    for r in s.resources.values() {
                        mod_count += r.files.len();
                        if r.active_hash.is_some() {
                            active_count += 1;
                        }
                    }
                }
            }
            HeroItem {
                display_name: display_name_of(&lib, &id),
                title: hero_title_of(&id),
                id,
                avatar,
                active_count,
                mod_count,
                has_mod: mod_count > 0,
            }
        })
        .collect())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceOut {
    /// 该资源所属的主体 ID。heroes 下恒为角色 ID；cards 下每行是不同卡号，
    /// 前端必须用这个字段而不是父级 heroId，否则导入/启用会写错主体。
    pub(crate) id: String,
    pub(crate) canonical: String,
    pub(crate) variant: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) display_name: Option<String>,
    pub(crate) original_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) outimg_path: Option<String>,
    pub(crate) outimg_available: bool,
    pub(crate) active_hash: Option<String>,
    pub(crate) disabled: bool,
    pub(crate) files: Vec<FileEntryOut>,
}

pub(crate) fn effective_outimg_dir(settings: &Settings) -> Option<PathBuf> {
    if !settings.outimg_path.trim().is_empty() {
        return Some(PathBuf::from(settings.outimg_path.trim()));
    }
    let modimg = PathBuf::from(settings.modimg_path.trim());
    modimg
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.join("outimg"))
}

/// 定位入口同时支持旧版导出的目录和新版插件的提取目录。
pub(crate) fn outimg_dirs(settings: &Settings) -> impl Iterator<Item = PathBuf> {
    effective_outimg_dir(settings)
        .into_iter()
        .chain(std::iter::once(data_dir().join("assets/outimg")))
}

pub(crate) fn outimg_roots(settings: &Settings) -> Vec<PathBuf> {
    outimg_dirs(settings)
        .filter_map(|dir| dir.canonicalize().ok())
        .filter(|dir| dir.is_dir())
        .collect()
}

pub(crate) fn outimg_file_for(
    settings: &Settings,
    canonical: &str,
    original_url: Option<&String>,
) -> Option<PathBuf> {
    locate_outimg(
        &outimg_roots(settings),
        canonical,
        original_url.map(String::as_str),
    )
}

pub(crate) fn locate_outimg(
    roots: &[PathBuf],
    canonical: &str,
    original_url: Option<&str>,
) -> Option<PathBuf> {
    let canonical_png = safe_relative_path(&format!("{canonical}.png"))?;
    let fallback = original_url
        .and_then(|url| url.rsplit(['/', '\\']).next())
        .and_then(safe_relative_path)
        .filter(|name| name != &canonical_png);
    roots.iter().find_map(|root| {
        std::iter::once(&canonical_png)
            .chain(fallback.as_ref())
            .filter_map(|name| root.join(name).canonicalize().ok())
            .find(|file| file.starts_with(root) && file.is_file())
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileEntryOut {
    pub(crate) hash: String,
    pub(crate) source_name: String,
    pub(crate) url: String,
    pub(crate) rel_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SectionOut {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) group: String,
    pub(crate) resources: Vec<ResourceOut>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HeroDetail {
    pub(crate) hero_id: String,
    pub(crate) display_name: String,
    pub(crate) title: Option<String>,
    pub(crate) avatar: Option<String>,
    pub(crate) sections: Vec<SectionOut>,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn hero_detail(
    hero_id: String,
    state: tauri::State<AppState>,
) -> Result<HeroDetail, String> {
    let lib = load_library();
    let res = cached_scan(&state);
    let asset = res.manifest.get(&hero_id);
    let lib_hero = lib.heroes.get(&hero_id);
    let template_canonicals = hero_template_canonicals(&res, &lib, &hero_id, true);
    let avatar_canonical = format!("UT_Item_Hero_{}", hero_id);
    let avatar = manifest_preview_file(&hero_id, "item_icons", &avatar_canonical)
        .and_then(|preview| preview_path_for_file(&preview))
        .or_else(|| {
            res.manifest
                .get(&hero_id)
                .and_then(|m| m.get("item_icons"))
                .and_then(|icons| icons.get(&avatar_canonical))
                .and_then(|icon| icon.files.first().map(|file| (icon, file)))
                .map(|(icon, file)| {
                    Path::new(&lib.settings.asset_root)
                        .join(&icon.dir)
                        .join(file)
                        .to_string_lossy()
                        .to_string()
                })
                .filter(|path| Path::new(path).exists())
        });

    let sections = sections_for_category("heroes")
        .into_iter()
        .map(|s| {
            let from_asset = asset.and_then(|m| m.get(&s.key));
            let from_lib = lib_hero
                .and_then(|h| h.sections.get(&s.key))
                .map(|x| &x.resources);

            let mut cans: Vec<String> = template_canonicals
                .get(&s.key)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect();
            // 用户库是显式意图来源，即使该资源来自旧版本库、当前解包目录已不存在，
            // 也必须继续显示它。
            if let Some(fl) = from_lib {
                for k in fl.keys() {
                    if canonicalize_for_section(k, &s.key, false).as_deref() == Some(k.as_str())
                        && !cans.contains(k)
                    {
                        cans.push(k.clone());
                    }
                }
            }
            cans.sort_by(|a, b| variant_order(a).cmp(&variant_order(b)).then(a.cmp(b)));

            let resources = cans
                .into_iter()
                .map(|cn| {
                    let orig = manifest_preview_file(&hero_id, &s.key, &cn)
                        .and_then(|preview| preview_path_for_file(&preview))
                        .or_else(|| {
                            from_asset
                                .and_then(|fa| fa.get(&cn))
                                .and_then(|e| {
                                    e.files.first().map(|f| {
                                        Path::new(&lib.settings.asset_root)
                                            .join(&e.dir)
                                            .join(f)
                                            .to_string_lossy()
                                            .to_string()
                                    })
                                })
                                .filter(|path| Path::new(path).exists())
                        });
                    let entry = from_lib.and_then(|fl| fl.get(&cn));
                    let vd = vault_dir();
                    let files = entry
                        .map(|e| {
                            e.files
                                .iter()
                                .map(|f| {
                                    let abs = if !f.rel_path.is_empty() {
                                        vd.join(&f.rel_path)
                                    } else {
                                        vd.join(format!("{}{}", f.hash, f.ext))
                                    };
                                    FileEntryOut {
                                        hash: f.hash.clone(),
                                        source_name: f.source_name.clone(),
                                        url: abs.to_string_lossy().to_string(),
                                        rel_path: f.rel_path.clone(),
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    ResourceOut {
                        id: hero_id.clone(),
                        variant: if s.key == "account_background" {
                            "原皮".to_string()
                        } else {
                            variant_label(&cn)
                        },
                        canonical: cn.clone(),
                        display_name: None,
                        original_url: orig.clone(),
                        outimg_path: outimg_file_for(&lib.settings, &cn, orig.as_ref())
                            .map(|p| p.to_string_lossy().to_string()),
                        outimg_available: outimg_file_for(&lib.settings, &cn, orig.as_ref())
                            .is_some(),
                        active_hash: entry.and_then(|e| e.active_hash.clone()),
                        disabled: entry.map(|e| e.disabled).unwrap_or(false),
                        files,
                    }
                })
                .collect();

            SectionOut {
                key: s.key,
                label: s.label,
                group: s.group,
                resources,
            }
        })
        .collect();

    Ok(HeroDetail {
        display_name: display_name_of(&lib, &hero_id),
        title: hero_title_of(&hero_id),
        hero_id,
        avatar,
        sections,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CardDetail {
    pub(crate) sections: Vec<SectionOut>,
}

/// 卡牌大类（cards）：扁平展示，不按角色分组。
/// 每一行是不同卡号的独立资源，canonical 来源 = 磁盘扫描 ∪ 已入库 ∪ 内嵌 manifest 兜底。
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn cards_detail(state: tauri::State<AppState>) -> Result<CardDetail, String> {
    let lib = load_library();
    let res = cached_scan(&state);
    let vd = vault_dir();

    let sections = sections_for_category("cards")
        .into_iter()
        .map(|s| {
            let mut pairs: Vec<(String, String)> = Vec::new();
            fn push_unique(pairs: &mut Vec<(String, String)>, id: String, cn: String) {
                if !pairs.iter().any(|(_, c)| c == &cn) {
                    pairs.push((id, cn));
                }
            }
            macro_rules! add {
                ($id:expr, $cn:expr) => {
                    push_unique(&mut pairs, $id, $cn)
                };
            }

            // 1) 磁盘扫描出来的（assetRoot 已设置时）
            for (id, secs) in res.manifest.iter() {
                if let Some(cans) = secs.get(&s.key) {
                    for k in cans.keys() {
                        add!(id.clone(), k.clone());
                    }
                }
            }
            // 2) 已经导入到素材库的（用户导过但源文件没了也要显示）
            for (id, h) in lib.heroes.iter() {
                if let Some(sec) = h.sections.get(&s.key) {
                    for k in sec.resources.keys() {
                        add!(id.clone(), k.clone());
                    }
                }
            }
            // 3) 内嵌 manifest 兜底（没设 assetRoot 时全靠这个）
            for (id, cn) in manifest_section_entries(&s.key) {
                add!(id, cn);
            }
            pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

            let resources = pairs
                .into_iter()
                .map(|(card_id, cn)| {
                    let entry = lib
                        .heroes
                        .get(&card_id)
                        .and_then(|h| h.sections.get(&s.key))
                        .and_then(|sec| sec.resources.get(&cn));

                    let orig = manifest_preview_file(&card_id, &s.key, &cn)
                        .and_then(|preview| preview_path_for_file(&preview))
                        .or_else(|| {
                            res.manifest
                                .get(&card_id)
                                .and_then(|m| m.get(&s.key))
                                .and_then(|cans| cans.get(&cn))
                                .and_then(|e| {
                                    e.files.first().map(|f| {
                                        Path::new(&lib.settings.asset_root)
                                            .join(&e.dir)
                                            .join(f)
                                            .to_string_lossy()
                                            .to_string()
                                    })
                                })
                                .filter(|p| Path::new(p).exists())
                        });

                    let files = entry
                        .map(|e| {
                            e.files
                                .iter()
                                .map(|f| {
                                    let abs = if !f.rel_path.is_empty() {
                                        vd.join(&f.rel_path)
                                    } else {
                                        vd.join(format!("{}{}", f.hash, f.ext))
                                    };
                                    FileEntryOut {
                                        hash: f.hash.clone(),
                                        source_name: f.source_name.clone(),
                                        url: abs.to_string_lossy().to_string(),
                                        rel_path: f.rel_path.clone(),
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    ResourceOut {
                        id: card_id,
                        display_name: card_display_name(&s.key, &cn),
                        canonical: cn.clone(),
                        variant: "原皮".to_string(),
                        original_url: orig.clone(),
                        outimg_path: outimg_file_for(&lib.settings, &cn, orig.as_ref())
                            .map(|p| p.to_string_lossy().to_string()),
                        outimg_available: outimg_file_for(&lib.settings, &cn, orig.as_ref())
                            .is_some(),
                        active_hash: entry.and_then(|e| e.active_hash.clone()),
                        disabled: entry.map(|e| e.disabled).unwrap_or(false),
                        files,
                    }
                })
                .collect();

            SectionOut {
                key: s.key,
                label: s.label,
                group: s.group,
                resources,
            }
        })
        .collect();

    Ok(CardDetail { sections })
}

/* ============ 怪物（monsters）板块：扁平展示，对齐 cards ============ */

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MonstersDetail {
    pub(crate) sections: Vec<SectionOut>,
}

/// 怪物大类（monsters）：与 cards 一样扁平平摊，不划分二级板块。
/// 每一行是一个怪物在某部位的独立资源；行来源 = 磁盘扫描 ∪ 已入库 ∪ 内置 ID 兜底合成。
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn monsters_detail(state: tauri::State<AppState>) -> Result<MonstersDetail, String> {
    let lib = load_library();
    let res = cached_scan(&state);
    let vd = vault_dir();

    // 主体 id 集：内置 64 个 ∪ 扫描中出现的怪物 ID ∪ 用户库 ID（怪物统一以 id 标识）。
    let mut ids: Vec<String> = builtin_monster_ids();
    for (id, secs) in res.manifest.iter() {
        if secs.keys().any(|k| category_of(k) == "monsters") && !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    for k in lib.monsters.keys() {
        if !ids.contains(k) {
            ids.push(k.clone());
        }
    }
    ids.sort_by(|a, b| a.cmp(b));

    let sections = sections_for_category("monsters")
        .into_iter()
        .map(|s| {
            let mut pairs: Vec<(String, String)> = Vec::new();
            fn push_unique(pairs: &mut Vec<(String, String)>, id: &str, cn: String) {
                if !pairs.iter().any(|(i, c)| i == id && c == &cn) {
                    pairs.push((id.to_string(), cn));
                }
            }

            // 1) 磁盘扫描出来的（assetRoot 已设置时）
            for id in ids.iter() {
                if let Some(cans) = res.manifest.get(id).and_then(|m| m.get(&s.key)) {
                    for k in cans.keys() {
                        push_unique(&mut pairs, id, k.clone());
                    }
                }
            }
            // 2) 已经导入到素材库的（用户导过但源文件没了也要显示）
            for (id, h) in lib.monsters.iter() {
                if let Some(sec) = h.sections.get(&s.key) {
                    for k in sec.resources.keys() {
                        push_unique(&mut pairs, id, k.clone());
                    }
                }
            }
            // 3) 内置兜底合成（assetRoot 未设置时也能平摊全部怪物）：
            //    card → UT_Monster_Card_{id}；bust → UT_Monster_Bust_{id}_0；profile → UT_Hero_ProfilePhoto_{id}
            for id in ids.iter() {
                let cn = match s.key.as_str() {
                    "monster_card" => format!("UT_Monster_Card_{}", id),
                    "monster_bust" => format!("UT_Monster_Bust_{}_0", id),
                    "monster_profile" => format!("UT_Hero_ProfilePhoto_{}", id),
                    _ => continue,
                };
                push_unique(&mut pairs, id, cn);
            }
            pairs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

            let resources = pairs
                .into_iter()
                .map(|(monster_id, cn)| {
                    let entry = lib
                        .monsters
                        .get(&monster_id)
                        .and_then(|h| h.sections.get(&s.key))
                        .and_then(|sec| sec.resources.get(&cn));

                    let orig = manifest_preview_file(&monster_id, &s.key, &cn)
                        .and_then(|preview| preview_path_for_file(&preview))
                        .or_else(|| {
                            res.manifest
                                .get(&monster_id)
                                .and_then(|m| m.get(&s.key))
                                .and_then(|cans| cans.get(&cn))
                                .and_then(|e| {
                                    e.files.first().map(|f| {
                                        Path::new(&lib.settings.asset_root)
                                            .join(&e.dir)
                                            .join(f)
                                            .to_string_lossy()
                                            .to_string()
                                    })
                                })
                                .filter(|p| Path::new(p).exists())
                        });

                    let files = entry
                        .map(|e| {
                            e.files
                                .iter()
                                .map(|f| {
                                    let abs = if !f.rel_path.is_empty() {
                                        vd.join(&f.rel_path)
                                    } else {
                                        vd.join(format!("{}{}", f.hash, f.ext))
                                    };
                                    FileEntryOut {
                                        hash: f.hash.clone(),
                                        source_name: f.source_name.clone(),
                                        url: abs.to_string_lossy().to_string(),
                                        rel_path: f.rel_path.clone(),
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    ResourceOut {
                        id: monster_id.clone(),
                        display_name: Some(monster_display_name(&lib, &monster_id)),
                        canonical: cn.clone(),
                        variant: "原皮".to_string(),
                        original_url: orig.clone(),
                        outimg_path: outimg_file_for(&lib.settings, &cn, orig.as_ref())
                            .map(|p| p.to_string_lossy().to_string()),
                        outimg_available: outimg_file_for(&lib.settings, &cn, orig.as_ref())
                            .is_some(),
                        active_hash: entry.and_then(|e| e.active_hash.clone()),
                        disabled: entry.map(|e| e.disabled).unwrap_or(false),
                        files,
                    }
                })
                .collect();

            SectionOut {
                key: s.key,
                label: s.label,
                group: s.group,
                resources,
            }
        })
        .collect();

    Ok(MonstersDetail { sections })
}

pub(crate) fn cached_scan(state: &tauri::State<AppState>) -> ScanResult {
    let mut guard = state.scan.lock().unwrap();
    if guard.is_none() {
        let lib = load_library();
        *guard = Some(scan_assets(&lib.settings.asset_root));
    }
    guard.clone().unwrap()
}

pub(crate) fn sha256_file(p: &Path) -> Result<String, String> {
    let mut file = fs::File::open(p).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buffer[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

pub(crate) fn ext_of(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| format!(".{}", e.to_ascii_lowercase()))
        .unwrap_or_else(|| ".png".to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportResult {
    pub(crate) file: String,
    pub(crate) ok: bool,
    pub(crate) reason: Option<String>,
    pub(crate) duplicate: bool,
    pub(crate) applied: bool,
    pub(crate) apply_error: Option<String>,
    pub(crate) hero_id: Option<String>,
    pub(crate) section_key: Option<String>,
    pub(crate) canonical: Option<String>,
    pub(crate) hash: Option<String>,
    pub(crate) rel_path: Option<String>,
}

pub(crate) fn import_error(file: String, reason: String) -> ImportResult {
    ImportResult {
        file,
        ok: false,
        reason: Some(reason),
        duplicate: false,
        applied: false,
        apply_error: None,
        hero_id: None,
        section_key: None,
        canonical: None,
        hash: None,
        rel_path: None,
    }
}

pub(crate) fn import_single(
    lib: &mut Library,
    vd: &Path,
    path: &str,
    hero_id: &str,
    section_key: &str,
    canonical: &str,
    source_stem: Option<&str>,
) -> Result<(ImportResult, bool), ImportResult> {
    let pb = PathBuf::from(path);
    let original_name = pb
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = ext_of(&original_name);
    if !ext.eq_ignore_ascii_case(".png") {
        return Err(import_error(original_name, "only_png".into()));
    }
    let name = source_stem
        .map(|s| format!("{}{}", s, ext))
        .unwrap_or(original_name);
    let hash = match sha256_file(&pb) {
        Ok(h) => h,
        Err(e) => {
            return Err(import_error(name, e));
        }
    };
    let cat = category_of(section_key);
    let (dest, rel) = match place_in_vault(vd, &cat, hero_id, section_key, &name, &hash, &ext) {
        Ok(v) => v,
        Err(e) => {
            return Err(import_error(name, e));
        }
    };
    if !dest.exists() {
        if let Err(e) = fs::copy(&pb, &dest) {
            return Err(import_error(name, e.to_string()));
        }
    }
    let res = get_resource(lib, &cat, hero_id, section_key, canonical);
    let dup = res.files.iter().any(|f| f.hash == hash);
    let first_mod = !dup && res.files.is_empty();
    if !dup {
        res.files.push(FileEntry {
            hash: hash.clone(),
            ext,
            source_name: name.clone(),
            rel_path: rel.clone(),
            added_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            info: Default::default(),
            classification: Default::default(),
            fs_identity: String::new(),
        });
    }
    Ok((
        ImportResult {
            file: name,
            ok: true,
            reason: None,
            duplicate: dup,
            applied: false,
            apply_error: None,
            hero_id: Some(hero_id.to_string()),
            section_key: Some(section_key.to_string()),
            canonical: Some(canonical.to_string()),
            hash: Some(hash),
            rel_path: Some(rel),
        },
        first_mod,
    ))
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn import_files(
    paths: Vec<String>,
    state: tauri::State<AppState>,
) -> Result<Vec<ImportResult>, String> {
    crate::image_import::import_selected(serde_json::json!({ "paths": paths }), state)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportTargetArgs {
    pub(crate) paths: Vec<String>,
    pub(crate) hero_id: String,
    pub(crate) section_key: String,
    pub(crate) canonical: String,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn import_files_to_target(
    args: ImportTargetArgs,
    state: tauri::State<AppState>,
) -> Result<Vec<ImportResult>, String> {
    crate::image_import::import_selected(
        serde_json::json!({ "paths": args.paths, "target": { "heroId": args.hero_id, "sectionKey": args.section_key, "canonical": args.canonical } }),
        state,
    )
}
