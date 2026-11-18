//! 工坊目录、资源归属与界面快照的同源契约。
use crate::*;
#[derive(Debug, Clone, Serialize, Deserialize, Default, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OwnedFile {
    pub(crate) hero_id: String,
    pub(crate) section_key: String,
    pub(crate) canonical: String,
    pub(crate) hash: String,
    pub(crate) source_name: String,
    pub(crate) rel_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstalledWork {
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) archive_hash: String,
    pub(crate) files: Vec<OwnedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct WorkFile {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) source_name: String,
    #[serde(default)]
    pub(crate) canonical: String,
    pub(crate) sha256: String,
    #[ts(type = "number")]
    pub(crate) bytes: u64,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub(crate) resource: Option<WorkshopResource>,
}

#[derive(Debug, Clone, Serialize, Default, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct WorkshopResource {
    pub(crate) category: String,
    pub(crate) category_name: String,
    pub(crate) entity_id: String,
    pub(crate) entity_name: String,
    pub(crate) section_key: String,
    pub(crate) section_name: String,
    pub(crate) canonical: String,
    pub(crate) resource_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ts_rs::TS)]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct WorkTag {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) highlight: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct Work {
    pub(crate) id: String,
    /// 公开编号与本地安装键分离，更新作品时保持稳定。
    #[serde(default)]
    pub(crate) published_id: Option<String>,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) category: String,
    #[serde(default)]
    pub(crate) tags: Vec<WorkTag>,
    #[serde(default)]
    pub(crate) author: String,
    #[serde(default)]
    pub(crate) artist: String,
    #[serde(default)]
    pub(crate) author_urls: Vec<String>,
    #[serde(default)]
    pub(crate) artist_urls: Vec<String>,
    #[serde(default)]
    pub(crate) source_url: String,
    #[serde(default)]
    pub(crate) game_version: String,
    #[serde(default)]
    pub(crate) change_note: String,
    #[serde(default)]
    #[ts(type = "number")]
    pub(crate) updated_at: u64,
    #[serde(default)]
    #[ts(type = "number")]
    pub(crate) revision: u64,
    #[serde(default)]
    pub(crate) previews: Vec<String>,
    #[serde(default)]
    pub(crate) history: Vec<WorkRevision>,
    pub(crate) version: String,
    pub(crate) cover: String,
    pub(crate) archive: String,
    pub(crate) sha256: String,
    #[ts(type = "number")]
    pub(crate) bytes: u64,
    pub(crate) files: Vec<WorkFile>,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub(crate) cover_data_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) schema: u32,
    pub(crate) works: Vec<Work>,
}

#[derive(Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct ListedWork {
    pub(crate) work: Work,
    pub(crate) installed_version: Option<String>,
    pub(crate) locally_modified: bool,
    pub(crate) update_available: bool,
}

#[derive(Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct Listing {
    pub(crate) available: bool,
    pub(crate) status: String,
    pub(crate) message: String,
    pub(crate) works: Vec<ListedWork>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct WorkRevision {
    pub version: String,
    pub change_note: String,
    #[ts(type = "number")]
    pub published_at: u64,
}
impl WorkFile {
    pub(crate) fn source(&self) -> &str {
        if self.source_name.is_empty() {
            &self.name
        } else {
            &self.source_name
        }
    }
    pub(crate) fn parsed(&self) -> Result<crate::Parsed, String> {
        let name = if self.canonical.is_empty() {
            self.source().to_string()
        } else {
            format!("{}.png", self.canonical)
        };
        let parsed = crate::parse_filename(&name).ok_or("资源尚未完成分类")?;
        if !self.canonical.is_empty() && parsed.canonical != self.canonical {
            return Err("资源分类不是完整名称".into());
        }
        Ok(parsed)
    }
}
