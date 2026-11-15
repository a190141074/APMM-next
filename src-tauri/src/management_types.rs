//! 管理功能的持久契约；可分享来源与本机备注分别保存，预设内容不随扫描重定向。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct Provenance {
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub author_urls: Vec<String>,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub artist_urls: Vec<String>,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub version: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct CandidateInfo {
    #[serde(default)]
    pub provenance: Provenance,
    #[serde(default)]
    pub note: String,
}
impl CandidateInfo {
    /// 合并导入记录时仅补空字段，目标记录和本机备注优先。
    pub fn fill_empty(&mut self, incoming: &Self) {
        for (target, source) in [
            (&mut self.provenance.author, &incoming.provenance.author),
            (&mut self.provenance.artist, &incoming.provenance.artist),
            (
                &mut self.provenance.source_url,
                &incoming.provenance.source_url,
            ),
            (&mut self.provenance.version, &incoming.provenance.version),
            (&mut self.note, &incoming.note),
        ] {
            if target.trim().is_empty() {
                *target = source.clone();
            }
        }
        for (target, source) in [
            (
                &mut self.provenance.author_urls,
                &incoming.provenance.author_urls,
            ),
            (
                &mut self.provenance.artist_urls,
                &incoming.provenance.artist_urls,
            ),
        ] {
            if target.is_empty() {
                *target = source.clone();
            }
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct ResourceSlot {
    pub category: String,
    pub hero_id: String,
    pub section_key: String,
    pub canonical: String,
}
impl ResourceSlot {
    pub fn from_parsed(p: &crate::Parsed) -> Self {
        Self {
            category: crate::category_of(&p.section_key),
            hero_id: p.hero_id.clone(),
            section_key: p.section_key.clone(),
            canonical: p.canonical.clone(),
        }
    }
    pub fn valid(&self) -> bool {
        crate::parse_filename(&self.canonical).is_some_and(|p| Self::from_parsed(&p) == *self)
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct ClassificationRecord {
    /// 记录临时图片上次观察的游戏禁用状态，同一状态不覆盖用户的库内启禁选择。
    #[serde(default)]
    pub external_disabled: Option<bool>,
    #[serde(default)]
    pub manual: bool,
    #[serde(default)]
    pub original_name: String,
    #[serde(default)]
    pub confirmed_rule: Option<String>,
    #[serde(default)]
    pub pending: Option<ResourceSlot>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct PresetItem {
    pub category: String,
    pub hero_id: String,
    pub section_key: String,
    pub canonical: String,
    /// 空哈希表示原版；旧预设的字符串哈希保持原样读取。
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub display_name: String,
}
impl PresetItem {
    pub fn slot(&self) -> ResourceSlot {
        // 旧预设将鼠标、地块和卡面等都写入 heroes 容器；保留原记录，只按完整身份解析实际槽位。
        if self.category == "heroes" {
            if let Some(parsed) = crate::parse_filename(&self.canonical) {
                let slot = ResourceSlot::from_parsed(&parsed);
                if slot.category != "monsters"
                    && slot.hero_id == self.hero_id
                    && slot.section_key == self.section_key
                {
                    return slot;
                }
            }
        }
        ResourceSlot {
            category: self.category.clone(),
            hero_id: self.hero_id.clone(),
            section_key: self.section_key.clone(),
            canonical: self.canonical.clone(),
        }
    }
}
fn global_scope() -> String {
    "global".into()
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct GlobalPreset {
    pub id: String,
    pub name: String,
    pub items: Vec<PresetItem>,
    #[serde(default = "global_scope")]
    pub scope: String,
    #[serde(default)]
    pub locked: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct PresetChange {
    pub item: PresetItem,
    pub saved_path: Option<String>,
    pub current_path: Option<String>,
    pub current_hash: Option<String>,
    pub changed: bool,
    pub issues: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct PresetPreview {
    pub preset: GlobalPreset,
    pub changes: Vec<PresetChange>,
    pub issues: Vec<String>,
    pub can_apply: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalBinding {
    pub hash: String,
    pub slot: ResourceSlot,
}
