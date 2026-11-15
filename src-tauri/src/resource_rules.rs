//! 新资源名称规则；展示分组不改变完整资源名和游戏应用槽位。
use crate::*;
use std::sync::OnceLock;

fn official_names() -> &'static Mutex<HashMap<String, (String, String)>> {
    static NAMES: OnceLock<Mutex<HashMap<String, (String, String)>>> = OnceLock::new();
    NAMES.get_or_init(|| Mutex::new(read_names()))
}
fn read_names() -> HashMap<String, (String, String)> {
    let mut values: Vec<serde_json::Value> =
        serde_json::from_str(HERO_MANIFEST).unwrap_or_default();
    values.extend(crate::workspace_catalog::entries().iter().cloned());
    if let Ok(entries) = fs::read_dir(data_dir().join("assets/maps")) {
        for e in entries.flatten() {
            if let Ok(bytes) = fs::read(e.path()) {
                if let Ok(mut rows) = serde_json::from_slice::<Vec<serde_json::Value>>(&bytes) {
                    values.append(&mut rows);
                }
            }
        }
    }
    values
        .iter()
        .filter_map(|v| {
            Some((
                v["canonical"].as_str()?.to_string(),
                (
                    v["section"].as_str()?.to_string(),
                    v["heroId"]
                        .as_str()
                        .or_else(|| v["entityId"].as_str())?
                        .to_string(),
                ),
            ))
        })
        .collect()
}
pub fn reload_catalog_names() {
    *official_names().lock().unwrap() = read_names();
}
pub(crate) fn special_name(name: &str) -> Option<Parsed> {
    let base = name.strip_suffix(".png").unwrap_or(name);
    if let Some((mut section, id)) = official_names().lock().unwrap().get(base).cloned() {
        if base.starts_with("UT_Hero_ProfilePhoto_") && id.parse::<u32>().is_ok_and(|v| v > 999) {
            section = "monster_profile".into();
        }
        return Some(Parsed {
            hero_id: id,
            section_key: section,
            canonical: base.into(),
            source: base.into(),
        });
    }
    if matches!(base, "cursor_up" | "cursor_down") {
        return Some(Parsed {
            hero_id: "cursor".into(),
            section_key: "cursor".into(),
            canonical: base.into(),
            source: base.into(),
        });
    }
    if let Some(c) = Regex::new(r"^UT_Relic_(\d+)(?:_\d+)?$")
        .ok()?
        .captures(base)
    {
        return Some(Parsed {
            hero_id: c[1].into(),
            section_key: "relic".into(),
            canonical: base.into(),
            source: base.into(),
        });
    }
    if let Some(c) = Regex::new(r"^UT_Relic_([A-Za-z]+)$").ok()?.captures(base) {
        return Some(Parsed {
            hero_id: c[1].into(),
            section_key: "relic_icon".into(),
            canonical: base.into(),
            source: base.into(),
        });
    }
    if let Some(c) = Regex::new(r"^UT_Hero_ProfilePhoto_(\d+)(?:_[A-Za-z0-9_]+)?$")
        .ok()?
        .captures(base)
    {
        if c[1].parse::<u32>().ok()? > 999 {
            return Some(Parsed {
                hero_id: c[1].into(),
                section_key: "monster_profile".into(),
                canonical: base.into(),
                source: base.into(),
            });
        }
    }
    if let Some(core) = base.strip_suffix("_sfw") {
        let mut parsed = parse_filename(core)?;
        parsed.canonical = base.into();
        parsed.source = base.into();
        return Some(parsed);
    }
    None
}
pub(crate) fn view_category(section: &str) -> String {
    match section {
        "land" | "platform" => "lands".into(),
        "relic" | "relic_icon" => "chips".into(),
        "cursor" => "cursors".into(),
        "temporary" => "temporary".into(),
        _ => category_of(section),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserve_special_assets() {
        for name in [
            "UT_Relic_50075_1",
            "UT_Relic_Attack",
            "cursor_up",
            "cursor_down",
            "UT_HandCard_20006_sfw",
        ] {
            assert_eq!(parse_filename(name).unwrap().canonical, name)
        }
        assert_eq!(
            parse_filename("UT_Hero_ProfilePhoto_1001")
                .unwrap()
                .section_key,
            "monster_profile"
        );
    }
}
