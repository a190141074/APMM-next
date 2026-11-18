//! 工作台 IPC 契约。生成的 TypeScript 类型与 Rust 返回值保持同源。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct ModFile {
    pub hash: String,
    pub source_name: String,
    pub path: String,
    #[ts(optional)]
    pub info: Option<crate::management_types::CandidateInfo>,
    #[ts(optional)]
    pub classification: Option<crate::management_types::ClassificationRecord>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct ResourceRecord {
    pub key: String,
    pub category: String,
    pub entity_id: String,
    pub entity_name: String,
    #[serde(default)]
    pub name_known: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub section: String,
    pub section_label: String,
    #[serde(default)]
    pub resource_label: String,
    #[serde(default)]
    pub variant_key: String,
    #[serde(default)]
    pub variant_label: String,
    #[serde(default)]
    pub variant_order: i32,
    pub canonical: String,
    pub group_key: String,
    pub full_name: String,
    pub application_slot: String,
    pub original_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub outimg_path: Option<String>,
    #[serde(default)]
    pub original_status: String,
    pub files: Vec<ModFile>,
    pub active_hash: Option<String>,
    pub disabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct WorkspaceSnapshot {
    #[serde(default)]
    pub revision: u32,
    pub version: String,
    pub data_dir: String,
    pub game_dir: Option<String>,
    pub resources: Vec<ResourceRecord>,
    pub settings: WorkspaceSettings,
    pub migrated: bool,
}

/// 增量仅更新展示资源；版本断档或工作区配置变化时重新读取完整快照。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct WorkspaceDelta {
    pub base_revision: u32,
    pub revision: u32,
    pub resources: Vec<ResourceRecord>,
    pub removed_keys: Vec<String>,
    pub full_refresh: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct PresetSummary {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct WorkspaceSettings {
    pub author_mode: bool,
    #[ts(optional)]
    pub appearance: Option<crate::appearance::AppearanceSettings>,
    pub outimg_path: String,
    pub plugin_platform: Option<String>,
    pub search_mode: String,
    pub presets: Vec<PresetSummary>,
    #[ts(optional)]
    pub monitor_directories: Option<bool>,
    pub update_auto_check: bool,
    pub update_interval_hours: u32,
    pub update_skipped_version: Option<String>,
    pub update_last_checked_at: Option<f64>,
}
impl From<&crate::storage::Settings> for WorkspaceSettings {
    fn from(s: &crate::storage::Settings) -> Self {
        Self {
            author_mode: s.author_mode,
            appearance: s.appearance.clone().filter(|value| value.validate().is_ok()),
            outimg_path: s.outimg_path.clone(),
            monitor_directories: Some(s.monitor_directories),
            plugin_platform: s.plugin_platform.clone(),
            search_mode: s.search_mode.clone().unwrap_or_else(|| "smart".into()),
            presets: s
                .presets
                .iter()
                .map(|p| PresetSummary {
                    id: p.id.clone(),
                    name: p.name.clone(),
                })
                .collect(),
            update_auto_check: s.update_auto_check,
            update_interval_hours: s.update_interval_hours,
            update_skipped_version: s.update_skipped_version.clone(),
            update_last_checked_at: s.update_last_checked_at.map(|v| v as f64),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct TaskSnapshot {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub phase: String,
    pub message: String,
    pub done: f64,
    pub total: Option<f64>,
    pub speed: Option<f64>,
    pub revision: u32,
    pub can_cancel: bool,
    #[serde(default)]
    pub can_retry: bool,
    #[ts(type = "unknown")]
    pub result: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct ResourceAction {
    pub entity_id: String,
    pub section: String,
    pub canonical: String,
    pub hash: Option<String>,
}
