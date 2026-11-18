use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use aes_gcm::{
    aead::{Aead, Payload},
    Aes256Gcm, KeyInit, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_RANGE, RANGE};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Emitter;

use crate::remote_source::{RemoteSourcePool, Source, SourceKind};

mod app;
mod appearance;
mod appearance_commands;
mod candidate_management;
mod catalog;
mod classification;
mod commands;
mod diagnostics;
mod directory_files;
mod directory_monitor;
mod directory_observation;
mod directory_scan;
mod directory_sync;
mod directory_watch;
mod file_verification;
mod image_drop;
mod helper_process;
#[cfg(test)]
mod incremental_tests;
mod installation;
mod library;
mod logs;
mod management_types;
mod migration_journal;
mod migration_merge;
mod migration_detection;
mod msi_update;
mod original_cache;
mod package;
mod packages;
mod preset_domain;
mod presets;
#[cfg(windows)]
mod screen_picker;
mod update_gate;
mod update_install;
mod vault_sync;
mod workspace_cache;
pub(crate) use management_types::GlobalPreset;
mod author;
mod remote_source;
mod scan;
mod scanner;
mod settings_commands;
mod task_history;
mod thumbnails;
mod update_bootstrap;
mod update_channel;
mod update_package;
mod updates;
mod vault;
mod workshop;
mod workspace_catalog;

pub(crate) use catalog::*;
pub(crate) use diagnostics::{
    log_client_error, log_mutation, logs_export, logs_reveal, logs_status, write_log,
};
pub(crate) use packages::*;
pub(crate) use presets::{preset_apply, preset_delete, preset_restore_all, preset_save};
pub(crate) use settings_commands::{settings_get, settings_set};
pub(crate) use update_install::update_apply;
#[cfg(test)]
use updates::{find_update_payload, UPDATE_RUNTIME_FILES};
pub(crate) use updates::{update_cancel, update_check, update_download};
pub(crate) use workshop::{
    workshop_image, workshop_install, workshop_list, workshop_preflight, workshop_uninstall,
};

mod preview_engine;
mod preview_install;
pub(crate) use preview_engine::*;

mod apx;
mod apx_directories;
pub(crate) use apx::*;

mod naming;
pub(crate) use naming::*;

mod storage;
pub(crate) use storage::*;

mod asset_catalog;
pub(crate) use asset_catalog::*;

mod external_links;
mod game;
pub(crate) use game::*;

mod legacy_commands;
pub(crate) use legacy_commands::*;

mod resource_mutation;
pub(crate) use resource_mutation::*;

pub fn run() {
    app::run(tauri::generate_handler![
        appearance_commands::screen_pick_color,
        appearance_commands::theme_export,
        external_links::open_external,
        apx_directories::plugin_match_directory,
        apx_directories::plugin_set_directory,
        auxiliary::auxiliary_status,
        auxiliary::auxiliary_uninstall,
        auxiliary::auxiliary_check_update,
        image_import::images_inspect,
        image_drop::images_match_drop,
        updates::update_startup_status,
        updates::update_download_status,
        migration::migration_inspect,
        migration_detection::migration_status,
        task_dispatch::task_execute,
        workspace::workspace_snapshot,
        workspace::resources_apply,
        tasks::tasks_list,
        tasks::task_cancel,
        tasks::task_pause,
        tasks::task_retry,
        detect,
        resolve_manual,
        open_modimg,
        asset_scan,
        heroes_list,
        hero_detail,
        cards_detail,
        monsters_detail,
        import_files,
        import_files_to_target,
        set_active,
        clear_active,
        mod_set_disabled,
        remove_file,
        rename_file,
        reveal_file,
        reveal_mod_file,
        reconcile,
        scan_modimg_start,
        scan_modimg_status,
        temporary_mods,
        non_png_mods_check,
        reveal_non_png_mod,
        cleanup_library,
        logs_status,
        logs_reveal,
        logs_export,
        log_client_error,
        log_mutation,
        credential_default_password_get,
        credential_default_password_set,
        credential_default_password_clear,
        package_export_candidates,
        package_export,
        quick_export,
        package_import_inspect,
        package_import_commit,
        package_import_queue_begin,
        package_import_queue_add,
        package_import_queue_preview,
        package_import_queue_preview_image,
        package_import_queue_commit,
        package_import_queue_cancel,
        settings_get,
        settings_set,
        outimg_reveal,
        directory_monitor::directory_sync_status,
        candidate_management::candidate_info_set,
        candidate_management::candidate_classify,
        presets::preset_create,
        presets::preset_detail,
        presets::preset_update,
        presets::preset_copy,
        presets::preset_lock,
        preset_save,
        preset_delete,
        preset_apply,
        preset_restore_all,
        preview_status,
        preview_check_update,
        preview_download,
        preview_cancel,
        preview_clear,
        get_data_dir,
        file_base64,
        plugin_status,
        plugin_check_update,
        browser_drop_begin,
        browser_drop_append,
        browser_drop_finish,
        browser_drop_cancel,
        plugin_prepare_cache,
        plugin_install,
        plugin_restore,
        plugin_export,
        plugin_cancel,
        plugin_reveal,
        plugin_open_dir,
        update_check,
        update_download,
        update_apply,
        update_cancel,
        workshop_list,
        workshop_image,
        workshop_preflight,
        workshop_install,
        workshop_uninstall,
        author::store::author_state,
        author::store::author_new,
        author::store::author_save,
        author::sources::author_add_source,
        author::cover::author_set_cover,
        author::sources::author_add_library,
        author::sources::author_image,
        author::credentials::author_backup,
        author::credentials::author_restore,
        author::snapshot::author_preflight,
        author::snapshot::author_export,
        author::remote::author_service,
        author::remote::author_refresh,
        author::remote::author_cancel,
        author::offline::author_sync_published,
    ]);
}

/// 404 返回空体，触发前端 img 的 error 事件 → 走 data URL 兜底
fn not_found() -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(404)
        .header("Content-Type", "text/plain")
        .body(Vec::new())
        .unwrap()
}

#[cfg(test)]
#[path = "legacy_tests.rs"]
mod tests;

mod dto;
mod originals;
mod resource_rules;
mod staging;
mod tasks;
mod workspace;

mod aux_manifest;
mod auxiliary;
mod downloads;
mod image_import;
mod migration;
mod portable_update;
mod resource_transaction;
mod task_dispatch;

#[cfg(test)]
mod management_tests;

#[cfg(test)]
mod sync_management_tests;

#[cfg(test)]
mod preview_release_tests;
