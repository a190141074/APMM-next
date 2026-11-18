//! Embedded catalog and manifest lookups.
use crate::{canonicalize_for_section, Library};
use serde::Deserialize;
use std::collections::HashMap;
mod sections {
    pub const JSON: &str = include_str!("../../config/sections.json");
}
mod hero_names {
    pub const JSON: &str = include_str!("../../config/heroes.json");
}
mod monster_names {
    pub const JSON: &str = include_str!("../../config/monsters.json");
}
mod card_names {
    pub const JSON: &str = include_str!("../../config/card_names.json");
}

pub(crate) fn card_display_name(section: &str, canonical: &str) -> Option<String> {
    let id = match section {
        "hand_card" => canonical.strip_prefix("UT_HandCard_"),
        "event_card" => canonical.strip_prefix("UT_Event_"),
        "map_event" => canonical.strip_prefix("UT_MapEvent_"),
        _ => None,
    }?;
    static CACHE: std::sync::OnceLock<HashMap<String, HashMap<String, String>>> =
        std::sync::OnceLock::new();
    let map = CACHE.get_or_init(|| {
        serde_json::from_str::<HashMap<String, HashMap<String, String>>>(card_names::JSON)
            .unwrap_or_default()
    });
    map.get(section)
        .and_then(|names| {
            names
                .get(id)
                .or_else(|| names.get(id.split('_').next().unwrap_or(id)))
        })
        .cloned()
}

/* ============ 角色名对照表（ID -> 中/英名 + 称号） ============ */

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct HeroMeta {
    #[serde(default)]
    pub(crate) cn: String,
    #[serde(default)]
    pub(crate) en: String,
    #[serde(default, rename = "titleCn")]
    pub(crate) title_cn: String,
}

fn hero_meta(id: &str) -> Option<HeroMeta> {
    static CACHE: std::sync::OnceLock<HashMap<String, HeroMeta>> = std::sync::OnceLock::new();
    let map = CACHE.get_or_init(|| {
        serde_json::from_str::<HashMap<String, HeroMeta>>(hero_names::JSON).unwrap_or_default()
    });
    map.get(id).cloned()
}

/// 显示名优先级：用户在库中自定义 > 内置对照表中文名 > 角色 ID
pub(crate) fn display_name_of(lib: &Library, id: &str) -> String {
    if let Some(v) = lib.settings.display_names.get(id) {
        if !v.trim().is_empty() {
            return v.clone();
        }
    }
    hero_meta(id)
        .filter(|m| !m.cn.is_empty())
        .map(|m| m.cn)
        .unwrap_or_else(|| id.to_string())
}

pub(crate) fn hero_title_of(id: &str) -> Option<String> {
    hero_meta(id)
        .filter(|m| !m.title_cn.is_empty())
        .map(|m| m.title_cn)
}

/* ============ 怪物名对照表（ID -> 中/英名 + 称号，名称暂缺，留待补充） ============ */

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct MonsterMeta {
    #[serde(default)]
    pub(crate) cn: String,
    #[serde(default)]
    pub(crate) en: String,
    #[serde(default, rename = "titleCn")]
    pub(crate) title_cn: String,
}

fn monster_meta(id: &str) -> Option<MonsterMeta> {
    static CACHE: std::sync::OnceLock<HashMap<String, MonsterMeta>> = std::sync::OnceLock::new();
    let map = CACHE.get_or_init(|| {
        serde_json::from_str::<HashMap<String, MonsterMeta>>(monster_names::JSON)
            .unwrap_or_default()
    });
    map.get(id).cloned()
}

/// 怪物显示名优先级：用户在库中自定义 > 内置对照表中文名 > 怪物 ID
pub(crate) fn monster_display_name(lib: &Library, id: &str) -> String {
    if let Some(v) = lib.settings.display_names.get(id) {
        if !v.trim().is_empty() {
            return v.clone();
        }
    }
    monster_meta(id)
        .filter(|m| !m.cn.is_empty())
        .map(|m| m.cn)
        .unwrap_or_else(|| id.to_string())
}

/// 怪物称号（名称/称号对照暂缺，待 monsters.json 补充后启用）
#[allow(dead_code)]
fn monster_title_of(id: &str) -> Option<String> {
    monster_meta(id)
        .filter(|m| !m.title_cn.is_empty())
        .map(|m| m.title_cn)
}

/// 内置怪物 ID：来自 config/monsters.json（全部 64 个，名称延后补充）
pub(crate) fn builtin_monster_ids() -> Vec<String> {
    static CACHE: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let map: HashMap<String, MonsterMeta> =
                serde_json::from_str(monster_names::JSON).unwrap_or_default();
            let mut v: Vec<String> = map.keys().cloned().collect();
            v.sort();
            v
        })
        .clone()
}

/* ============ section 定义 ============ */

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SectionDef {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) group: String,
    #[serde(default = "default_category")]
    pub(crate) category: String,
    #[serde(rename = "assetDir")]
    pub(crate) asset_dir: String,
    pub(crate) regex: String,
    #[serde(default, rename = "stripIndex")]
    pub(crate) strip_index: bool,
    #[serde(default, rename = "stripSkin")]
    pub(crate) strip_skin: bool,
}
fn default_category() -> String {
    "heroes".into()
}

pub(crate) fn section_defs() -> Vec<SectionDef> {
    serde_json::from_str(sections::JSON).unwrap_or_default()
}

/// 全局默认角色 ID：101-129 + 301-306（共 35 名）
pub(crate) fn builtin_hero_ids() -> Vec<String> {
    let mut v: Vec<String> = (101..=129).map(|i| i.to_string()).collect();
    v.extend((301..=306).map(|i| i.to_string()));
    v
}

pub(crate) fn category_of(section_key: &str) -> String {
    section_defs()
        .into_iter()
        .find(|s| s.key == section_key)
        .map(|s| s.category)
        .unwrap_or_else(default_category)
}

/// 取某个顶层大类（heroes / cards …）下的全部 section
pub(crate) fn sections_for_category(cat: &str) -> Vec<SectionDef> {
    section_defs()
        .into_iter()
        .filter(|s| s.category == cat)
        .collect()
}

/// 从内嵌 manifest 里取某个 section 的全部 (heroId, canonical)。
/// 没设 assetRoot 时磁盘扫不到任何资源，前端全靠这份清单兜底。
pub(crate) fn manifest_section_entries(section_key: &str) -> Vec<(String, String)> {
    #[derive(Deserialize)]
    struct Entry {
        #[serde(rename = "heroId")]
        hero_id: String,
        section: String,
        canonical: String,
    }
    let entries: Vec<Entry> = serde_json::from_str(HERO_MANIFEST).unwrap_or_default();
    entries
        .into_iter()
        .filter(|e| e.section == section_key)
        .map(|e| (e.hero_id, e.canonical))
        .collect()
}

/* ============ 嵌入资源清单（previews manifest） ============ */

pub(crate) const HERO_MANIFEST: &str = include_str!("../../config/manifest.json");

pub(crate) fn manifest_canonicals(hero_id: &str, section_key: &str) -> Vec<String> {
    #[derive(Deserialize)]
    struct Entry {
        #[serde(rename = "heroId")]
        hero_id: String,
        section: String,
        canonical: String,
        #[serde(default)]
        preview: Option<String>,
    }
    let entries: Vec<Entry> = serde_json::from_str(HERO_MANIFEST).unwrap_or_default();
    entries
        .into_iter()
        // `preview` 是旧版“拿别的皮肤预览图凑数”的回退标记，不能作为真实资源。
        .filter(|e| {
            e.hero_id == hero_id
                && e.section == section_key
                && e.preview.is_none()
                && canonicalize_for_section(&e.canonical, section_key, false).as_deref()
                    == Some(e.canonical.as_str())
        })
        .map(|e| e.canonical)
        .collect()
}

pub(crate) fn manifest_preview_file(
    hero_id: &str,
    section_key: &str,
    canonical: &str,
) -> Option<String> {
    #[derive(Deserialize)]
    struct Entry {
        #[serde(rename = "heroId")]
        hero_id: String,
        section: String,
        canonical: String,
        #[serde(default)]
        preview: Option<String>,
        #[serde(default, rename = "previewFile")]
        preview_file: Option<String>,
        #[serde(default, rename = "previewExt")]
        preview_ext: Option<String>,
    }
    static INDEX: std::sync::OnceLock<HashMap<(String, String, String), String>> =
        std::sync::OnceLock::new();
    let index = INDEX.get_or_init(|| {
        serde_json::from_str::<Vec<Entry>>(HERO_MANIFEST)
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| entry.preview.is_none())
            .map(|entry| {
                let file = entry.preview_file.unwrap_or_else(|| {
                    format!(
                        "{}.{}",
                        entry.canonical,
                        entry.preview_ext.unwrap_or_else(|| "webp".into())
                    )
                });
                ((entry.hero_id, entry.section, entry.canonical), file)
            })
            .collect()
    });
    index
        .get(&(
            hero_id.to_string(),
            section_key.to_string(),
            canonical.to_string(),
        ))
        .cloned()
}
