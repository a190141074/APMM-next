//! 工坊领域入口；目录、归档、安装与作者流程分别拥有自己的状态。
use crate::*;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
mod archive;
pub(crate) mod attribution;
#[cfg(test)]
mod behavior_tests;
mod catalog;
mod install;
mod plan;
#[cfg(test)]
mod tests;
mod types;
pub(crate) use archive::validate_png;
use archive::*;
pub(crate) use catalog::safe_source;
use catalog::*;
pub(crate) use catalog::{workshop_image, workshop_list};
pub(crate) use install::{workshop_install, workshop_uninstall};
pub(crate) use plan::workshop_preflight;
pub(crate) use plan::InstallRequest;
pub(crate) use types::*;
const MAX_MANIFEST: u64 = 2 * 1024 * 1024;
const MAX_ARCHIVE: u64 = 64 * 1024 * 1024;
const MAX_UNPACKED: u64 = 256 * 1024 * 1024;
const PUBLISHER_KEY_B64: &str = "UI39wMOvv7FSpxLE4fOlkOv/QK4yPMrk6eblJNTS8zU=";
