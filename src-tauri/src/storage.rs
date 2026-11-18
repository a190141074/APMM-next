//! 从旧版迁入的领域实现；保留协议与行为，在明确边界内逐步替换。
use crate::*;

/* ============ 数据目录（便携） ============ */

pub(crate) fn data_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        if let Ok(isolated) = std::env::var("ASTRAL_DEV_DATA_DIR") {
            if !isolated.trim().is_empty() {
                return PathBuf::from(isolated);
            }
        }
        crate::staging::work_root().join("dev-data")
    } else {
        let directory = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(std::env::temp_dir);
        crate::installation::data_for(&directory)
    }
}

pub(crate) fn ensure_dir(p: &Path) -> std::io::Result<()> {
    if !p.exists() {
        fs::create_dir_all(p)?;
    }
    Ok(())
}

pub(crate) fn vault_dir() -> PathBuf {
    let d = data_dir().join("vault");
    let _ = ensure_dir(&d);
    d
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct ModScanCache {
    #[serde(default)]
    pub(crate) entries: HashMap<String, ModScanCacheEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ModScanCacheEntry {
    pub(crate) size: u64,
    pub(crate) modified: u64,
    pub(crate) hash: String,
    /// A path relative to modimg, plus the final parsed stable key.  All are
    /// optional on disk so v0.2.8 cache files migrate without a full rehash.
    #[serde(default)]
    pub(crate) relative_path: String,
    #[serde(default)]
    pub(crate) category: String,
    #[serde(default)]
    pub(crate) entity_id: String,
    #[serde(default)]
    pub(crate) section_key: String,
    #[serde(default)]
    pub(crate) canonical: String,
}
pub(crate) fn mod_scan_cache_path() -> PathBuf {
    data_dir().join("modimg-scan-cache.json")
}
pub(crate) fn load_mod_scan_cache() -> ModScanCache {
    fs::read_to_string(mod_scan_cache_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
pub(crate) fn save_mod_scan_cache(cache: &ModScanCache) {
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = fs::write(mod_scan_cache_path(), json);
    }
}
pub(crate) fn cached_file_hash_for_key(
    path: &Path,
    cache: &mut ModScanCache,
    key: &str,
) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Some(entry) = cache.entries.get(key) {
        if entry.size == meta.len() && entry.modified == modified {
            return Ok(entry.hash.clone());
        }
    }
    let hash = sha256_file(path)?;
    cache.entries.insert(
        key.to_string(),
        ModScanCacheEntry {
            size: meta.len(),
            modified,
            hash: hash.clone(),
            relative_path: String::new(),
            category: String::new(),
            entity_id: String::new(),
            section_key: String::new(),
            canonical: String::new(),
        },
    );
    Ok(hash)
}
pub(crate) fn cached_file_hash(path: &Path, cache: &mut ModScanCache) -> Result<String, String> {
    let key = path.to_string_lossy().to_string();
    cached_file_hash_for_key(path, cache, &key)
}

pub(crate) fn scan_relative_key(modimg: &Path, path: &Path) -> String {
    path.strip_prefix(modimg)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn migrate_scan_cache_keys(cache: &mut ModScanCache, modimg: &Path) {
    let mut migrated = HashMap::new();
    for (key, mut entry) in std::mem::take(&mut cache.entries) {
        let relative = Path::new(&key)
            .strip_prefix(modimg)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| key.replace('\\', "/"));
        if entry.relative_path.is_empty() {
            entry.relative_path = relative.clone();
        }
        migrated.insert(relative, entry);
    }
    cache.entries = migrated;
}

pub(crate) fn logs_dir() -> PathBuf {
    let d = data_dir().join("logs");
    let _ = ensure_dir(&d);
    d
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// JSONL keeps diagnostics machine-readable while remaining easy to attach to
/// a bug report.  Rotation prevents a runaway error loop filling the disk.

/* package implementation moved to packages.rs */
/* ============ 树状存储：{category}/{id}/{part}/{file} ============ */

pub(crate) const INVALID_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

pub(crate) fn sanitize_name(name: &str) -> String {
    let mut s: String = name
        .chars()
        .filter(|c| !INVALID_CHARS.contains(c) && !c.is_control())
        .collect();
    s = s.trim().trim_matches('.').to_string();
    if s.is_empty() {
        s = "mod".to_string();
    }
    s
}

/// vault 顶层目录名与数据类型对应：临时资源（section_key/category == "temporary"）
/// 独立存放在与 heroes 平级的 `temporary/`，其余按大类目录。
pub(crate) fn vault_top_dir(category: &str, part: &str) -> String {
    if part == "temporary" || category == "temporary" {
        "temporary".to_string()
    } else {
        sanitize_name(category)
    }
}

/// 在 vault 中为贴图分配树状路径；返回 (绝对路径, 相对路径)
/// 目标：`{vault}/{category}/{id}/{part}/{source_name}`
pub(crate) fn place_in_vault(
    vd: &Path,
    category: &str,
    id: &str,
    part: &str,
    source_name: &str,
    hash: &str,
    ext: &str,
) -> Result<(PathBuf, String), String> {
    // `temporary:<sha256>` is a stable library identity but `:` is forbidden
    // in Windows directory names.  Every tree component must therefore be
    // converted to a filesystem-safe representation; the original stable key
    // remains exclusively in library.json, never inferred from this path.
    let category_dir = vault_top_dir(category, part);
    let id_dir = sanitize_name(id);
    let part_dir = sanitize_name(part);
    let dir = vd.join(&category_dir).join(&id_dir).join(&part_dir);
    ensure_dir(&dir).map_err(|e| e.to_string())?;

    let base = sanitize_name(source_name);
    let mut final_name = if base
        .to_ascii_lowercase()
        .ends_with(&ext.to_ascii_lowercase())
    {
        base.clone()
    } else {
        format!("{}{}", base, ext)
    };
    let mut p = dir.join(&final_name);
    if p.exists() && sha256_file(&p).unwrap_or_default() != hash {
        let stem = final_name
            .rsplit_once('.')
            .map(|(a, _)| a.to_string())
            .unwrap_or_else(|| final_name.clone());
        let short: String = hash.chars().take(8).collect();
        final_name = format!("{}_{}{}", stem, short, ext);
        p = dir.join(&final_name);
    }
    Ok((
        p,
        format!("{}/{}/{}/{}", category_dir, id_dir, part_dir, final_name),
    ))
}

/* ============ library.json ============ */

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileEntry {
    pub(crate) hash: String,
    #[serde(default = "default_ext")]
    pub(crate) ext: String,
    #[serde(default)]
    pub(crate) source_name: String,
    /// 相对 vault 的树状路径，如 heroes/101/busts/xxx.png
    #[serde(default)]
    pub(crate) rel_path: String,
    #[serde(default)]
    pub(crate) added_at: u64,
    #[serde(default)]
    pub(crate) info: crate::management_types::CandidateInfo,
    #[serde(default)]
    pub(crate) classification: crate::management_types::ClassificationRecord,
    #[serde(default)]
    pub(crate) fs_identity: String,
}
pub(crate) fn default_ext() -> String {
    ".png".into()
}

pub(crate) fn is_png_mod_ext(ext: &str) -> bool {
    ext.eq_ignore_ascii_case(".png")
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceEntry {
    pub(crate) active_hash: Option<String>,
    /// The game file has been atomically renamed to `.disabled`.  Keep the
    /// active hash so enabling can restore exactly the selected Mod.
    #[serde(default)]
    pub(crate) disabled: bool,
    /// Temporary resources may be present in modimg under their original
    /// filename, but they do not occupy a formal canonical resource slot.
    #[serde(default)]
    pub(crate) external_present: bool,
    #[serde(default)]
    pub(crate) files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SectionEntry {
    #[serde(default)]
    pub(crate) resources: HashMap<String, ResourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HeroEntry {
    #[serde(default)]
    pub(crate) sections: HashMap<String, SectionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    #[serde(default)]
    pub(crate) author_mode: bool,
    #[serde(default)]
    pub(crate) appearance: Option<crate::appearance::AppearanceSettings>,
    #[serde(default)]
    pub(crate) modimg_path: String,
    #[serde(default)]
    pub(crate) asset_root: String,
    #[serde(default)]
    pub(crate) outimg_path: String,
    #[serde(default)]
    pub(crate) display_names: HashMap<String, String>,
    #[serde(default)]
    pub(crate) view_mode: Option<String>,
    #[serde(default)]
    pub(crate) thumb_size: Option<String>,
    #[serde(default)]
    pub(crate) resource_filter: Option<String>,
    #[serde(default)]
    pub(crate) search_mode: Option<String>,
    #[serde(default)]
    pub(crate) presets: Vec<GlobalPreset>,
    #[serde(default = "default_true")]
    pub(crate) monitor_directories: bool,
    #[serde(default = "default_true")]
    pub(crate) package_apply_settings: bool,
    /// 插件库使用的客户端：CN_STEAM / CN_TAPTAP / CN_BILIBILI / CN_WEGAME
    #[serde(default)]
    pub(crate) plugin_platform: Option<String>,
    #[serde(default)]
    pub(crate) plugin_locallow_dir: Option<String>,
    #[serde(default)]
    pub(crate) plugin_data_dir: Option<String>,
    #[serde(default)]
    pub(crate) plugin_root_dir: Option<String>,
    /// APX 的目录按渠道独立保存；旧全局字段仅用于读取迁移。
    #[serde(default)]
    pub(crate) plugin_directories: HashMap<String, crate::apx_directories::PluginDirectories>,
    /// 启动时 / 定时自动检查软件更新（默认开启）。
    #[serde(default = "default_true")]
    pub(crate) update_auto_check: bool,
    /// 自动检查间隔（小时，1–720，默认 24）。
    #[serde(default = "default_update_interval_hours")]
    pub(crate) update_interval_hours: u32,
    /// 用户选择「跳过该版本」时记录的版本号。
    #[serde(default)]
    pub(crate) update_skipped_version: Option<String>,
    /// 上一次成功检查更新的时间（Unix 秒，前端用来做节流）。
    #[serde(default)]
    pub(crate) update_last_checked_at: Option<u64>,
}

pub(crate) fn default_update_interval_hours() -> u32 {
    24
}

/// 归一化更新检查间隔：非法值或越界值回落到 24 小时。
pub fn normalized_update_interval_hours(raw: u32) -> u32 {
    if raw == 0 || raw > 720 {
        24
    } else {
        raw
    }
}

impl Default for Settings {
    fn default() -> Self {
        serde_json::from_str("{}").expect("设置默认值完整")
    }
}

pub(crate) fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Library {
    #[serde(default)]
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) settings: Settings,
    #[serde(default)]
    pub(crate) heroes: HashMap<String, HeroEntry>,
    #[serde(default)]
    pub(crate) monsters: HashMap<String, HeroEntry>,
    #[serde(default)]
    pub(crate) workshop: HashMap<String, workshop::InstalledWork>,
    #[serde(default)]
    pub(crate) external_bindings: HashMap<String, crate::management_types::ExternalBinding>,
}

pub(crate) fn library_path() -> PathBuf {
    let d = data_dir();
    let _ = ensure_dir(&d);
    d.join("library.json")
}

/// 历史版本把临时资源放在 `heroes/temporary:.../temporary/`。加载库时把仍在
/// 旧位置的文件搬到与 heroes 平级的 `temporary/`，并同步 library.json 的
/// rel_path。包含已禁用的条目；搬移失败时保留原 rel_path，由 cleanup 兜底。
/// 返回成功搬移的文件数。
pub(crate) fn migrate_temporary_vault_layout(lib: &mut Library, vault: &Path) -> usize {
    if !vault.exists() {
        return 0;
    }
    let mut moved = 0usize;
    let ids: Vec<String> = lib.heroes.keys().cloned().collect();
    for id in ids {
        if !id.starts_with("temporary:") {
            continue;
        }
        // 收集 (rel_path, hash)，仅处理仍在 heroes/ 下的旧路径。
        let items: Vec<(String, String)> = lib
            .heroes
            .get(&id)
            .and_then(|h| h.sections.get("temporary"))
            .map(|s| {
                s.resources
                    .values()
                    .flat_map(|r| r.files.iter())
                    .filter(|f| f.rel_path.starts_with("heroes/"))
                    .map(|f| (f.rel_path.clone(), f.hash.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if items.is_empty() {
            continue;
        }
        let mut touched = false;
        for (old_rel, hash) in items {
            let new_rel = format!("temporary/{}", &old_rel["heroes/".len()..]);
            let from = vault.join(&old_rel);
            let to = vault.join(&new_rel);
            if from.exists() {
                let Some(parent) = to.parent() else { continue };
                if ensure_dir(parent).is_err() {
                    continue;
                }
                if to.exists() {
                    if sha256_file(&to).ok().as_deref() != Some(hash.as_str()) {
                        continue;
                    }
                    let _ = fs::remove_file(&from);
                } else if fs::rename(&from, &to).is_err() {
                    continue;
                }
            } else if !to.exists() {
                // 新旧位置都不存在：保留索引原样，交给 cleanup 清理。
                continue;
            }
            if let Some(section) = lib
                .heroes
                .get_mut(&id)
                .and_then(|h| h.sections.get_mut("temporary"))
            {
                for resource in section.resources.values_mut() {
                    for file in resource.files.iter_mut() {
                        if file.rel_path == old_rel {
                            file.rel_path = new_rel.clone();
                        }
                    }
                }
            }
            touched = true;
            moved += 1;
        }
        if touched {
            // 尽力清理旧空目录（非空时 remove_dir 自然失败，无副作用）。
            let id_dir = sanitize_name(&id);
            let _ = fs::remove_dir(vault.join("heroes").join(&id_dir).join("temporary"));
            let _ = fs::remove_dir(vault.join("heroes").join(&id_dir));
        }
    }
    moved
}

pub(crate) fn load_library() -> Library {
    let p = library_path();
    let mut lib = fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str::<Library>(&s).ok())
        .or_else(|| {
            // Never silently discard a corrupt library. Keep the original for
            // diagnosis and recover the last atomic backup when available.
            let backup = p.with_extension("json.bak");
            let corrupt = p.with_extension(format!("json.corrupt-{}", unix_now()));
            if p.exists() && fs::rename(&p, &corrupt).is_err() {
                write_log("error", "library_load", "库文件损坏且无法保留副本", None);
            }
            fs::read_to_string(backup)
                .ok()
                .and_then(|s| serde_json::from_str::<Library>(&s).ok())
        })
        .unwrap_or_default();
    crate::apx_directories::migrate_plugin_directories(&mut lib.settings);
    if migrate_temporary_vault_layout(&mut lib, &vault_dir()) > 0 {
        let _ = save_library(&lib);
    }
    lib
}

pub(crate) fn save_library(lib: &Library) -> Result<(), String> {
    let s = serde_json::to_string_pretty(lib).map_err(|e| e.to_string())?;
    let target = library_path();
    let temp = target.with_extension("json.part");
    let backup = target.with_extension("json.bak");
    {
        let mut file = fs::File::create(&temp).map_err(|e| format!("写入库暂存文件失败：{}", e))?;
        file.write_all(s.as_bytes())
            .map_err(|e| format!("写入库暂存文件失败：{}", e))?;
        file.sync_all()
            .map_err(|e| format!("同步库暂存文件失败：{}", e))?;
    }
    let had_target = target.exists();
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    if had_target {
        if let Err(error) = fs::rename(&target, &backup) {
            let _ = fs::remove_file(&temp);
            return Err(format!("准备替换库索引失败：{}", error));
        }
    }
    if let Err(error) = fs::rename(&temp, &target) {
        if had_target {
            let _ = fs::rename(&backup, &target);
        }
        let _ = fs::remove_file(&temp);
        return Err(format!("提交库索引失败：{}", error));
    }
    if had_target {
        let _ = fs::remove_file(&backup);
    }
    Ok(())
}

/// 按大类返回素材库中的主体 map：monsters 单独存放，其余（heroes/cards…）统一在 heroes。
pub(crate) fn lib_map_mut<'a>(
    lib: &'a mut Library,
    category: &str,
) -> &'a mut HashMap<String, HeroEntry> {
    if category == "monsters" {
        &mut lib.monsters
    } else {
        &mut lib.heroes
    }
}

pub(crate) fn lib_map<'a>(lib: &'a Library, category: &str) -> &'a HashMap<String, HeroEntry> {
    if category == "monsters" {
        &lib.monsters
    } else {
        &lib.heroes
    }
}

pub(crate) fn get_resource<'a>(
    lib: &'a mut Library,
    category: &str,
    id: &str,
    section: &str,
    canonical: &str,
) -> &'a mut ResourceEntry {
    lib_map_mut(lib, category)
        .entry(id.to_string())
        .or_default()
        .sections
        .entry(section.to_string())
        .or_default()
        .resources
        .entry(canonical.to_string())
        .or_default()
}

/* ============ 旧扁平 vault -> 树状迁移 ============ */

pub(crate) fn migrate_map(map: &mut HashMap<String, HeroEntry>, vd: &Path, changed: &mut usize) {
    let ids: Vec<String> = map.keys().cloned().collect();
    for hid in ids {
        let Some(hero) = map.get_mut(&hid) else {
            continue;
        };
        let sec_keys: Vec<String> = hero.sections.keys().cloned().collect();
        for skey in sec_keys {
            let Some(sec) = hero.sections.get_mut(&skey) else {
                continue;
            };
            let cat = category_of(&skey);
            let cans: Vec<String> = sec.resources.keys().cloned().collect();
            for cn in cans {
                let Some(res) = sec.resources.get_mut(&cn) else {
                    continue;
                };
                for f in res.files.iter_mut() {
                    // 已有路径即使暂时缺失也交给目录同步关联，不能抢先抹掉外部重命名线索。
                    if !f.rel_path.is_empty() {
                        continue;
                    }
                    let old = vd.join(format!("{}{}", f.hash, f.ext));
                    let (dest, rel) = match place_in_vault(
                        &vd,
                        &cat,
                        &hid,
                        &skey,
                        &f.source_name,
                        &f.hash,
                        &f.ext,
                    ) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    if old.exists() {
                        if dest.exists() && dest != old {
                            let _ = fs::remove_file(&old);
                        } else {
                            let _ = fs::rename(&old, &dest);
                        }
                    }
                    f.rel_path = rel;
                    *changed += 1;
                }
            }
        }
    }
}

pub(crate) fn migrate_vault() -> usize {
    let mut lib = load_library();
    let vd = vault_dir();
    let mut changed = 0usize;

    migrate_map(&mut lib.heroes, &vd, &mut changed);
    migrate_map(&mut lib.monsters, &vd, &mut changed);

    // 清理根目录遗留的扁平文件（{hash}.{ext}）
    if let Ok(rd) = fs::read_dir(&vd) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let is_flat = name
                .rsplit_once('.')
                .map(|(s, ext)| {
                    s.len() == 64
                        && s.chars().all(|c| c.is_ascii_hexdigit())
                        && matches!(
                            ext.to_ascii_lowercase().as_str(),
                            "png" | "jpg" | "jpeg" | "webp"
                        )
                })
                .unwrap_or(false);
            if is_flat {
                let _ = fs::remove_file(&p);
            }
        }
    }

    if changed > 0 {
        let _ = save_library(&lib);
    }
    changed
}
