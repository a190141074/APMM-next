//! 工作台与网页共用的显示元数据；目录不保存应用状态，完整资源键保持不变。
use crate::{dto::ResourceRecord, *};
use std::sync::OnceLock;

pub const JSON: &str = include_str!("../../config/resources.json");

pub fn entries() -> &'static Vec<serde_json::Value> {
    static DATA: OnceLock<Vec<serde_json::Value>> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(JSON).expect("内置资源目录必须通过构建校验"))
}

pub fn record(lib: &Library, section: &str, canonical: &str, id: &str) -> ResourceRecord {
    static RECORDS: OnceLock<HashMap<String, ResourceRecord>> = OnceLock::new();
    let records = RECORDS.get_or_init(|| {
        serde_json::from_str::<Vec<ResourceRecord>>(JSON)
            .expect("内置工作台目录格式无效")
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect()
    });
    let key = format!("{section}/{canonical}");
    let mut row = records.get(&key).cloned().unwrap_or_else(|| {
        let category = crate::resource_rules::view_category(section);
        let name = match category.as_str() {
            "heroes" | "lands" => display_name_of(lib, id),
            "monsters" => monster_display_name(lib, id),
            "cards" => card_display_name(section, canonical).unwrap_or_else(|| id.into()),
            "cursors" => "默认鼠标".into(),
            _ => id.into(),
        };
        let label = section_defs()
            .iter()
            .find(|s| s.key == section)
            .map(|s| s.label.clone())
            .unwrap_or_else(|| section.into());
        let shared = matches!(section, "account_background" | "item_icons");
        ResourceRecord {
            key: key.clone(),
            category: category.clone(),
            entity_id: id.into(),
            name_known: name != id,
            entity_name: if name == id {
                format!("未收录名称 · {id}")
            } else {
                name
            },
            section: section.into(),
            section_label: label.clone(),
            resource_label: label,
            canonical: canonical.into(),
            full_name: canonical.into(),
            application_slot: canonical.into(),
            group_key: match category.as_str() {
                "lands" => format!("land:{id}"),
                "cursors" => "cursor".into(),
                _ => key,
            },
            variant_key: if shared {
                "shared".into()
            } else {
                template_variant(canonical)
            },
            variant_label: if shared {
                "公共资源".into()
            } else {
                variant_label(canonical)
            },
            variant_order: if shared {
                1000
            } else {
                variant_order(canonical)
            },
            original_status: "not-extracted".into(),
            ..Default::default()
        }
    });
    if let Some(name) = lib
        .settings
        .display_names
        .get(id)
        .filter(|n| !n.trim().is_empty())
    {
        row.aliases.push(row.entity_name.clone());
        row.entity_name = name.clone();
        row.name_known = true;
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_catalog_preserves_names_variants_and_sections() {
        let rows: Vec<ResourceRecord> = serde_json::from_str(JSON).unwrap();
        assert_eq!(
            rows.iter()
                .map(|r| &r.section)
                .collect::<HashSet<_>>()
                .len(),
            22
        );
        assert_eq!(
            rows.iter().map(|r| &r.key).collect::<HashSet<_>>().len(),
            rows.len()
        );
        for (canonical, name) in [
            ("UT_Hero_Card_101", "帕露南"),
            ("UT_Hero_Card_102", "芬妮"),
            ("UT_Hero_Card_108", "米米"),
            ("UT_Relic_50029", "魔法秘典"),
            ("UT_Relic_50031", "智能手表"),
            ("UT_Relic_50040", "缓冲盾牌"),
        ] {
            assert_eq!(
                rows.iter()
                    .find(|r| r.canonical == canonical)
                    .unwrap()
                    .entity_name,
                name
            );
        }
        assert!(rows.iter().any(|r| r.canonical == "UT_Relic_50075_1"));
        assert!(rows.iter().any(|r| r.variant_key == "Max"));
        assert!(rows
            .iter()
            .filter(|r| r.section == "account_background")
            .all(|r| r.variant_key == "shared"));
    }
}
