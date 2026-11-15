//! Encrypted .apmod package domain.
//! IPC names and package persistence formats remain compatible.

use crate::*;
use aes_gcm::{
    aead::{Aead, Payload},
    Aes256Gcm, KeyInit, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

/* ============ Encrypted .apmod packages ============ */

pub(crate) const PACKAGE_MAGIC_V1: &[u8; 8] = b"APMMOD01";
pub(crate) const PACKAGE_MAGIC_V2: &[u8; 8] = b"APMMOD02";
pub(crate) const PACKAGE_MAGIC_V3: &[u8; 8] = b"APMMOD03";
pub(crate) const PACKAGE_V1_HEADER_LEN: usize = 36;
pub(crate) const PACKAGE_V2_HEADER_LEN: usize = 49;
pub(crate) const PACKAGE_KDF_ID_ARGON2ID: u8 = 1;
pub(crate) const PACKAGE_KDF_MEMORY_KIB: u32 = 65_536;
pub(crate) const PACKAGE_KDF_ITERATIONS: u32 = 3;
pub(crate) const PACKAGE_KDF_PARALLELISM: u32 = 1;
pub(crate) const PACKAGE_V1_KDF_MEMORY_KIB: u32 = 19_456;
pub(crate) const PACKAGE_V1_KDF_ITERATIONS: u32 = 2;
pub(crate) const PACKAGE_KDF_MAX_MEMORY_KIB: u32 = 131_072;
pub(crate) const PACKAGE_KDF_MAX_ITERATIONS: u32 = 8;
pub(crate) const PACKAGE_KDF_MAX_PARALLELISM: u32 = 4;
pub(crate) const PACKAGE_MAX_FILES: usize = 10_000;
pub(crate) const PACKAGE_MAX_BYTES: usize = 1024 * 1024 * 1024;
pub(crate) const PACKAGE_MIN_VOLUME_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const PACKAGE_DEFAULT_VOLUME_BYTES: u64 = 512 * 1024 * 1024;
pub(crate) const PACKAGE_V3_HEADER_LEN: usize = 68;
pub(crate) const PACKAGE_CHUNK_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageFileMeta {
    pub(crate) category: String,
    pub(crate) entity_id: String,
    pub(crate) section_key: String,
    pub(crate) canonical: String,
    pub(crate) hash: String,
    pub(crate) ext: String,
    pub(crate) source_name: String,
    /// Unique vault filename, distinct from the user-facing source name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) storage_name: Option<String>,
    pub(crate) was_enabled: bool,
    /// Optional presentation-only labels. They are ignored for identity and
    /// remain absent in older package manifests.
    #[serde(default)]
    pub(crate) entity_name: String,
    #[serde(default)]
    pub(crate) section_name: String,
    #[serde(default)]
    pub(crate) resource_name: String,
    #[serde(default)]
    pub(crate) category_name: String,
    #[serde(default)]
    pub(crate) category_order: u8,
    /// UI-only local vault path. This is omitted from package manifests by
    /// leaving it unset when the manifest is serialized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) preview_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) provenance: Option<crate::management_types::Provenance>,
    #[serde(default)]
    pub(crate) manual_classification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageApplicationState {
    #[serde(default)]
    enabled_resources: Vec<PackageSelection>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageManifest {
    format_version: u8,
    app_version: String,
    created_at: u64,
    files: Vec<PackageFileMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    application_state: Option<PackageApplicationState>,
}

/// Package metadata is untrusted input even after decryption.  A package may
/// carry a stable key from a future build or a hand-edited manifest; such an
/// item must never create an arbitrary library branch.  Keep it safely
/// importable by placing it in the deterministic temporary bucket instead.
pub(crate) fn normalized_package_meta(meta: &PackageFileMeta) -> PackageFileMeta {
    // Older packages may carry a stale category/section key.  The original
    // filename is the strongest portable signal, so apply the current parser
    // before trusting package metadata.  This makes cards land in their real
    // sections during preview and commit instead of waiting for a restart.
    let source_file = Path::new(&meta.source_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(meta.source_name.as_str());
    if meta.manual_classification
        && parse_filename(&meta.canonical).is_some_and(|p| {
            p.hero_id == meta.entity_id
                && p.section_key == meta.section_key
                && category_of(&p.section_key) == meta.category
        })
    {
        let mut safe = meta.clone();
        safe.preview_url = None;
        return safe;
    }
    if let Some(parsed) = parse_filename(source_file) {
        return PackageFileMeta {
            category: category_of(&parsed.section_key),
            entity_id: parsed.hero_id,
            section_key: parsed.section_key,
            canonical: parsed.canonical,
            hash: meta.hash.clone(),
            ext: meta.ext.clone(),
            source_name: meta.source_name.clone(),
            storage_name: meta.storage_name.clone(),
            was_enabled: meta.was_enabled,
            entity_name: String::new(),
            section_name: String::new(),
            resource_name: String::new(),
            category_name: String::new(),
            category_order: 0,
            preview_url: None,
            provenance: meta.provenance.clone(),
            manual_classification: meta.manual_classification,
        };
    }
    let valid = section_defs().iter().any(|section| {
        section.key == meta.section_key
            && section.category == meta.category
            && !meta.entity_id.trim().is_empty()
            && !meta.canonical.trim().is_empty()
    });
    if valid
        || ((meta.category == "heroes" || meta.category == "temporary")
            && meta.section_key == "temporary"
            && meta.entity_id.starts_with("temporary:"))
    {
        let mut safe = meta.clone();
        safe.preview_url = None;
        return safe;
    }
    PackageFileMeta {
        category: "heroes".into(),
        entity_id: format!("temporary:{}", meta.hash.to_ascii_lowercase()),
        section_key: "temporary".into(),
        canonical: format!("unmatched:{}", meta.hash.to_ascii_lowercase()),
        hash: meta.hash.clone(),
        ext: meta.ext.clone(),
        source_name: meta.source_name.clone(),
        was_enabled: false,
        storage_name: meta.storage_name.clone(),
        entity_name: String::new(),
        section_name: String::new(),
        resource_name: String::new(),
        category_name: "临时".into(),
        category_order: 3,
        preview_url: None,
        provenance: meta.provenance.clone(),
        manual_classification: meta.manual_classification,
    }
}

/// Package manifests are portable and may contain stale or absent labels.
/// The import review always renders names from this installation's current
/// metadata, while keeping the normalized stable key untouched for commit.
pub(crate) fn localize_package_meta(lib: &Library, mut meta: PackageFileMeta) -> PackageFileMeta {
    let logical_category = if meta.section_key == "temporary" {
        "temporary"
    } else {
        meta.category.as_str()
    };
    let (category_name, category_order) = logical_category_label(logical_category);
    meta.category_name = category_name;
    meta.category_order = category_order;
    if meta.section_key == "temporary" {
        meta.entity_name = "临时".into();
        meta.section_name = "临时".into();
        meta.resource_name = "临时资源".into();
        return meta;
    }

    let section = section_defs()
        .into_iter()
        .find(|section| section.key == meta.section_key);
    let is_card = section
        .as_ref()
        .map(|section| section.category == "cards")
        .unwrap_or(false);
    meta.entity_name = if meta.category == "monsters" {
        monster_display_name(lib, &meta.entity_id)
    } else if is_card {
        card_display_name(&meta.section_key, &meta.canonical)
            .unwrap_or_else(|| meta.entity_id.clone())
    } else {
        display_name_of(lib, &meta.entity_id)
    };
    meta.section_name = section
        .map(|section| section.label)
        .unwrap_or_else(|| meta.section_key.clone());
    meta.resource_name = if is_card {
        card_display_name(&meta.section_key, &meta.canonical)
            .unwrap_or_else(|| meta.canonical.clone())
    } else {
        variant_label(&meta.canonical)
    };
    meta.preview_url = None;
    meta
}

pub(crate) fn logical_category_for_section(section_key: &str, map_category: &str) -> String {
    if section_key == "temporary" {
        return "temporary".into();
    }
    section_defs()
        .into_iter()
        .find(|s| s.key == section_key)
        .map(|s| s.category)
        .unwrap_or_else(|| {
            if map_category == "monsters" {
                "monsters".into()
            } else {
                "other".into()
            }
        })
}

pub(crate) fn logical_category_label(category: &str) -> (String, u8) {
    match category {
        "heroes" => ("角色".into(), 0),
        "cards" => ("卡面".into(), 1),
        "monsters" => ("怪物".into(), 2),
        "temporary" => ("临时".into(), 3),
        _ => ("其他".into(), 4),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageSelection {
    category: String,
    entity_id: String,
    section_key: String,
    canonical: String,
    hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageExportArgs {
    destination: String,
    password: String,
    items: Vec<PackageSelection>,
    #[serde(default)]
    kdf_profile: Option<String>,
    #[serde(default)]
    include_application_state: bool,
    #[serde(default = "default_volume_bytes")]
    max_volume_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuickExportArgs {
    destination: String,
    mode: String,
    items: Vec<PackageSelection>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageExportResult {
    path: String,
    files: usize,
    bytes: usize,
    #[serde(default)]
    volumes: Vec<String>,
}

fn default_volume_bytes() -> u64 {
    PACKAGE_DEFAULT_VOLUME_BYTES
}

struct PackageTempFile(PathBuf);
impl Drop for PackageTempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn package_volume_paths(path: &Path) -> Vec<PathBuf> {
    let raw_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("AstralParty-Mods");
    let stem = raw_stem
        .rsplit_once(".part")
        .and_then(|(base, suffix)| {
            (suffix.len() >= 3 && suffix.bytes().all(|b| b.is_ascii_digit())).then_some(base)
        })
        .unwrap_or(raw_stem);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut paths = Vec::new();
    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(suffix) = name
                .strip_prefix(&format!("{}.part", stem))
                .and_then(|tail| tail.strip_suffix(".apmod"))
            {
                if suffix.len() >= 3 && suffix.bytes().all(|b| b.is_ascii_digit()) {
                    paths.push(entry.path());
                }
            }
        }
    }
    paths.sort();
    paths
}

fn package_is_v3(path: &str) -> Result<bool, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("读取加密包失败：{}", e))?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic).map_err(|e| e.to_string())?;
    Ok(&magic == PACKAGE_MAGIC_V3)
}

#[cfg(windows)]
fn package_available_space(path: &Path) -> Result<u64, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut available = 0u64;
    unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut available), None, None) }
        .map_err(|e| format!("检查磁盘空间失败：{}", e))?;
    Ok(available)
}

#[cfg(not(windows))]
fn package_available_space(_path: &Path) -> Result<u64, String> {
    Ok(u64::MAX)
}

#[cfg(windows)]
fn package_file_size_limit(path: &Path) -> Result<Option<u64>, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetVolumeInformationW;
    let raw = path.to_string_lossy();
    let bytes = raw.as_bytes();
    if bytes.len() < 3 || bytes[1] != b':' {
        return Ok(None);
    }
    let root: Vec<u16> = format!("{}\\", &raw[..2])
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut name = [0u16; 32];
    unsafe {
        GetVolumeInformationW(
            PCWSTR(root.as_ptr()),
            None,
            None,
            None,
            None,
            Some(&mut name),
        )
    }
    .map_err(|e| format!("检查文件系统失败：{}", e))?;
    let end = name
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(name.len());
    let filesystem = String::from_utf16_lossy(&name[..end]);
    Ok(if filesystem.eq_ignore_ascii_case("FAT32") {
        Some(u32::MAX as u64)
    } else {
        None
    })
}

#[cfg(not(windows))]
fn package_file_size_limit(_path: &Path) -> Result<Option<u64>, String> {
    Ok(None)
}

fn write_package_v3(
    zip_path: &Path,
    target: &Path,
    password: &str,
    max_volume: u64,
    kdf: (u32, u32, u32),
) -> Result<Vec<String>, String> {
    write_package_v3_cancellable(zip_path, target, password, max_volume, kdf, None)
}
fn write_package_v3_cancellable(
    zip_path: &Path,
    target: &Path,
    password: &str,
    max_volume: u64,
    kdf: (u32, u32, u32),
    cancel: Option<&AtomicBool>,
) -> Result<Vec<String>, String> {
    let frame_bytes = (PACKAGE_CHUNK_BYTES + 32) as u64;
    if max_volume < PACKAGE_CHUNK_BYTES as u64 + PACKAGE_V3_HEADER_LEN as u64 + 32 {
        return Err("分卷大小不足以容纳加密块".into());
    }
    let frames_per_volume = (max_volume - PACKAGE_V3_HEADER_LEN as u64) / frame_bytes;
    let plain_size = fs::metadata(zip_path).map_err(|e| e.to_string())?.len();
    let frame_count = plain_size.div_ceil(PACKAGE_CHUNK_BYTES as u64).max(1);
    let count = frame_count.div_ceil(frames_per_volume);
    if count > u32::MAX as u64 {
        return Err("分卷数量超出格式范围".into());
    }
    let mut set_id = [0u8; 16];
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut set_id);
    rand::thread_rng().fill_bytes(&mut salt);
    let key = package_key(password, &salt, kdf.0, kdf.1, kdf.2)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let mut source = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let parent = target.parent().ok_or("导出目录无效")?;
    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("导出文件名无效")?;
    let mut pending = Vec::new();
    let result: Result<(), String> = (|| {
        for volume_index in 0..count {
            let final_path = parent.join(format!("{}.part{:03}.apmod", stem, volume_index + 1));
            if final_path.exists() {
                return Err(format!("目标分卷已存在：{}", final_path.display()));
            }
            let temp_path = parent.join(format!("{}.part{:03}.apmod.tmp", stem, volume_index + 1));
            let mut out = fs::File::create(&temp_path).map_err(|e| e.to_string())?;
            pending.push((temp_path, final_path));
            out.write_all(PACKAGE_MAGIC_V3).map_err(|e| e.to_string())?;
            out.write_all(&set_id).map_err(|e| e.to_string())?;
            out.write_all(&(volume_index as u32).to_le_bytes())
                .map_err(|e| e.to_string())?;
            out.write_all(&(count as u32).to_le_bytes())
                .map_err(|e| e.to_string())?;
            for value in [kdf.0, kdf.1, kdf.2] {
                out.write_all(&value.to_le_bytes())
                    .map_err(|e| e.to_string())?;
            }
            out.write_all(&salt).map_err(|e| e.to_string())?;
            out.write_all(&frame_count.to_le_bytes())
                .map_err(|e| e.to_string())?;
            for slot in 0..frames_per_volume.min(frame_count - volume_index * frames_per_volume) {
                if cancel.is_some_and(|flag| flag.load(Ordering::Acquire)) {
                    return Err("导出已取消，分卷未提交".into());
                }
                let sequence = volume_index * frames_per_volume + slot;
                let mut plain = vec![
                    0u8;
                    PACKAGE_CHUNK_BYTES.min(
                        (plain_size - sequence * PACKAGE_CHUNK_BYTES as u64) as usize
                    )
                ];
                source.read_exact(&mut plain).map_err(|e| e.to_string())?;
                let len = plain.len();
                plain.truncate(len);
                let mut nonce = [0u8; 12];
                rand::thread_rng().fill_bytes(&mut nonce);
                let mut aad = Vec::from(set_id);
                aad.extend_from_slice(&(count as u32).to_le_bytes());
                for value in [kdf.0, kdf.1, kdf.2] {
                    aad.extend_from_slice(&value.to_le_bytes());
                }
                aad.extend_from_slice(&salt);
                aad.extend_from_slice(&frame_count.to_le_bytes());
                aad.extend_from_slice(&sequence.to_le_bytes());
                let encrypted = cipher
                    .encrypt(
                        Nonce::from_slice(&nonce),
                        Payload {
                            msg: &plain,
                            aad: &aad,
                        },
                    )
                    .map_err(|_| "分卷加密失败".to_string())?;
                out.write_all(&(len as u32).to_le_bytes())
                    .map_err(|e| e.to_string())?;
                out.write_all(&nonce).map_err(|e| e.to_string())?;
                out.write_all(&encrypted).map_err(|e| e.to_string())?;
            }
            out.sync_all().map_err(|e| e.to_string())?;
        }
        for (temp, final_path) in &pending {
            fs::rename(temp, final_path).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        for (temp, final_path) in &pending {
            let _ = fs::remove_file(temp);
            let _ = fs::remove_file(final_path);
        }
        return Err(error);
    }
    Ok(pending
        .into_iter()
        .map(|(_, path)| path.to_string_lossy().into_owned())
        .collect())
}

fn unpack_package_v3(
    path: &str,
    password: &str,
    stage: &Path,
) -> Result<(PackageManifest, Vec<PathBuf>), String> {
    ensure_dir(stage).map_err(|e| e.to_string())?;
    let zip_path = stage.join("payload.zip");
    let result: Result<(PackageManifest, Vec<PathBuf>), String> = (|| {
        let candidates = package_volume_paths(Path::new(path));
        let mut first = [0u8; PACKAGE_V3_HEADER_LEN];
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .read_exact(&mut first)
            .map_err(|_| "分卷头无效".to_string())?;
        let count = u32::from_le_bytes(first[28..32].try_into().unwrap()) as usize;
        let frame_count = u64::from_le_bytes(first[60..68].try_into().unwrap());
        if count == 0 || count > 100_000 || frame_count == 0 {
            return Err("分卷数量无效".into());
        }
        let mut volumes: Vec<Option<fs::File>> = (0..count).map(|_| None).collect();
        for candidate in candidates {
            let mut file = fs::File::open(&candidate).map_err(|e| e.to_string())?;
            let mut header = [0u8; PACKAGE_V3_HEADER_LEN];
            file.read_exact(&mut header).map_err(|e| e.to_string())?;
            if header[..24] != first[..24] {
                continue;
            }
            if header[28..] != first[28..] {
                return Err("分卷集合信息不一致".into());
            }
            let index = u32::from_le_bytes(header[24..28].try_into().unwrap()) as usize;
            if index >= count || volumes[index].is_some() {
                return Err("分卷重复或编号无效".into());
            }
            volumes[index] = Some(file);
        }
        if volumes.iter().any(Option::is_none) {
            return Err("分卷不完整，请保留同目录全部分卷".into());
        }
        let compressed_bytes = volumes
            .iter()
            .filter_map(|file| {
                file.as_ref()
                    .and_then(|f| f.metadata().ok().map(|m| m.len()))
            })
            .sum::<u64>();
        if package_available_space(stage)?
            < compressed_bytes
                .saturating_mul(2)
                .saturating_add(64 * 1024 * 1024)
        {
            return Err("导入暂存磁盘空间不足".into());
        }
        let memory = u32::from_le_bytes(first[32..36].try_into().unwrap());
        let iterations = u32::from_le_bytes(first[36..40].try_into().unwrap());
        let parallelism = u32::from_le_bytes(first[40..44].try_into().unwrap());
        let mut salt = [0u8; 16];
        salt.copy_from_slice(&first[44..60]);
        let key = package_key(password, &salt, memory, iterations, parallelism)?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
        let mut zip_out = fs::File::create(&zip_path).map_err(|e| e.to_string())?;
        let mut sequence = 0u64;
        for mut volume in volumes.into_iter().flatten() {
            loop {
                let mut len_bytes = [0u8; 4];
                let first_byte = volume
                    .read(&mut len_bytes[..1])
                    .map_err(|e| e.to_string())?;
                if first_byte == 0 {
                    break;
                }
                volume
                    .read_exact(&mut len_bytes[1..])
                    .map_err(|_| "分卷被截断".to_string())?;
                let len = u32::from_le_bytes(len_bytes) as usize;
                if len > PACKAGE_CHUNK_BYTES || sequence >= frame_count {
                    return Err("分卷数据长度无效".into());
                }
                let mut nonce = [0u8; 12];
                volume
                    .read_exact(&mut nonce)
                    .map_err(|_| "分卷被截断".to_string())?;
                let mut encrypted = vec![0u8; len + 16];
                volume
                    .read_exact(&mut encrypted)
                    .map_err(|_| "分卷被截断".to_string())?;
                let mut aad = Vec::from(&first[8..24]);
                aad.extend_from_slice(&first[28..68]);
                aad.extend_from_slice(&sequence.to_le_bytes());
                let plain = cipher
                    .decrypt(
                        Nonce::from_slice(&nonce),
                        Payload {
                            msg: &encrypted,
                            aad: &aad,
                        },
                    )
                    .map_err(|_| "密码错误或分卷已损坏".to_string())?;
                zip_out.write_all(&plain).map_err(|e| e.to_string())?;
                sequence += 1;
            }
        }
        if sequence != frame_count {
            return Err("分卷数据不完整".into());
        }
        drop(zip_out);
        let mut zip = zip::ZipArchive::new(fs::File::open(&zip_path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("加密包内容无效：{}", e))?;
        if zip.len() > PACKAGE_MAX_FILES + 1 {
            return Err("加密包文件数超过上限".into());
        }
        let mut manifest_text = String::new();
        zip.by_name("manifest.json")
            .map_err(|_| "加密包缺少 manifest.json".to_string())?
            .take(1_048_577)
            .read_to_string(&mut manifest_text)
            .map_err(|e| e.to_string())?;
        if manifest_text.len() > 1_048_576 {
            return Err("manifest 过大".into());
        }
        let manifest: PackageManifest =
            serde_json::from_str(&manifest_text).map_err(|e| e.to_string())?;
        if manifest.format_version != 3 || manifest.files.len() > PACKAGE_MAX_FILES {
            return Err("不支持的分卷包版本或文件数".into());
        }
        let mut paths = Vec::with_capacity(manifest.files.len());
        for (index, meta) in manifest.files.iter().enumerate() {
            if !is_png_mod_ext(&meta.ext)
                || meta.hash.len() != 64
                || !meta.hash.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("分卷清单中的资源无效".into());
            }
            let mut entry = zip
                .by_name(&format!("payload/{}", meta.hash))
                .map_err(|_| format!("缺少文件 {}", meta.source_name))?;
            let output = stage.join(format!("{:05}.payload", index));
            let mut file = fs::File::create(&output).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
            if sha256_file(&output)?.to_ascii_lowercase() != meta.hash.to_ascii_lowercase() {
                return Err("分卷资源哈希校验失败".into());
            }
            paths.push(output);
        }
        Ok((manifest, paths))
    })();
    let _ = fs::remove_file(&zip_path);
    if result.is_err() {
        let _ = crate::staging::remove(stage);
    }
    result
}

#[derive(Debug)]
pub(crate) struct QuickExportFile {
    rel_path: PathBuf,
    bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageImportArgs {
    path: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageCommitArgs {
    path: String,
    password: String,
    #[serde(default)]
    selected_hashes: Vec<String>,

    /// selfSelect/importOnly/applyAll/fillEmpty. `selfSelect` only imports;
    /// the UI applies the explicitly selected groups after previewing them.
    #[serde(default)]
    apply_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackagePreview {
    app_version: String,
    created_at: u64,
    files: Vec<PackageFileMeta>,
    #[serde(default)]
    has_application_state: bool,
    #[serde(default)]
    application_state_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    application_state: Option<PackageApplicationState>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageQueueBeginResult {
    queue_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageQueueIdArgs {
    queue_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageQueuePreviewImageArgs {
    queue_id: String,
    item_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageQueueAddArgs {
    queue_id: String,
    path: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageQueueFilePreview {
    id: String,
    meta: PackageFileMeta,
    /// New clients use the explicit three-way import classification.  Keep
    /// `conflict` below for older shims that still understand the v0.2.24
    /// boolean preview shape.
    status: String,
    conflict: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageQueuePreview {
    files: Vec<PackageQueueFilePreview>,
    conflict_count: usize,
    duplicate_count: usize,
    new_count: usize,
    #[serde(default)]
    has_application_state: bool,
    #[serde(default)]
    application_state_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    application_state: Option<PackageApplicationState>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageQueueCommitItem {
    id: String,
    apply_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageQueueCommitArgs {
    queue_id: String,
    items: Vec<PackageQueueCommitItem>,
    #[serde(default)]
    use_application_state: bool,
}

#[derive(Debug)]
pub(crate) struct QueuedPackageFile {
    id: String,
    meta: PackageFileMeta,
    bytes: Vec<u8>,
    staged_path: Option<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct PackageImportQueue {
    id: String,
    files: Vec<QueuedPackageFile>,
    application_states: Vec<PackageApplicationState>,
}

pub(crate) fn package_key(
    password: &str,
    salt: &[u8; 16],
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
) -> Result<[u8; 32], String> {
    if password.is_empty() {
        return Err("密码不能为空".into());
    }
    if !(8_192..=PACKAGE_KDF_MAX_MEMORY_KIB).contains(&memory_kib)
        || !(1..=PACKAGE_KDF_MAX_ITERATIONS).contains(&iterations)
        || !(1..=PACKAGE_KDF_MAX_PARALLELISM).contains(&parallelism)
    {
        return Err("加密包 KDF 参数超出安全范围".into());
    }
    let params = Params::new(memory_kib, iterations, parallelism, Some(32))
        .map_err(|_| "加密包中的 Argon2id 参数无效".to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("密码派生失败：{}", e))?;
    Ok(key)
}

pub(crate) fn package_kdf_profile(profile: Option<&str>) -> Result<(u32, u32, u32), String> {
    match profile.unwrap_or("standard") {
        "compat" => Ok((PACKAGE_V1_KDF_MEMORY_KIB, PACKAGE_V1_KDF_ITERATIONS, 1)),
        "standard" => Ok((
            PACKAGE_KDF_MEMORY_KIB,
            PACKAGE_KDF_ITERATIONS,
            PACKAGE_KDF_PARALLELISM,
        )),
        "high" => Ok((PACKAGE_KDF_MAX_MEMORY_KIB, 4, 1)),
        _ => Err("未知的密码保护档位".into()),
    }
}

pub(crate) fn encrypt_package_with_params(
    payload: &[u8],
    password: &str,
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
) -> Result<Vec<u8>, String> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);
    let key = package_key(password, &salt, memory_kib, iterations, parallelism)?;
    let mut header = Vec::with_capacity(PACKAGE_V2_HEADER_LEN);
    header.extend_from_slice(PACKAGE_MAGIC_V2);
    header.push(PACKAGE_KDF_ID_ARGON2ID);

    header.extend_from_slice(&memory_kib.to_le_bytes());
    header.extend_from_slice(&iterations.to_le_bytes());
    header.extend_from_slice(&parallelism.to_le_bytes());
    header.extend_from_slice(&salt);
    header.extend_from_slice(&nonce);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: payload,
                aad: &header,
            },
        )
        .map_err(|_| "加密失败".to_string())?;
    header.extend_from_slice(&encrypted);
    Ok(header)
}

pub(crate) fn encrypt_package(payload: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let (memory_kib, iterations, parallelism) = package_kdf_profile(Some("standard"))?;
    encrypt_package_with_params(payload, password, memory_kib, iterations, parallelism)
}

pub(crate) fn decrypt_package(bytes: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if bytes.len() < PACKAGE_V1_HEADER_LEN {
        return Err("不是受支持的 .apmod 加密包".into());
    }
    let (header_len, memory_kib, iterations, parallelism, salt_start, nonce_start) = if &bytes[..8]
        == PACKAGE_MAGIC_V2
    {
        if bytes.len() < PACKAGE_V2_HEADER_LEN || bytes[8] != PACKAGE_KDF_ID_ARGON2ID {
            return Err("不支持的 .apmod KDF 头".into());
        }
        let memory_kib = u32::from_le_bytes(bytes[9..13].try_into().map_err(|_| "加密包头无效")?);
        let iterations = u32::from_le_bytes(bytes[13..17].try_into().map_err(|_| "加密包头无效")?);
        let parallelism = u32::from_le_bytes(bytes[17..21].try_into().map_err(|_| "加密包头无效")?);
        (
            PACKAGE_V2_HEADER_LEN,
            memory_kib,
            iterations,
            parallelism,
            21usize,
            37usize,
        )
    } else if &bytes[..8] == PACKAGE_MAGIC_V1 {
        (
            PACKAGE_V1_HEADER_LEN,
            PACKAGE_V1_KDF_MEMORY_KIB,
            PACKAGE_V1_KDF_ITERATIONS,
            1,
            8usize,
            24usize,
        )
    } else {
        return Err("不是受支持的 .apmod 加密包".into());
    };
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&bytes[salt_start..salt_start + 16]);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&bytes[nonce_start..nonce_start + 12]);
    let key = package_key(password, &salt, memory_kib, iterations, parallelism)?;
    Aes256Gcm::new_from_slice(&key)
        .map_err(|e| e.to_string())?
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &bytes[header_len..],
                aad: &bytes[..header_len],
            },
        )
        .map_err(|_| "密码错误或加密包已损坏".to_string())
}

pub(crate) fn package_contents(
    path: &str,
    password: &str,
) -> Result<(PackageManifest, Vec<Vec<u8>>), String> {
    if fs::metadata(path)
        .map_err(|e| format!("读取加密包失败：{}", e))?
        .len()
        > PACKAGE_MAX_BYTES as u64
    {
        return Err("旧版单文件加密包超过 1 GiB 上限".into());
    }
    let encrypted = fs::read(path).map_err(|e| format!("读取加密包失败：{}", e))?;
    if encrypted.len() > PACKAGE_MAX_BYTES {
        return Err("旧版单文件加密包超过 1 GiB 上限".into());
    }
    let plain = decrypt_package(&encrypted, password)?;
    if plain.len() > PACKAGE_MAX_BYTES {
        return Err("旧版单文件解密内容超过 1 GiB 上限".into());
    }
    let mut zip =
        zip::ZipArchive::new(Cursor::new(plain)).map_err(|e| format!("加密包内容无效：{}", e))?;
    if zip.len() > PACKAGE_MAX_FILES + 1 {
        return Err("加密包文件数超过上限".into());
    }
    let mut manifest_text = String::new();
    zip.by_name("manifest.json")
        .map_err(|_| "加密包缺少 manifest.json".to_string())?
        .take(1_048_576)
        .read_to_string(&mut manifest_text)
        .map_err(|e| e.to_string())?;
    let manifest: PackageManifest =
        serde_json::from_str(&manifest_text).map_err(|e| format!("manifest 无效：{}", e))?;
    if !matches!(manifest.format_version, 1 | 2) || manifest.files.len() > PACKAGE_MAX_FILES {
        return Err("不支持的加密包版本或文件数".into());
    }
    let mut data = Vec::with_capacity(manifest.files.len());
    let mut total = 0usize;
    for file in &manifest.files {
        if !is_png_mod_ext(&file.ext) {
            return Err("加密包仅支持 PNG Mod".into());
        }
        if file.hash.len() != 64 || !file.hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("manifest 中存在无效哈希".into());
        }
        let entry = zip
            .by_name(&format!("payload/{}", file.hash))
            .map_err(|_| format!("缺少文件 {}", file.source_name))?;
        let mut bytes = Vec::new();
        entry
            .take(PACKAGE_MAX_BYTES as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        total = total.checked_add(bytes.len()).ok_or("解压大小溢出")?;
        if total > PACKAGE_MAX_BYTES || sha256_bytes(&bytes) != file.hash.to_ascii_lowercase() {
            return Err("文件校验失败或超出解压上限".into());
        }
        data.push(bytes);
    }
    Ok((manifest, data))
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn credential_default_password_get() -> Result<Option<String>, String> {
    let entry = keyring::Entry::new("AstralPartyModManager", "default-export-password")
        .map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("读取系统凭据失败：{}", e)),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn credential_default_password_set(password: String) -> Result<(), String> {
    if password.is_empty() {
        return Err("默认密码不能为空".into());
    }
    keyring::Entry::new("AstralPartyModManager", "default-export-password")
        .map_err(|e| e.to_string())?
        .set_password(&password)
        .map_err(|e| format!("保存系统凭据失败：{}", e))
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn credential_default_password_clear() -> Result<(), String> {
    let entry = keyring::Entry::new("AstralPartyModManager", "default-export-password")
        .map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("清除系统凭据失败：{}", e)),
    }
}

/// Flat, stable export rows for the front-end selector.  Names are display
/// data only; all operations continue to use category/id/section/canonical/hash.
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn package_export_candidates() -> Vec<PackageFileMeta> {
    let lib = load_library();
    let mut out = Vec::new();
    // The persisted library keeps non-monster resources (including cards)
    // in the heroes map for compatibility.  Derive the real category from
    // the section definition instead of labelling the whole map as heroes.
    for (map_category, map) in [("heroes", &lib.heroes), ("monsters", &lib.monsters)] {
        for (entity_id, hero) in map {
            for (section_key, section) in &hero.sections {
                let section_def = section_defs().into_iter().find(|s| s.key == *section_key);
                let category = logical_category_for_section(section_key, map_category);
                let section_name = section_def
                    .map(|s| s.label)
                    .unwrap_or_else(|| section_key.clone());
                for (canonical, resource) in &section.resources {
                    let entity_name = if category == "monsters" {
                        monster_display_name(&lib, entity_id)
                    } else if category == "cards" {
                        card_display_name(section_key, canonical)
                            .unwrap_or_else(|| entity_id.clone())
                    } else {
                        display_name_of(&lib, entity_id)
                    };
                    let resource_name = if category == "cards" {
                        card_display_name(section_key, canonical)
                            .unwrap_or_else(|| canonical.clone())
                    } else {
                        variant_label(canonical)
                    };
                    for file in &resource.files {
                        if !is_png_mod_ext(&file.ext) {
                            continue;
                        }
                        let stored_path = vault_path(&vault_dir(), file);
                        out.push(PackageFileMeta {
                            category: category.clone(),
                            entity_id: entity_id.clone(),
                            section_key: section_key.clone(),
                            canonical: canonical.clone(),
                            hash: file.hash.clone(),
                            ext: file.ext.clone(),
                            source_name: file.source_name.clone(),
                            was_enabled: resource.active_hash.as_deref()
                                == Some(file.hash.as_str())
                                && !resource.disabled,
                            storage_name: stored_path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .map(str::to_string),
                            entity_name: entity_name.clone(),
                            section_name: section_name.clone(),
                            resource_name: resource_name.clone(),
                            category_name: logical_category_label(&category).0,
                            category_order: logical_category_label(&category).1,
                            preview_url: Some(stored_path.to_string_lossy().to_string()),
                            provenance: Some(file.info.provenance.clone()),
                            manual_classification: file.classification.manual,
                        });
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| {
        (
            a.category_order,
            &a.entity_id,
            &a.section_key,
            &a.canonical,
            &a.source_name,
        )
            .cmp(&(
                b.category_order,
                &b.entity_id,
                &b.section_key,
                &b.canonical,
                &b.source_name,
            ))
    });
    out
}

pub(crate) fn package_export_blocking(
    args: PackageExportArgs,
    app: tauri::AppHandle,
    mutation: Arc<Mutex<()>>,
) -> Result<PackageExportResult, String> {
    if args.items.is_empty() {
        return Err("请至少选择一个 Mod".into());
    }
    if args.max_volume_bytes < PACKAGE_MIN_VOLUME_BYTES {
        return Err("分卷大小不能小于 16 MiB".into());
    }
    let (memory_kib, iterations, parallelism) = package_kdf_profile(args.kdf_profile.as_deref())?;
    let _ = app.emit(
        "package-progress",
        serde_json::json!({"phase":"export","percent":5,"message":"正在校验并打包 Mod"}),
    );
    let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let lib = load_library();
    let mut manifest = PackageManifest {
        format_version: 3,
        app_version: env!("CARGO_PKG_VERSION").into(),
        created_at: unix_now(),
        files: Vec::new(),
        application_state: None,
    };
    let mut payloads: Vec<(String, PathBuf)> = Vec::new();
    let mut seen_records = HashSet::new();
    let mut seen_payloads = HashSet::new();
    let mut duplicate_records = 0usize;
    let mut selected_state = Vec::new();
    for item in args.items {
        crate::tasks::checkpoint(&app)?;
        let record_key = (
            item.category.clone(),
            item.entity_id.clone(),
            item.section_key.clone(),
            item.canonical.clone(),
            item.hash.to_ascii_lowercase(),
        );
        if !seen_records.insert(record_key) {
            duplicate_records += 1;
            continue;
        }
        let resource = lib_map(&lib, &item.category)
            .get(&item.entity_id)
            .and_then(|h| h.sections.get(&item.section_key))
            .and_then(|s| s.resources.get(&item.canonical))
            .ok_or("所选资源不存在")?;
        let entry = resource
            .files
            .iter()
            .find(|f| f.hash == item.hash)
            .ok_or("所选 Mod 不存在")?;
        if !is_png_mod_ext(&entry.ext) {
            return Err("仅支持导出 PNG Mod".into());
        }
        let source = vault_path(&vault_dir(), entry);
        if sha256_file(&source)?.to_ascii_lowercase() != entry.hash.to_ascii_lowercase() {
            return Err(format!("{} 的哈希校验失败", entry.source_name));
        }
        let is_enabled =
            resource.active_hash.as_deref() == Some(entry.hash.as_str()) && !resource.disabled;
        let meta = PackageFileMeta {
            category: item.category.clone(),
            entity_id: item.entity_id.clone(),
            section_key: item.section_key.clone(),
            canonical: item.canonical.clone(),
            hash: entry.hash.clone(),
            ext: entry.ext.clone(),
            source_name: entry.source_name.clone(),
            storage_name: source
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string),
            was_enabled: is_enabled,
            entity_name: String::new(),
            section_name: String::new(),
            resource_name: String::new(),
            category_name: String::new(),
            category_order: 4,
            preview_url: None,
            provenance: Some(entry.info.provenance.clone()),
            manual_classification: entry.classification.manual,
        };
        manifest.files.push(localize_package_meta(&lib, meta));
        if args.include_application_state && is_enabled {
            selected_state.push(PackageSelection {
                category: item.category.clone(),
                entity_id: item.entity_id.clone(),
                section_key: item.section_key.clone(),
                canonical: item.canonical.clone(),
                hash: entry.hash.clone(),
            });
        }
        if seen_payloads.insert(entry.hash.to_ascii_lowercase()) {
            payloads.push((entry.hash.clone(), source));
        }
    }
    if args.include_application_state && !selected_state.is_empty() {
        manifest.application_state = Some(PackageApplicationState {
            enabled_resources: selected_state,
        });
    }
    let mut target = PathBuf::from(args.destination);
    if target
        .extension()
        .and_then(|x| x.to_str())
        .map(|x| !x.eq_ignore_ascii_case("apmod"))
        .unwrap_or(true)
    {
        target.set_extension("apmod");
    }
    let parent = target.parent().ok_or("导出路径无效")?;
    ensure_dir(parent).map_err(|e| e.to_string())?;
    if package_file_size_limit(parent)?.is_some_and(|limit| args.max_volume_bytes > limit) {
        return Err("所选分卷大小超过目标文件系统的单文件上限，请调小分卷".into());
    }
    let source_bytes = payloads.iter().try_fold(0u64, |total, (_, path)| {
        let len = fs::metadata(path).map_err(|e| e.to_string())?.len();
        total
            .checked_add(len)
            .ok_or_else(|| "导出总大小溢出".to_string())
    })?;
    let required = source_bytes
        .saturating_mul(2)
        .saturating_add(64 * 1024 * 1024);
    if package_available_space(parent)? < required {
        return Err("目标磁盘空间不足，导出需要同时暂存压缩数据和分卷".into());
    }
    let zip_temp = parent.join(format!(".astral-package-{}.zip.tmp", new_queue_id()));
    let _zip_cleanup = PackageTempFile(zip_temp.clone());
    let mut zip = zip::ZipWriter::new(fs::File::create(&zip_temp).map_err(|e| e.to_string())?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&serde_json::to_vec(&manifest).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let unique_payload_count = payloads.len();
    for (hash, source) in payloads {
        crate::tasks::checkpoint(&app)?;
        zip.start_file(format!("payload/{}", hash), options)
            .map_err(|e| e.to_string())?;
        std::io::copy(
            &mut fs::File::open(&source).map_err(|e| e.to_string())?,
            &mut zip,
        )
        .map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    let _ = app.emit(
        "package-progress",
        serde_json::json!({"phase":"export","percent":68,"message":"正在加密 .apmod"}),
    );
    let cancel = crate::tasks::cancellation(&app);
    let result = write_package_v3_cancellable(
        &zip_temp,
        &target,
        &args.password,
        args.max_volume_bytes,
        (memory_kib, iterations, parallelism),
        cancel.as_deref(),
    );
    let volumes = result?;
    let bytes = volumes
        .iter()
        .filter_map(|path| fs::metadata(path).ok().map(|meta| meta.len()))
        .sum::<u64>();
    write_log(
        "info",
        "package.export",
        &format!(
            "导出 {} 条资源记录、{} 个唯一 payload，跳过 {} 条完全重复记录",
            manifest.files.len(),
            unique_payload_count,
            duplicate_records
        ),
        None,
    );
    let _ = app.emit(
        "package-progress",
        serde_json::json!({"phase":"complete","percent":100,"message":"加密包导出完成"}),
    );
    Ok(PackageExportResult {
        path: volumes
            .first()
            .cloned()
            .unwrap_or_else(|| target.to_string_lossy().to_string()),
        files: manifest.files.len(),
        bytes: bytes.min(usize::MAX as u64) as usize,
        volumes,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn package_export(
    args: PackageExportArgs,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<PackageExportResult, String> {
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || package_export_blocking(args, app, mutation))
        .await
        .map_err(|e| format!("导出后台任务失败：{}", e))?
}

pub(crate) fn quick_export_relative_path(
    category: &str,
    entity_id: &str,
    section_key: &str,
    source_name: &str,
    hash: &str,
) -> PathBuf {
    let clean = sanitize_name(source_name);
    let stem = clean
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(clean.as_str());
    let short = hash.get(..8).unwrap_or(hash);
    PathBuf::from(sanitize_name(category))
        .join(sanitize_name(entity_id))
        .join(sanitize_name(section_key))
        .join(format!("{}_{}.png", sanitize_name(stem), short))
}

/// Resolve an indexed vault entry only when its final filesystem location is
/// still inside the vault.  This keeps quick export constrained even if an
/// old or manually edited library index contains a traversal path or link.
pub(crate) fn controlled_vault_file(vault: &Path, entry: &FileEntry) -> Result<PathBuf, String> {
    let source = vault_path(vault, entry);
    if !source.is_file() {
        return Err(format!(
            "读取 Mod 失败（{}）：文件不存在",
            entry.source_name
        ));
    }
    let vault_real = fs::canonicalize(vault).map_err(|e| format!("无法校验 Mod 库目录：{}", e))?;
    let source_real = fs::canonicalize(&source).map_err(|e| format!("无法校验 Mod 文件：{}", e))?;
    if !source_real.starts_with(&vault_real) {
        return Err("所选 Mod 不在受控库目录中".into());
    }
    Ok(source_real)
}

pub(crate) fn unique_quick_export_zip_name(rel: &Path, used: &mut HashSet<String>) -> String {
    let first = rel.to_string_lossy().replace('\\', "/");
    if used.insert(first.clone()) {
        return first;
    }
    let parent = rel.parent().unwrap_or_else(|| Path::new(""));
    let stem = rel
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("mod");
    let extension = rel
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    for index in 2..10_000usize {
        let candidate = parent
            .join(format!("{}_{}.{}", stem, index, extension))
            .to_string_lossy()
            .replace('\\', "/");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    first
}

pub(crate) fn selected_quick_export_files(
    lib: &Library,
    items: &[PackageSelection],
) -> Result<Vec<QuickExportFile>, String> {
    if items.is_empty() {
        return Err("请至少选择一个 Mod".into());
    }
    let vault = vault_dir();
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for item in items {
        let identity = format!(
            "{}/{}/{}/{}/{}",
            item.category, item.entity_id, item.section_key, item.canonical, item.hash
        );
        if !seen.insert(identity) {
            continue;
        }
        let resource = lib_map(lib, &item.category)
            .get(&item.entity_id)
            .and_then(|hero| hero.sections.get(&item.section_key))
            .and_then(|section| section.resources.get(&item.canonical))
            .ok_or("所选资源不存在")?;
        let entry = resource
            .files
            .iter()
            .find(|entry| entry.hash == item.hash)
            .ok_or("所选 Mod 不存在")?;
        if !is_png_mod_ext(&entry.ext) {
            return Err("快速导出仅支持 PNG Mod".into());
        }
        let source = controlled_vault_file(&vault, entry)?;
        let bytes = fs::read(&source)
            .map_err(|e| format!("读取 Mod 失败（{}）：{}", entry.source_name, e))?;
        if sha256_bytes(&bytes) != entry.hash.to_ascii_lowercase() {
            return Err(format!("{} 的哈希校验失败", entry.source_name));
        }
        files.push(QuickExportFile {
            rel_path: quick_export_relative_path(
                &item.category,
                &item.entity_id,
                &item.section_key,
                &entry.source_name,
                &entry.hash,
            ),
            bytes,
        });
    }
    if files.is_empty() {
        return Err("未找到可导出的 Mod".into());
    }
    Ok(files)
}

pub(crate) fn unique_quick_export_path(root: &Path, rel: &Path) -> PathBuf {
    let candidate = root.join(rel);
    if !candidate.exists() {
        return candidate;
    }
    let parent = candidate.parent().unwrap_or(root);
    let stem = candidate
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("mod");
    let extension = candidate
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    for index in 2..10_000usize {
        let next = parent.join(format!("{}_{}.{}", stem, index, extension));
        if !next.exists() {
            return next;
        }
    }
    candidate
}

pub(crate) fn quick_export_folder(
    destination: &Path,
    files: &[QuickExportFile],
    stage: &Path,
) -> Result<(), String> {
    ensure_dir(stage).map_err(|e| e.to_string())?;
    for file in files {
        let staged = stage.join(&file.rel_path);
        if let Some(parent) = staged.parent() {
            ensure_dir(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&staged, &file.bytes).map_err(|e| format!("写入快速导出暂存区失败：{}", e))?;
    }
    ensure_dir(destination).map_err(|e| e.to_string())?;
    let mut created = Vec::new();
    let result: Result<(), String> = (|| {
        for file in files {
            let staged = stage.join(&file.rel_path);
            let target = unique_quick_export_path(destination, &file.rel_path);
            let parent = target.parent().ok_or("快速导出目标路径无效")?;
            ensure_dir(parent).map_err(|e| e.to_string())?;

            let name = target
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or("快速导出目标文件名无效")?;
            let temporary = parent.join(format!(".{}.part", name));
            fs::copy(&staged, &temporary).map_err(|e| format!("提交快速导出文件失败：{}", e))?;
            if let Err(error) = fs::rename(&temporary, &target) {
                let _ = fs::remove_file(&temporary);
                return Err(format!("提交快速导出文件失败：{}", error));
            }
            created.push(target);
        }
        Ok(())
    })();
    if result.is_err() {
        for file in created {
            let _ = fs::remove_file(file);
        }
    }
    result
}

pub(crate) fn quick_export_blocking(
    args: QuickExportArgs,
    app: tauri::AppHandle,
    mutation: Arc<Mutex<()>>,
) -> Result<PackageExportResult, String> {
    let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let stage = crate::staging::root().join(format!("quick-export-{}", new_queue_id()));
    let result: Result<PackageExportResult, String> = (|| {
        let files = selected_quick_export_files(&load_library(), &args.items)?;
        crate::tasks::checkpoint(&app)?;
        let bytes = files.iter().map(|file| file.bytes.len()).sum();
        let _ = app.emit(
            "package-progress",
            serde_json::json!({"phase":"export","percent":8,"message":"正在校验快速导出内容"}),
        );
        let mut destination = PathBuf::from(&args.destination);
        match args.mode.as_str() {
            "file" => {
                if files.len() != 1 {
                    return Err("多个 Mod 请导出到文件夹或压缩包".into());
                }
                if destination
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|value| !value.eq_ignore_ascii_case("png"))
                    .unwrap_or(true)
                {
                    destination.set_extension("png");
                }
                let parent = destination.parent().ok_or("快速导出路径无效")?;
                ensure_dir(parent).map_err(|e| e.to_string())?;
                let temporary = destination.with_extension("png.part");
                crate::tasks::commit_phase(&app)?;
                fs::write(&temporary, &files[0].bytes)
                    .map_err(|e| format!("写入快速导出临时文件失败：{}", e))?;
                if let Err(error) = fs::rename(&temporary, &destination) {
                    let _ = fs::remove_file(&temporary);
                    return Err(format!("保存 PNG 失败：{}", error));
                }
            }
            "folder" => {
                crate::tasks::commit_phase(&app)?;
                quick_export_folder(&destination, &files, &stage)?;
            }
            "zip" => {
                if destination
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|value| !value.eq_ignore_ascii_case("zip"))
                    .unwrap_or(true)
                {
                    destination.set_extension("zip");
                }
                let parent = destination.parent().ok_or("快速导出路径无效")?;
                ensure_dir(parent).map_err(|e| e.to_string())?;
                let temporary = destination.with_extension("zip.part");
                let cursor = Cursor::new(Vec::new());
                let mut archive = zip::ZipWriter::new(cursor);
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                let mut names = HashSet::new();
                for file in &files {
                    crate::tasks::checkpoint(&app)?;
                    let name = unique_quick_export_zip_name(&file.rel_path, &mut names);
                    archive
                        .start_file(name, options)
                        .map_err(|e| e.to_string())?;
                    archive.write_all(&file.bytes).map_err(|e| e.to_string())?;
                }
                let data = archive.finish().map_err(|e| e.to_string())?.into_inner();
                crate::tasks::commit_phase(&app)?;
                fs::write(&temporary, data)
                    .map_err(|e| format!("写入快速导出临时压缩包失败：{}", e))?;
                if let Err(error) = fs::rename(&temporary, &destination) {
                    let _ = fs::remove_file(&temporary);
                    return Err(format!("保存压缩包失败：{}", error));
                }
            }
            _ => return Err("不支持的快速导出方式".into()),
        }
        let _ = app.emit(
            "package-progress",
            serde_json::json!({"phase":"complete","percent":100,"message":"快速导出完成"}),
        );
        Ok(PackageExportResult {
            path: destination.to_string_lossy().to_string(),
            files: files.len(),
            bytes,
            volumes: Vec::new(),
        })
    })();
    let _ = crate::staging::remove(&stage);
    match &result {
        Ok(value) => write_log(
            "info",
            "quick.export",
            &format!("导出 {} 个 Mod", value.files),
            None,
        ),
        Err(error) => write_log(
            "error",
            "quick.export",
            error,
            Some(&format!("{:?}", std::backtrace::Backtrace::force_capture())),
        ),
    }
    result
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn quick_export(
    args: QuickExportArgs,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<PackageExportResult, String> {
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || quick_export_blocking(args, app, mutation))
        .await
        .map_err(|e| format!("快速导出后台任务失败：{}", e))?
}

pub(crate) fn package_import_inspect_blocking(
    args: PackageImportArgs,
) -> Result<PackagePreview, String> {
    let manifest = if package_is_v3(&args.path)? {
        let stage = crate::staging::root().join(format!("inspect-{}", new_queue_id()));
        let result = unpack_package_v3(&args.path, &args.password, &stage);
        let _ = crate::staging::remove(&stage);
        result?.0
    } else {
        package_contents(&args.path, &args.password)?.0
    };
    let application_state_count = manifest
        .application_state
        .as_ref()
        .map(|s| s.enabled_resources.len())
        .unwrap_or(0);
    Ok(PackagePreview {
        app_version: manifest.app_version,
        created_at: manifest.created_at,
        files: manifest.files,
        has_application_state: application_state_count > 0,
        application_state_count,
        application_state: manifest.application_state,
    })
}

/// Legacy single-package inspection is kept for compatibility, but the
/// decrypt/ZIP/hash work runs off the Tauri main thread just like the queue
/// API.  This prevents older clients from briefly freezing the window.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn package_import_inspect(
    args: PackageImportArgs,
) -> Result<PackagePreview, String> {
    tauri::async_runtime::spawn_blocking(move || package_import_inspect_blocking(args))
        .await
        .map_err(|e| format!("导入包后台任务失败：{}", e))?
}

pub(crate) fn new_queue_id() -> String {
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn package_storage_name(meta: &PackageFileMeta) -> String {
    let raw = meta.storage_name.as_deref().unwrap_or(&meta.source_name);
    let name = Path::new(raw)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(raw);
    let clean = sanitize_name(name);
    if clean
        .to_ascii_lowercase()
        .ends_with(&meta.ext.to_ascii_lowercase())
    {
        clean
    } else {
        format!("{}{}", clean, meta.ext)
    }
}

fn reserve_package_vault_path(
    vault: &Path,
    meta: &PackageFileMeta,
    reserved: &mut HashSet<PathBuf>,
) -> Result<(PathBuf, String), String> {
    let preferred = package_storage_name(meta);
    let (dest, rel) = place_in_vault(
        vault,
        &meta.category,
        &meta.entity_id,
        &meta.section_key,
        &preferred,
        &meta.hash,
        &meta.ext,
    )?;
    if !dest.exists() && !reserved.contains(&dest) {
        reserved.insert(dest.clone());
        return Ok((dest, rel));
    }

    let stem = preferred
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(&preferred);
    let hash = meta.hash.to_ascii_lowercase();
    for collision in 0usize..10_000 {
        let suffix = if collision == 0 {
            hash[..8].to_string()
        } else {
            format!("{}_{}", &hash[..8], collision + 1)
        };
        let candidate = format!("{}_{}{}", stem, suffix, meta.ext);
        let (dest, rel) = place_in_vault(
            vault,
            &meta.category,
            &meta.entity_id,
            &meta.section_key,
            &candidate,
            &meta.hash,
            &meta.ext,
        )?;
        if !dest.exists() && reserved.insert(dest.clone()) {
            return Ok((dest, rel));
        }
    }
    Err("无法为导入文件分配唯一存储路径".into())
}

#[tauri::command(rename_all = "camelCase")]

pub(crate) fn package_import_queue_begin(
    state: tauri::State<AppState>,
) -> Result<PackageQueueBeginResult, String> {
    let id = new_queue_id();
    let mut slot = state.import_queue.lock().map_err(|_| "导入队列锁已损坏")?;
    if slot.is_some() {
        return Err("已有导入队列，请先完成、重试或取消该队列".into());
    }
    *slot = Some(PackageImportQueue {
        id: id.clone(),
        files: Vec::new(),
        application_states: Vec::new(),
    });
    Ok(PackageQueueBeginResult { queue_id: id })
}

pub(crate) fn package_import_queue_add_blocking(
    args: PackageQueueAddArgs,
    queue_state: Arc<Mutex<Option<PackageImportQueue>>>,
) -> Result<PackagePreview, String> {
    // Decrypt before acquiring queue state. Passwords exist only in this
    // stack frame; the queue retains verified payload bytes, never passwords.
    let is_v3 = package_is_v3(&args.path)?;
    let stage = crate::staging::root()
        .join(&args.queue_id)
        .join(new_queue_id());
    let (manifest, data, paths) = if is_v3 {
        let (manifest, paths) = unpack_package_v3(&args.path, &args.password, &stage)?;
        (manifest, Vec::new(), paths)
    } else {
        let (manifest, data) = package_contents(&args.path, &args.password)?;
        (manifest, data, Vec::new())
    };
    let application_state_count = manifest
        .application_state
        .as_ref()
        .map(|s| s.enabled_resources.len())
        .unwrap_or(0);
    let preview = PackagePreview {
        app_version: manifest.app_version.clone(),
        created_at: manifest.created_at,
        files: manifest.files.clone(),
        has_application_state: application_state_count > 0,
        application_state_count,
        application_state: manifest.application_state.clone(),
    };
    let mut slot = queue_state.lock().map_err(|_| "导入队列锁已损坏")?;
    let Some(queue) = slot.as_mut().filter(|queue| queue.id == args.queue_id) else {
        let _ = crate::staging::remove(&stage);
        return Err("导入队列不存在或已过期".into());
    };
    if let Some(application_state) = manifest.application_state.clone() {
        queue.application_states.push(application_state);
    }
    for (index, meta) in manifest.files.into_iter().enumerate() {
        let id = format!("{}:{}", queue.id, queue.files.len());
        queue.files.push(QueuedPackageFile {
            id,
            meta,
            bytes: if is_v3 {
                Vec::new()
            } else {
                data[index].clone()
            },
            staged_path: if is_v3 {
                Some(paths[index].clone())
            } else {
                None
            },
        });
    }
    Ok(preview)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn package_import_queue_add(
    args: PackageQueueAddArgs,
    state: tauri::State<'_, AppState>,
) -> Result<PackagePreview, String> {
    let queue_state = Arc::clone(&state.import_queue);
    tauri::async_runtime::spawn_blocking(move || {
        package_import_queue_add_blocking(args, queue_state)
    })
    .await
    .map_err(|e| format!("导入解密后台任务失败：{}", e))?
}

pub(crate) fn package_import_queue_preview_blocking(
    args: PackageQueueIdArgs,
    queue_state: Arc<Mutex<Option<PackageImportQueue>>>,
) -> Result<PackageQueuePreview, String> {
    let slot = queue_state.lock().map_err(|_| "导入队列锁已损坏")?;
    let queue = slot
        .as_ref()
        .filter(|queue| queue.id == args.queue_id)
        .ok_or("导入队列不存在或已过期")?;
    let lib = load_library();
    let mut conflict_count = 0usize;
    let mut duplicate_count = 0usize;
    let mut new_count = 0usize;
    let files = queue
        .files
        .iter()
        .map(|file| {
            let meta = localize_package_meta(&lib, normalized_package_meta(&file.meta));
            let resource = lib_map(&lib, &meta.category)
                .get(&meta.entity_id)
                .and_then(|hero| hero.sections.get(&meta.section_key))
                .and_then(|section| section.resources.get(&meta.canonical));
            let duplicate = resource
                .map(|item| {
                    item.files
                        .iter()
                        .any(|entry| entry.hash.eq_ignore_ascii_case(&meta.hash))
                })
                .unwrap_or(false);
            let conflict = !duplicate
                && resource
                    .map(|item| item.active_hash.is_some() && !item.disabled)
                    .unwrap_or(false);
            let status = if duplicate {
                duplicate_count += 1;
                "duplicate"
            } else if conflict {
                conflict_count += 1;
                "conflict"
            } else {
                new_count += 1;
                "new"
            };
            PackageQueueFilePreview {
                id: file.id.clone(),
                meta,
                status: status.into(),
                conflict,
            }
        })
        .collect();
    let application_state_count = queue
        .application_states
        .iter()
        .map(|s| s.enabled_resources.len())
        .sum();
    let application_state = queue
        .application_states
        .iter()
        .flat_map(|state| state.enabled_resources.iter().cloned())
        .collect::<Vec<_>>();
    Ok(PackageQueuePreview {
        files,
        conflict_count,
        duplicate_count,
        new_count,
        has_application_state: application_state_count > 0,
        application_state_count,
        application_state: if application_state.is_empty() {
            None
        } else {
            Some(PackageApplicationState {
                enabled_resources: application_state,
            })
        },
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn package_import_queue_preview(
    args: PackageQueueIdArgs,
    state: tauri::State<'_, AppState>,
) -> Result<PackageQueuePreview, String> {
    let queue_state = Arc::clone(&state.import_queue);
    tauri::async_runtime::spawn_blocking(move || {
        package_import_queue_preview_blocking(args, queue_state)
    })
    .await
    .map_err(|e| format!("导入预览后台任务失败：{}", e))?
}

/// Return an on-demand PNG preview for one already-decrypted queue item. The
/// queue is the authority, so no arbitrary filesystem path can be requested.
pub(crate) fn package_import_queue_preview_image_blocking(
    args: PackageQueuePreviewImageArgs,
    queue_state: Arc<Mutex<Option<PackageImportQueue>>>,
) -> Result<String, String> {
    const MAX_PREVIEW_BYTES: usize = 8 * 1024 * 1024;
    let slot = queue_state.lock().map_err(|_| "导入队列锁已损坏")?;
    let queue = slot
        .as_ref()
        .filter(|queue| queue.id == args.queue_id)
        .ok_or("导入队列不存在或已过期")?;
    let file = queue
        .files
        .iter()
        .find(|file| file.id == args.item_id)
        .ok_or("导入项不存在或已过期")?;
    if !file.meta.ext.eq_ignore_ascii_case(".png") {
        return Err("仅支持 PNG 预览".into());
    }
    let bytes = if let Some(path) = &file.staged_path {
        if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_PREVIEW_BYTES as u64 {
            return Err("文件过大，无法临时预览".into());
        }
        fs::read(path).map_err(|e| e.to_string())?
    } else {
        file.bytes.clone()
    };
    if bytes.len() > MAX_PREVIEW_BYTES {
        return Err("文件过大，无法临时预览".into());
    }
    Ok(format!("data:image/png;base64,{}", base64_encode(&bytes)))
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn package_import_queue_preview_image(
    args: PackageQueuePreviewImageArgs,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let queue_state = Arc::clone(&state.import_queue);
    tauri::async_runtime::spawn_blocking(move || {
        package_import_queue_preview_image_blocking(args, queue_state)
    })
    .await
    .map_err(|e| format!("导入图片预览后台任务失败：{}", e))?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn package_import_queue_cancel(
    args: PackageQueueIdArgs,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let mut slot = state.import_queue.lock().map_err(|_| "导入队列锁已损坏")?;
    if slot.as_ref().map(|queue| queue.id.as_str()) == Some(args.queue_id.as_str()) {
        *slot = None;
        let _ = crate::staging::remove(crate::staging::root().join(&args.queue_id));
    }
    Ok(())
}

pub(crate) fn package_import_queue_commit_blocking(
    args: PackageQueueCommitArgs,
    app: tauri::AppHandle,
    queue_state: Arc<Mutex<Option<PackageImportQueue>>>,
    mutation: Arc<Mutex<()>>,
) -> Result<usize, String> {
    if args.items.is_empty() {
        return Err("请至少选择一个 Mod".into());
    }
    // 提交期间保持队列所有权，失败时仍保留已校验的载荷以支持原任务重试。
    let mut slot = queue_state.lock().map_err(|_| "导入队列锁已损坏")?;
    let queue = slot
        .as_ref()
        .filter(|q| q.id == args.queue_id)
        .ok_or("导入队列不存在或已过期")?;
    let selected: HashMap<String, String> = args
        .items
        .into_iter()
        .map(|item| (item.id, item.apply_mode))
        .collect();
    let chosen: Vec<(&QueuedPackageFile, String)> = queue
        .files
        .iter()
        .filter_map(|file| selected.get(&file.id).cloned().map(|mode| (file, mode)))
        .collect();
    if chosen.is_empty() {
        return Err("未找到所选 Mod".into());
    }
    let _ = app.emit(
        "package-progress",
        serde_json::json!({"phase":"import","percent":5,"message":"正在校验并暂存导入内容"}),
    );
    let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let stage = crate::staging::create("package-commit")?;
    ensure_dir(&stage).map_err(|e| e.to_string())?;
    let mut lib = load_library();
    let vault = vault_dir();
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut reserved_destinations = HashSet::new();
    let mut created: Vec<PathBuf> = Vec::new();
    let mut created_dirs: Vec<PathBuf> = Vec::new();
    let mut to_apply = Vec::new();
    let mut apply_keys: HashSet<(String, String, String, String)> = HashSet::new();
    let mut game_backups: Vec<GameFileBackup> = Vec::new();
    let mut applied_canonicals: HashSet<String> = HashSet::new();
    let mut imported = 0usize;
    let mut duplicate_count = 0usize;
    let result: Result<(), String> = (|| {
        for (index, (file, apply_mode)) in chosen.iter().enumerate() {
            crate::tasks::checkpoint(&app)?;
            let actual_hash = if let Some(path) = &file.staged_path {
                sha256_file(path)?
            } else {
                sha256_bytes(&file.bytes)
            };
            if actual_hash != file.meta.hash.to_ascii_lowercase() {
                return Err("导入队列中的文件校验失败".into());
            }
            let meta = normalized_package_meta(&file.meta);
            if !meta.ext.eq_ignore_ascii_case(".png") {
                return Err("加密包仅支持 PNG Mod".into());
            }
            let resource = get_resource(
                &mut lib,
                &meta.category,
                &meta.entity_id,
                &meta.section_key,
                &meta.canonical,
            );
            if let Some(existing) = resource
                .files
                .iter_mut()
                .find(|entry| entry.hash == file.meta.hash)
            {
                existing
                    .info
                    .fill_empty(&crate::management_types::CandidateInfo {
                        provenance: meta.provenance.clone().unwrap_or_default(),
                        note: String::new(),
                    });
                if existing.source_name.trim().is_empty() {
                    existing.source_name = sanitize_name(&meta.source_name);
                }
                // Exact hash matches are informational duplicates.  They may
                // refresh the display name, but must never re-apply or create
                // another vault file, even when an older client sends an
                // apply mode for the row.
                duplicate_count += 1;
                continue;
            }
            let safe_apply_mode = apply_mode.as_str();
            let clean = sanitize_name(&meta.source_name);
            let expected_dir = vault
                .join(vault_top_dir(&meta.category, &meta.section_key))
                .join(sanitize_name(&meta.entity_id))
                .join(sanitize_name(&meta.section_key));
            let dir_was_present = expected_dir.exists();
            let (dest, rel) =
                reserve_package_vault_path(&vault, &meta, &mut reserved_destinations)?;
            if !dir_was_present {
                created_dirs.push(expected_dir);
            }
            let stage_file = stage.join(format!("{:05}.payload", index));
            if let Some(path) = &file.staged_path {
                fs::copy(path, &stage_file).map_err(|e| format!("写入导入暂存区失败：{}", e))?;
            } else {
                fs::write(&stage_file, &file.bytes)
                    .map_err(|e| format!("写入导入暂存区失败：{}", e))?;
            }
            crate::image_import::validate_png(&stage_file)?;
            resource.files.push(FileEntry {
                fs_identity: String::new(),
                hash: meta.hash.clone(),
                ext: meta.ext.clone(),
                source_name: clean,
                rel_path: rel,
                added_at: unix_now(),
                info: crate::management_types::CandidateInfo {
                    provenance: meta.provenance.clone().unwrap_or_default(),
                    note: String::new(),
                },
                classification: crate::management_types::ClassificationRecord {
                    manual: meta.manual_classification,
                    original_name: meta.source_name.clone(),
                    ..Default::default()
                },
            });
            if safe_apply_mode == "applyAll"
                || (safe_apply_mode == "fillEmpty" && resource.active_hash.is_none())
            {
                let key = (
                    meta.category.clone(),
                    meta.entity_id.clone(),
                    meta.section_key.clone(),
                    meta.canonical.clone(),
                );
                if apply_keys.insert(key) {
                    to_apply.push((
                        meta.entity_id.clone(),
                        meta.section_key.clone(),
                        meta.canonical.clone(),
                        meta.hash.clone(),
                    ));
                }
            }
            staged.push((stage_file, dest));
            imported += 1;
        }
        if args.use_application_state {
            let mut state_slots = HashSet::new();
            // Application-state records from old packages can contain the
            // same stale slot metadata as their file entries.  Associate the
            // requested hash with the selected file's current normalized key.
            let selected_metas: HashMap<String, PackageFileMeta> = chosen
                .iter()
                .map(|(file, _)| {
                    (
                        file.meta.hash.to_ascii_lowercase(),
                        normalized_package_meta(&file.meta),
                    )
                })
                .collect();
            for application_state in &queue.application_states {
                for desired in &application_state.enabled_resources {
                    let Some(meta) = selected_metas.get(&desired.hash.to_ascii_lowercase()) else {
                        continue;
                    };
                    let slot_key = format!(
                        "{}/{}/{}/{}",
                        meta.category, meta.entity_id, meta.section_key, meta.canonical
                    );
                    if !state_slots.insert(slot_key) {
                        continue;
                    }
                    let resource = lib_map(&lib, &meta.category)
                        .get(&meta.entity_id)
                        .and_then(|hero| hero.sections.get(&meta.section_key))
                        .and_then(|section| section.resources.get(&meta.canonical));
                    let Some(resource) = resource else {
                        continue;
                    };
                    if resource.disabled
                        || !resource
                            .files
                            .iter()
                            .any(|entry| entry.hash.eq_ignore_ascii_case(&desired.hash))
                    {
                        continue;
                    }
                    let key = (
                        meta.category.clone(),
                        meta.entity_id.clone(),
                        meta.section_key.clone(),
                        meta.canonical.clone(),
                    );
                    if apply_keys.insert(key) {
                        to_apply.push((
                            meta.entity_id.clone(),
                            meta.section_key.clone(),
                            meta.canonical.clone(),
                            desired.hash.clone(),
                        ));
                    }
                }
            }
        }
        if !to_apply.is_empty() && lib.settings.modimg_path.trim().is_empty() {
            return Err("未设置 modimg 目录，无法应用导入的 Mod".into());
        }
        if !to_apply.is_empty() {
            let modimg = Path::new(&lib.settings.modimg_path);
            let planned: HashSet<PathBuf> = staged.iter().map(|(_, dest)| dest.clone()).collect();
            for (id, section, canonical, hash) in &to_apply {
                let category = category_of(section);
                let resource = lib_map(&lib, &category)
                    .get(id)
                    .and_then(|hero| hero.sections.get(section))
                    .and_then(|part| part.resources.get(canonical))
                    .ok_or("导入应用预检失败：资源不存在")?;
                if resource.disabled {
                    return Err("导入应用预检失败：目标 Mod 已禁用".into());
                }
                let entry = resource
                    .files
                    .iter()
                    .find(|entry| entry.hash == *hash)
                    .ok_or("导入应用预检失败：文件不在库中")?;
                let source = vault_path(&vault, entry);

                if !source.exists() && !planned.contains(&source) {
                    return Err("导入应用预检失败：vault 文件不存在".into());
                }
                let backup_name = if section == "temporary"
                    || id.starts_with("temporary:")
                    || canonical.starts_with("unmatched:")
                {
                    let source_name = sanitize_name(&entry.source_name);
                    source_name
                        .rsplit_once('.')
                        .map(|(name, _)| name.to_string())
                        .unwrap_or(source_name)
                } else {
                    canonical.clone()
                };
                applied_canonicals.insert(backup_name);
            }
            game_backups = backup_modimg_for_queue(modimg, &applied_canonicals, &stage)?;
        }
        for (index, (stage_file, dest)) in staged.iter().enumerate() {
            crate::tasks::checkpoint(&app)?;
            let _ = app.emit("package-progress", serde_json::json!({"phase":"import","percent":20 + ((index * 65) / staged.len().max(1)),"message":"正在提交导入文件"}));
            if let Err(error) = crate::staging::copy_new(stage_file, dest) {
                return Err(format!("提交导入文件失败：{}", error));
            }
            created.push(dest.clone());
        }
        for (id, section, canonical, hash) in &to_apply {
            crate::tasks::checkpoint(&app)?;
            activate_resource(&mut lib, id, section, canonical, hash)?;
        }
        // Publish the index once, only after every selected file has been
        // moved and every requested apply operation has succeeded.  This is
        // what makes the queue rollback safe for library.json as well.
        crate::tasks::commit_phase(&app)?;
        save_library(&lib)?;
        Ok(())
    })();
    if let Err(error) = result {
        if !applied_canonicals.is_empty() && !lib.settings.modimg_path.is_empty() {
            if let Err(rollback) = restore_modimg_queue_backup(
                Path::new(&lib.settings.modimg_path),
                &applied_canonicals,
                &game_backups,
            ) {
                return Err(format!(
                    "{error}；回滚失败：{rollback}。备份保留在 {}",
                    stage.display()
                ));
            }
        }
        for path in created {
            let _ = fs::remove_file(path);
        }
        for directory in created_dirs.into_iter().rev() {
            let _ = fs::remove_dir(&directory);
        }
        let _ = crate::staging::remove(&stage);
        write_log(
            "error",
            "package.import.queue",
            &error,
            Some(&format!("{:?}", std::backtrace::Backtrace::force_capture())),
        );
        return Err(error);
    }
    let _ = crate::staging::remove(&stage);
    for file in &chosen {
        if let Some(path) = &file.0.staged_path {
            let _ = crate::staging::remove(path.parent().unwrap_or(Path::new(".")));
        }
    }
    let _ = crate::staging::remove(crate::staging::root().join(&args.queue_id));
    *slot = None;
    write_log(
        "info",
        "package.import.queue",
        &format!(
            "原子导入完成：新增 {} 个 Mod，跳过 {} 个同部位重复项",
            imported, duplicate_count
        ),
        None,
    );
    let _ = app.emit(
        "package-progress",
        serde_json::json!({"phase":"complete","percent":100,"message":"加密包导入完成"}),
    );
    Ok(imported)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn package_import_queue_commit(
    args: PackageQueueCommitArgs,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let queue_state = Arc::clone(&state.import_queue);
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || {
        package_import_queue_commit_blocking(args, app, queue_state, mutation)
    })
    .await
    .map_err(|e| format!("导入提交后台任务失败：{}", e))?
}

pub(crate) fn package_import_commit_blocking(
    args: PackageCommitArgs,
    app: tauri::AppHandle,
    queue_state: Arc<Mutex<Option<PackageImportQueue>>>,
    mutation: Arc<Mutex<()>>,
) -> Result<usize, String> {
    // Backward-compatible single-package entry point.  It deliberately feeds
    // the same queue transaction used by the UI so callers cannot bypass the
    // staging/rollback boundary.
    let is_v3 = package_is_v3(&args.path)?;
    let queue_id = new_queue_id();
    let stage = crate::staging::root().join(&queue_id).join("single");
    let (manifest, data, paths) = if is_v3 {
        let (manifest, paths) = unpack_package_v3(&args.path, &args.password, &stage)?;
        (manifest, Vec::new(), paths)
    } else {
        let (manifest, data) = package_contents(&args.path, &args.password)?;
        (manifest, data, Vec::new())
    };
    let wanted: HashSet<String> = args.selected_hashes.into_iter().collect();
    let mut files = Vec::new();
    let mut items = Vec::new();
    for (index, meta) in manifest.files.into_iter().enumerate() {
        if !wanted.is_empty() && !wanted.contains(&meta.hash) {
            continue;
        }
        let id = format!("{}:{}", queue_id, index);
        items.push(PackageQueueCommitItem {
            id: id.clone(),
            apply_mode: args.apply_mode.clone(),
        });
        files.push(QueuedPackageFile {
            id,
            meta,
            bytes: if is_v3 {
                Vec::new()
            } else {
                data[index].clone()
            },
            staged_path: if is_v3 {
                Some(paths[index].clone())
            } else {
                None
            },
        });
    }
    if files.is_empty() {
        return Err("未找到所选 Mod".into());
    }
    {
        let mut slot = queue_state.lock().map_err(|_| "导入队列锁已损坏")?;
        *slot = Some(PackageImportQueue {
            id: queue_id.clone(),
            files,
            application_states: manifest.application_state.into_iter().collect(),
        });
    }
    package_import_queue_commit_blocking(
        PackageQueueCommitArgs {
            queue_id,
            items,
            use_application_state: false,
        },
        app,
        queue_state,
        mutation,
    )
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn package_import_commit(
    args: PackageCommitArgs,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let queue_state = Arc::clone(&state.import_queue);
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || {
        package_import_commit_blocking(args, app, queue_state, mutation)
    })
    .await
    .map_err(|e| format!("导入后台任务失败：{}", e))?
}

#[cfg(test)]
mod v3_tests {
    use super::*;

    #[test]
    fn historical_v1_and_v2_headers_remain_readable() {
        let password = "legacy-fixture";
        let salt = [7u8; 16];
        let nonce = [9u8; 12];
        let key = package_key(
            password,
            &salt,
            PACKAGE_V1_KDF_MEMORY_KIB,
            PACKAGE_V1_KDF_ITERATIONS,
            1,
        )
        .unwrap();
        let mut v1 = Vec::from(PACKAGE_MAGIC_V1.as_slice());
        v1.extend_from_slice(&salt);
        v1.extend_from_slice(&nonce);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .unwrap()
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: b"historical payload",
                    aad: &v1,
                },
            )
            .unwrap();
        v1.extend_from_slice(&cipher);
        assert_eq!(
            decrypt_package(&v1, password).unwrap(),
            b"historical payload"
        );
        assert!(decrypt_package(&v1, "wrong").is_err());
        let v2 = encrypt_package(b"historical payload", password).unwrap();
        assert_eq!(&v2[..8], PACKAGE_MAGIC_V2);
        assert_eq!(
            decrypt_package(&v2, password).unwrap(),
            b"historical payload"
        );
    }

    #[test]
    fn legacy_same_name_items_reserve_distinct_paths_and_keep_hashes() {
        let root = crate::staging::root().join(format!(
            "astral-package-path-test-{}-{}",
            std::process::id(),
            new_queue_id()
        ));
        let vault = root.join("vault");
        fs::create_dir_all(&root).unwrap();
        let payloads = [
            b"first payload".as_slice(),
            b"second payload".as_slice(),
            b"third payload".as_slice(),
        ];
        let mut reserved = HashSet::new();
        let mut planned = Vec::new();
        for payload in payloads {
            let hash = sha256_bytes(payload);
            let meta = PackageFileMeta {
                category: "cards".into(),
                entity_id: "10007".into(),
                section_key: "hand_card".into(),
                canonical: "UT_HandCard_10007".into(),
                hash,
                ext: ".png".into(),
                source_name: "UT_HandCard_10007.png".into(),
                storage_name: None,
                was_enabled: false,
                entity_name: String::new(),
                section_name: String::new(),
                resource_name: String::new(),
                category_name: String::new(),
                category_order: 1,
                preview_url: None,
                provenance: None,
                manual_classification: false,
            };
            planned.push((
                reserve_package_vault_path(&vault, &meta, &mut reserved).unwrap(),
                payload,
            ));
        }
        let destinations: HashSet<PathBuf> =
            planned.iter().map(|((path, _), _)| path.clone()).collect();
        assert_eq!(destinations.len(), 3);
        for ((path, _), payload) in planned {
            fs::write(&path, payload).unwrap();
            assert_eq!(sha256_file(&path).unwrap(), sha256_bytes(payload));
        }
        let _ = crate::staging::remove(root);
    }

    #[test]
    fn package_import_path_allocation_preserves_existing_vault_file() {
        let root = crate::staging::root().join(format!(
            "astral-package-existing-path-test-{}-{}",
            std::process::id(),
            new_queue_id()
        ));
        let vault = root.join("vault");
        let target_dir = vault.join("cards").join("10007").join("hand_card");
        fs::create_dir_all(&target_dir).unwrap();
        let existing = target_dir.join("UT_HandCard_10007.png");
        fs::write(&existing, b"existing user data").unwrap();
        let existing_hash = sha256_file(&existing).unwrap();
        let meta = PackageFileMeta {
            category: "cards".into(),
            entity_id: "10007".into(),
            section_key: "hand_card".into(),
            canonical: "UT_HandCard_10007".into(),
            hash: sha256_bytes(b"incoming payload"),
            ext: ".png".into(),
            source_name: "UT_HandCard_10007.png".into(),
            storage_name: None,
            was_enabled: false,
            entity_name: String::new(),
            section_name: String::new(),
            resource_name: String::new(),
            category_name: String::new(),
            category_order: 1,
            preview_url: None,
            provenance: None,
            manual_classification: false,
        };
        let (destination, _) =
            reserve_package_vault_path(&vault, &meta, &mut HashSet::new()).unwrap();
        assert_ne!(destination, existing);
        assert_eq!(sha256_file(&existing).unwrap(), existing_hash);
        let _ = crate::staging::remove(root);
    }

    #[test]
    fn package_storage_name_is_portable_and_legacy_name_falls_back() {
        let mut meta = PackageFileMeta {
            category: "cards".into(),
            entity_id: "10007".into(),
            section_key: "hand_card".into(),
            canonical: "UT_HandCard_10007".into(),
            hash: "a".repeat(64),
            ext: ".png".into(),
            source_name: "UT_HandCard_10007.png".into(),
            storage_name: Some("cards/10007/UT_HandCard_10007_a1b2c3d4.png".into()),
            was_enabled: false,
            entity_name: String::new(),
            section_name: String::new(),
            resource_name: String::new(),
            category_name: String::new(),
            category_order: 1,
            preview_url: None,
            provenance: None,
            manual_classification: false,
        };
        assert_eq!(
            package_storage_name(&meta),
            "UT_HandCard_10007_a1b2c3d4.png"
        );
        meta.storage_name = None;
        assert_eq!(package_storage_name(&meta), "UT_HandCard_10007.png");
    }

    #[test]
    #[ignore = "requires ASTRAL_TEST_10007_DIR pointing at the three source PNGs"]
    fn real_10007_pngs_survive_encrypted_roundtrip_and_legacy_path_allocation() {
        let source_dir = PathBuf::from(
            std::env::var_os("ASTRAL_TEST_10007_DIR").expect("set ASTRAL_TEST_10007_DIR"),
        );
        let names = [
            "UT_HandCard_10007.png",
            "UT_HandCard_10007_542e6fbe.png",
            "UT_HandCard_10007_dbf36345.png",
        ];
        let root = crate::staging::root().join(format!(
            "astral-10007-roundtrip-{}-{}",
            std::process::id(),
            new_queue_id()
        ));
        fs::create_dir_all(&root).unwrap();
        let zip_path = root.join("source.zip");
        let target = root.join("10007.apmod");
        let mut payloads = Vec::new();
        let mut files = Vec::new();
        for name in names {
            let bytes = fs::read(source_dir.join(name)).unwrap();
            let hash = sha256_bytes(&bytes);
            payloads.push((hash.clone(), bytes));
            files.push(PackageFileMeta {
                category: "cards".into(),
                entity_id: "10007".into(),
                section_key: "hand_card".into(),
                canonical: "UT_HandCard_10007".into(),
                hash,
                ext: ".png".into(),
                source_name: "UT_HandCard_10007.png".into(),
                storage_name: Some(name.into()),
                was_enabled: false,
                entity_name: String::new(),
                section_name: String::new(),
                resource_name: String::new(),
                category_name: String::new(),
                category_order: 1,
                preview_url: None,
                provenance: None,
                manual_classification: false,
            });
        }
        let manifest = PackageManifest {
            format_version: 3,
            app_version: "test".into(),
            created_at: 0,
            files,
            application_state: None,
        };
        let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        for (hash, bytes) in &payloads {
            zip.start_file(format!("payload/{hash}"), options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();

        let volumes = write_package_v3(
            &zip_path,
            &target,
            "roundtrip-test",
            64 * 1024 * 1024,
            (8192, 1, 1),
        )
        .unwrap();
        let (roundtrip, extracted) =
            unpack_package_v3(&volumes[0], "roundtrip-test", &root.join("unpacked")).unwrap();
        assert_eq!(
            roundtrip
                .files
                .iter()
                .map(|meta| meta.storage_name.as_deref().unwrap())
                .collect::<Vec<_>>(),
            names
        );
        for (path, meta) in extracted.iter().zip(&roundtrip.files) {
            assert_eq!(sha256_file(path).unwrap(), meta.hash);
        }

        let vault = root.join("vault");
        let mut reserved = HashSet::new();
        let destinations: Vec<PathBuf> = roundtrip
            .files
            .iter()
            .map(|source| {
                let mut legacy = source.clone();
                legacy.storage_name = None;
                reserve_package_vault_path(&vault, &legacy, &mut reserved)
                    .unwrap()
                    .0
            })
            .collect();
        assert_eq!(
            destinations.iter().collect::<HashSet<_>>().len(),
            names.len()
        );
        for ((destination, source), expected) in
            destinations.iter().zip(&extracted).zip(&roundtrip.files)
        {
            fs::copy(source, destination).unwrap();
            assert_eq!(sha256_file(destination).unwrap(), expected.hash);
        }
        let _ = crate::staging::remove(root);
    }

    #[test]
    fn split_roundtrip_and_reject_missing_or_damaged_volume() {
        let root = crate::staging::root().join(format!("astral-v3-test-{}", new_queue_id()));
        fs::create_dir_all(&root).unwrap();
        let zip_path = root.join("source.zip");
        let payload = root.join("source.png");
        let mut bytes = vec![0u8; 18 * 1024 * 1024];
        let mut state = 0x1234_5678u32;
        for byte in &mut bytes {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        fs::write(&payload, &bytes).unwrap();
        let hash = sha256_file(&payload).unwrap();
        let meta = PackageFileMeta {
            category: "heroes".into(),
            entity_id: "112".into(),
            section_key: "avatars".into(),
            canonical: "test".into(),
            hash: hash.clone(),
            ext: ".png".into(),
            source_name: "test.png".into(),
            was_enabled: false,
            storage_name: Some("test.png".into()),
            entity_name: String::new(),
            section_name: String::new(),
            resource_name: String::new(),
            category_name: String::new(),
            category_order: 0,
            preview_url: None,
            provenance: None,
            manual_classification: false,
        };
        let manifest = PackageManifest {
            format_version: 3,
            app_version: "test".into(),
            created_at: 0,
            files: vec![meta],
            application_state: None,
        };
        let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        zip.start_file(format!("payload/{}", hash), options)
            .unwrap();
        zip.write_all(&bytes).unwrap();
        zip.finish().unwrap();
        drop(bytes);
        let target = root.join("example.apmod");
        let volumes =
            write_package_v3(&zip_path, &target, "secret", 16 * 1024 * 1024, (8192, 1, 1)).unwrap();
        assert!(volumes.len() >= 2);
        let (read_manifest, paths) =
            unpack_package_v3(&volumes[0], "secret", &root.join("ok")).unwrap();
        assert_eq!(read_manifest.files.len(), 1);
        assert_eq!(sha256_file(&paths[0]).unwrap(), hash);
        assert!(unpack_package_v3(&volumes[0], "wrong", &root.join("wrong")).is_err());
        fs::remove_file(volumes.last().unwrap()).unwrap();
        assert!(unpack_package_v3(&volumes[0], "secret", &root.join("missing")).is_err());
        let _ = crate::staging::remove(root);
    }
}
