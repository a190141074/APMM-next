//! 作者流程只写草稿区；资源库与游戏目录由独立的安装流程负责。
use crate::*;
pub(crate) mod cover;
pub(crate) mod credentials;
pub(crate) mod offline;
pub(crate) mod remote;
pub(crate) mod snapshot;
pub(crate) mod sources;
pub(crate) mod store;
mod types;
pub(crate) use remote::upload;
use store::*;
use types::SubmissionSnapshot;
pub(crate) use types::*;
#[cfg(test)]
mod cover_tests;
#[cfg(test)]
mod tests;

pub(super) fn checked_id(id: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| "作者记录编号无效".into())
}
