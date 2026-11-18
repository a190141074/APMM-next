//! 作者草稿、不可变投稿快照和安全回执的 IPC 契约。
use crate::*;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct AuthorMetadata {
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub author_urls: Vec<String>,
    #[serde(default)]
    pub artist_urls: Vec<String>,
    #[serde(default)]
    pub source_url: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub version: String,
    pub game_version: String,
    pub change_note: String,
}

impl From<&crate::workshop::Work> for AuthorMetadata {
    fn from(work: &crate::workshop::Work) -> Self {
        Self {
            title: work.title.clone(),
            author: work.author.clone(),
            artist: work.artist.clone(),
            author_urls: work.author_urls.clone(),
            artist_urls: work.artist_urls.clone(),
            source_url: work.source_url.clone(),
            description: work.description.clone(),
            category: work.category.clone(),
            tags: work.tags.iter().map(|tag| tag.name.clone()).collect(),
            version: work.version.clone(),
            game_version: work.game_version.clone(),
            change_note: work.change_note.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct AuthorFile {
    pub id: String,
    pub source_name: String,
    pub canonical: String,
    pub sha256: String,
    #[ts(type = "number")]
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct AuthorDraft {
    pub id: String,
    #[ts(type = "number")]
    pub revision: u64,
    pub work_id: Option<String>,
    pub metadata: AuthorMetadata,
    pub files: Vec<AuthorFile>,
    pub images: Vec<AuthorFile>,
    pub metadata_only: bool,
    #[ts(type = "number")]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct AuthorReceipt {
    pub id: String,
    pub work_id: String,
    pub draft_id: String,
    pub snapshot_hash: String,
    pub status: String,
    pub message: String,
    #[ts(type = "number")]
    pub created_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct AuthorWork {
    pub id: String,
    pub title: String,
    #[ts(type = "number")]
    pub revision: u64,
    pub published: Option<crate::workshop::Work>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct AuthorState {
    pub drafts: Vec<AuthorDraft>,
    pub works: Vec<AuthorWork>,
    pub receipts: Vec<AuthorReceipt>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct AuthorPreview {
    pub id: String,
    pub draft: AuthorDraft,
    #[ts(type = "number")]
    pub bytes: u64,
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
    pub metadata_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SubmissionSnapshot {
    pub preview: AuthorPreview,
    pub base_revision: u64,
    pub work_id: String,
    pub archive_hash: String,
}
