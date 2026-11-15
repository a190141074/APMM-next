//! 从旧版迁入的领域实现；保留协议与行为，在明确边界内逐步替换。
use crate::*;

/* ============ 命名规范解析 ============ */

pub(crate) fn is_censored(name: &str) -> bool {
    name.to_ascii_lowercase().contains("sfw")
}

/// busts 专用：完整结构 UT_Hero_Bust_{id}[_Max|_0N]_{idx}[_dup]
///   _0/_1 是战斗交互索引（独立可替换条目，modimg 按完整文件名加载）→ 必须保留
///   末尾两位数字段（_02/_03/…）是解包重复副本 → 剥离
pub(crate) fn canonicalize_busts(base: &str) -> Option<String> {
    let re = Regex::new(r"^(UT_Hero_Bust_(\d+?))(?:_(Max|[0-9]{2}))?_(\d)(?:_\d{2,})?$").ok()?;
    let caps = re.captures(base)?;
    let id = caps.get(2)?.as_str();
    if caps.get(3).map(|m| m.as_str()) == Some("Max") && !id.starts_with('1') {
        return None;
    }
    if let Some(skin) = caps.get(3).map(|m| m.as_str()) {
        if skin != "Max"
            && skin
                .parse::<u32>()
                .ok()
                .filter(|n| (1..=99).contains(n))
                .is_none()
        {
            return None;
        }
    }
    let mut c = caps.get(1)?.as_str().to_string();
    if let Some(skin) = caps.get(3) {
        c.push('_');
        c.push_str(skin.as_str());
    }
    c.push('_');
    c.push_str(caps.get(4)?.as_str());
    Some(c)
}

/// 严格解析：前缀_数字ID + 可选后缀
/// flag:
///   None         - 只保留第一个有效段（Max 或 0N），其他尾段丢弃
///   "stripSkin"  - item_icons 专用：任何后缀都丢弃，只剩 UT_Item_Hero_<heroId>
pub(crate) fn canonicalize_strict(base: &str, flag: Option<&str>) -> Option<String> {
    let re = Regex::new(r"^(UT_[A-Za-z0-9_]+?)_(\d+)(.*)$").ok()?;
    let caps = re.captures(base)?;
    let prefix = caps.get(1)?.as_str();
    let id = caps.get(2)?.as_str();
    let rest = caps.get(3).map(|m| m.as_str()).unwrap_or("");

    if flag == Some("stripSkin") {
        return Some(format!("{}_{}", prefix, id));
    }

    let mut skin_segment: Option<String> = None;
    for seg in rest.split('_').filter(|s| !s.is_empty()) {
        if seg.eq_ignore_ascii_case("max") {
            skin_segment = Some("Max".into());
            break;
        }
        if seg.len() == 2 && seg.starts_with('0') && seg.chars().nth(1).unwrap().is_ascii_digit() {
            if let Some(n) = seg.chars().nth(1).and_then(|c| c.to_digit(10)) {
                if (1..=9).contains(&n) {
                    skin_segment = Some(seg.to_string());
                    break;
                }
            }
        }
    }
    let base_canon = format!(
        "{}_{}{}",
        prefix,
        id,
        skin_segment
            .as_deref()
            .map(|s| format!("_{}", s))
            .unwrap_or_default()
    );
    Some(base_canon)
}

/// 英雄贴图按部位解析，而不是把任意尾部数字都猜成皮肤。
/// 解包器的“同名不覆盖”会在合法文件后追加 `_02` / `_03`；这里仅保留
/// 第一个合法皮肤段，额外数字一律视为解包重复后缀。
pub(crate) fn canonicalize_hero_resource(base: &str, section_key: &str) -> Option<String> {
    let (prefix, allows_skin) = match section_key {
        "card" => ("UT_Hero_Card", true),
        "card2" => ("UT_Hero_Card2", true),
        "thin_card" => ("UT_Hero_ThinCard", true),
        "level_up" => ("UT_Hero_LevelUp", true),
        "profile_photos" => ("UT_Hero_ProfilePhoto", true),
        "role_photos" => ("UT_Hero_RolePhoto", true),
        "standing_paintings" => ("UT_Item_StandingPainting", true),
        "card_in_game" => ("UT_Hero_InGameCard", true),
        "item_icons" => ("UT_Item_Hero", false),
        "land" => ("UT_Land", false),
        "platform" => ("UT_Platform", false),
        _ => return None,
    };
    let re = Regex::new(&format!(
        r"^{}_([0-9]+)(?:_(Max|[0-9]{{2}}))?(?:_[0-9]+)*$",
        regex::escape(prefix)
    ))
    .ok()?;
    let caps = re.captures(base)?;
    let id = caps.get(1)?.as_str();
    let skin = caps.get(2).map(|m| m.as_str());
    if !allows_skin || section_key == "item_icons" {
        return Some(format!("{}_{}", prefix, id));
    }
    Some(match skin {
        Some("Max") if id.starts_with('1') => format!("{}_{}_Max", prefix, id),
        Some("Max") => return None,
        Some(value) if value != "00" => format!("{}_{}_{}", prefix, id, value),
        Some(_) => return None,
        None => format!("{}_{}", prefix, id),
    })
}

pub(crate) fn canonicalize_account_background(base: &str) -> Option<String> {
    let re = Regex::new(r"^(UT_AccountBackground_([0-9]+)_([0-9]{2}))(?:_[0-9]+)*$").ok()?;
    let caps = re.captures(base)?;
    Some(caps.get(1)?.as_str().to_string())
}

pub(crate) fn canonicalize_for_section(
    base: &str,
    section_key: &str,
    strip_skin: bool,
) -> Option<String> {
    if base.is_empty() || is_censored(base) {
        return None;
    }
    if section_key == "busts" {
        // busts 必须有战斗交互索引 _0/_1/...，缺索引的裸 canonical 无效
        return canonicalize_busts(base);
    }
    if section_key == "monster_bust" {
        // 怪物战斗交互：保留末尾战斗交互索引 _0/_1，剥离解包重复后缀 _02 等
        let re = Regex::new(r"^UT_Monster_Bust_(\d+)_(\d+)(?:_\d{2,})?$").ok()?;
        let caps = re.captures(base)?;
        return Some(format!(
            "UT_Monster_Bust_{}_{}",
            caps.get(1)?.as_str(),
            caps.get(2)?.as_str()
        ));
    }
    if section_key == "account_background" {
        return canonicalize_account_background(base);
    }
    // Card packages can contain multiple replacements for one card slot.  A
    // duplicate extracted filename receives a numeric suffix in the archive
    // (for example UT_HandCard_10007_02); keep that file as a separate
    // resource while resolving it to the same canonical card slot.
    if matches!(section_key, "hand_card" | "event_card" | "map_event") {
        let prefix = match section_key {
            "hand_card" => "UT_HandCard",
            "event_card" => "UT_Event",
            "map_event" => "UT_MapEvent",
            _ => unreachable!(),
        };
        let re = Regex::new(&format!(r"^{}_([0-9]+)(?:_\d{{2,}})?$", prefix)).ok()?;
        let caps = re.captures(base)?;
        return Some(format!("{}_{}", prefix, caps.get(1)?.as_str()));
    }
    if matches!(
        section_key,
        "card"
            | "card2"
            | "thin_card"
            | "level_up"
            | "profile_photos"
            | "role_photos"
            | "standing_paintings"
            | "card_in_game"
            | "item_icons"
            | "land"
            | "platform"
    ) {
        return canonicalize_hero_resource(base, section_key);
    }
    let flag = if strip_skin { Some("stripSkin") } else { None };
    if let Some(c) = canonicalize_strict(base, flag) {
        return Some(c);
    }
    let re = Regex::new(r"(UT_[A-Za-z0-9_]*\d+)").ok()?;
    let m = re.captures(base)?;
    canonicalize_strict(m.get(1)?.as_str(), flag)
}

pub(crate) fn detect_section(name: &str) -> Option<(String, String)> {
    for s in section_defs() {
        if let Ok(re) = Regex::new(&s.regex) {
            if let Some(c) = re.captures(name) {
                if let Some(id) = c.get(1) {
                    return Some((s.key.clone(), id.as_str().to_string()));
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Parsed {
    pub(crate) hero_id: String,
    pub(crate) section_key: String,
    pub(crate) canonical: String,
    pub(crate) source: String,
}

/// Only actual hero portraits use the game's 1..=999 hero id namespace.
/// `land`, `platform`, and account backgrounds use other identifiers even
/// though their files live in the broader hero asset tree.  Monster profile
/// photos can share the `UT_Hero_ProfilePhoto` prefix. Out-of-range IDs must
/// not be placed in the hero category.
pub(crate) fn is_hero_character_section(section_key: &str) -> bool {
    !matches!(section_key, "land" | "platform" | "account_background")
        && section_defs()
            .iter()
            .any(|section| section.key == section_key && section.category == "heroes")
}

pub(crate) fn valid_hero_character_id(section_key: &str, id: &str) -> bool {
    !is_hero_character_section(section_key)
        || id.parse::<u32>().map(|value| value <= 999).unwrap_or(false)
}

pub(crate) fn parse_filename(fname: &str) -> Option<Parsed> {
    if let Some(parsed) = crate::resource_rules::special_name(fname) {
        return Some(parsed);
    }
    let base = fname.rsplit_once('.').map(|(a, _)| a).unwrap_or(fname);
    // 优先尝试每个 section 的特殊规则（busts/item_icons），确保 canonical 与 detect_section 一致
    for s in section_defs() {
        let re = Regex::new(&s.regex).ok()?;
        if !re.is_match(base) {
            continue;
        }
        if let Some(c) = canonicalize_for_section(base, &s.key, s.strip_skin) {
            if let Some(caps) = re.captures(&c) {
                if let Some(id) = caps.get(1) {
                    if !valid_hero_character_id(&s.key, id.as_str()) {
                        // Another section may share the filename prefix.
                        continue;
                    }
                    return Some(Parsed {
                        hero_id: id.as_str().to_string(),
                        section_key: s.key.clone(),
                        canonical: c,
                        source: base.to_string(),
                    });
                }
            }
        }
    }
    // 兜底：默认规则（仅用于非 busts 段；busts 必须带战斗交互索引）
    if let Some(c) = canonicalize_for_section(base, "", false) {
        if let Some((key, hero)) = detect_section(&c) {
            if !valid_hero_character_id(&key, &hero) {
                return None;
            }
            if key == "busts" && canonicalize_busts(base).is_none() {
                return None;
            }
            return Some(Parsed {
                hero_id: hero,
                section_key: key,
                canonical: c,
                source: base.to_string(),
            });
        }
    }
    None
}

/// 剥离 busts 末尾的战斗交互索引（_0/_1 单数字段），返回皮肤层 canonical
pub(crate) fn strip_combat_index(c: &str) -> String {
    let re = Regex::new(r"_(\d)$").unwrap();
    re.replace(c, "").to_string()
}

/// 返回模板变体键：`base`、`01`…或 `Max`。名片、地块、物品图标没有皮肤变体。
pub(crate) fn template_variant(c: &str) -> String {
    if c.starts_with("UT_AccountBackground_")
        || c.starts_with("UT_Land_")
        || c.starts_with("UT_Platform_")
        || c.starts_with("UT_Item_Hero_")
    {
        return "base".into();
    }
    let core = strip_combat_index(c);
    let re = Regex::new(r"_(Max|[0-9]{2})$").unwrap();
    re.captures(&core)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "base".into())
}

pub(crate) fn is_valid_template_variant(hero_id: &str, variant: &str) -> bool {
    if variant == "base" {
        return true;
    }
    if variant.eq_ignore_ascii_case("Max") {
        return hero_id.starts_with('1');
    }
    variant
        .parse::<u32>()
        .map(|n| (1..=99).contains(&n))
        .unwrap_or(false)
}

pub(crate) fn variant_label(c: &str) -> String {
    let variant = template_variant(c);
    if variant.eq_ignore_ascii_case("max") {
        return "契约皮".into();
    }
    if variant != "base" {
        let n: usize = variant.parse().unwrap_or(0);
        let names = ["一", "二", "三", "四", "五", "六", "七", "八", "九"];
        return if (1..=9).contains(&n) {
            format!("皮肤{}", names[n - 1])
        } else {
            format!("皮肤{}", n)
        };
    }
    "原皮".into()
}

pub(crate) fn variant_order(c: &str) -> i32 {
    let variant = template_variant(c);
    if variant.eq_ignore_ascii_case("max") {
        return 1;
    }
    variant.parse::<i32>().map(|n| n + 1).unwrap_or(0)
}

pub(crate) fn canonical_for_template(section: &str, id: &str, variant: &str) -> Option<String> {
    let suffix = if variant == "base" {
        "".to_string()
    } else {
        format!("_{}", variant)
    };
    let prefix = match section {
        "card" => "UT_Hero_Card",
        "card2" => "UT_Hero_Card2",
        "thin_card" => "UT_Hero_ThinCard",
        "item_icons" => "UT_Item_Hero",
        "level_up" => "UT_Hero_LevelUp",
        "profile_photos" => "UT_Hero_ProfilePhoto",
        "role_photos" => "UT_Hero_RolePhoto",
        "standing_paintings" => "UT_Item_StandingPainting",
        "card_in_game" => "UT_Hero_InGameCard",
        "land" => "UT_Land",
        "platform" => "UT_Platform",
        _ => return None,
    };
    if matches!(section, "item_icons" | "land" | "platform") && variant != "base" {
        return None;
    }
    if section == "standing_paintings" && variant == "base" {
        return None;
    }
    Some(format!("{}_{}{}", prefix, id, suffix))
}

pub(crate) fn required_template_resources(id: &str, variant: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if variant == "base" {
        out.push(("busts".into(), format!("UT_Hero_Bust_{}_0", id)));
        out.push(("busts".into(), format!("UT_Hero_Bust_{}_1", id)));
        for section in [
            "card",
            "card2",
            "thin_card",
            "item_icons",
            "level_up",
            "profile_photos",
            "role_photos",
            "land",
            "platform",
        ] {
            if let Some(canonical) = canonical_for_template(section, id, variant) {
                out.push((section.into(), canonical));
            }
        }
    } else {
        out.push(("busts".into(), format!("UT_Hero_Bust_{}_{}_0", id, variant)));
        out.push(("busts".into(), format!("UT_Hero_Bust_{}_{}_1", id, variant)));
        for section in [
            "card",
            "card2",
            "thin_card",
            "level_up",
            "profile_photos",
            "role_photos",
            "standing_paintings",
        ] {
            if let Some(canonical) = canonical_for_template(section, id, variant) {
                out.push((section.into(), canonical));
            }
        }
    }
    out
}
