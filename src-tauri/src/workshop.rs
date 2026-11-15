//! Curated, read-only workshop catalog. The publisher is local; only the
//! explicitly configured source may supply a catalog or archives.
use super::*;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::collections::HashSet;

const MAX_MANIFEST: u64 = 2 * 1024 * 1024;
const MAX_ARCHIVE: u64 = 64 * 1024 * 1024;
const MAX_UNPACKED: u64 = 256 * 1024 * 1024;
const PUBLISHER_KEY_B64: &str = "UI39wMOvv7FSpxLE4fOlkOv/QK4yPMrk6eblJNTS8zU=";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OwnedFile {
    hero_id: String,
    section_key: String,
    canonical: String,
    hash: String,
    source_name: String,
    rel_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstalledWork {
    version: String,
    files: Vec<OwnedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkFile {
    name: String,
    sha256: String,
    bytes: u64,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    resource: Option<WorkshopResource>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkshopResource {
    category: String,
    category_name: String,
    entity_id: String,
    entity_name: String,
    section_key: String,
    section_name: String,
    canonical: String,
    resource_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkTag {
    name: String,
    #[serde(default)]
    highlight: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Work {
    id: String,
    /// Stable public identifier, separate from the local installation key.
    #[serde(default)]
    published_id: Option<String>,
    title: String,
    description: String,
    category: String,
    #[serde(default)]
    tags: Vec<WorkTag>,
    version: String,
    cover: String,
    archive: String,
    sha256: String,
    bytes: u64,
    files: Vec<WorkFile>,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    cover_data_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    schema: u32,
    works: Vec<Work>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListedWork {
    work: Work,
    installed_version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Listing {
    available: bool,
    message: String,
    works: Vec<ListedWork>,
}

fn base_url() -> Option<String> {
    // Test override never enters release builds.
    #[cfg(debug_assertions)]
    if let Ok(raw) = std::env::var("ASTRAL_WORKSHOP_BASE_URL") {
        let trimmed = raw.trim_end_matches('/');
        if let Ok(url) = reqwest::Url::parse(trimmed) {
            if url.scheme() == "http"
                && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
                && url.port().is_some()
                && url.path() == "/"
                && url.query().is_none()
            {
                return Some(trimmed.to_string());
            }
        }
    }
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../config/cnb.json")).ok()?;
    let source = &config["workshop"];
    if source["enabled"] != true {
        return None;
    }
    let raw = source["rawPrefix"].as_str()?.trim_end_matches('/');
    let url = reqwest::Url::parse(raw).ok()?;
    if url.scheme() == "https"
        && url.host_str() == Some("cnb.cool")
        && url.path() == "/astral-weird-egg/astral-party-mod-manager-assets/-/git/raw/main/workshop"
        && url.query().is_none()
        && url.fragment().is_none()
    {
        Some(raw.to_string())
    } else {
        None
    }
}

fn verify_manifest(bytes: &[u8], signature: &[u8]) -> Result<(), String> {
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(PUBLISHER_KEY_B64)
        .map_err(|_| "发布者公钥无效")?;
    let key: [u8; 32] = key_bytes.try_into().map_err(|_| "发布者公钥无效")?;
    let verifier = VerifyingKey::from_bytes(&key).map_err(|_| "发布者公钥无效")?;
    let signature = Signature::from_slice(signature).map_err(|_| "作品清单签名无效")?;
    verifier
        .verify(bytes, &signature)
        .map_err(|_| "作品清单签名验证失败".into())
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.contains("..")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn valid_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn resource_for_file(lib: &Library, file: &WorkFile) -> Option<WorkshopResource> {
    let parsed = parse_filename(&file.name)?;
    let meta = PackageFileMeta {
        category: category_of(&parsed.section_key),
        entity_id: parsed.hero_id,
        section_key: parsed.section_key,
        canonical: parsed.canonical,
        hash: file.sha256.clone(),
        ext: ".png".into(),
        source_name: file.name.clone(),
        storage_name: None,
        was_enabled: false,
        entity_name: String::new(),
        section_name: String::new(),
        resource_name: String::new(),
        category_name: String::new(),
        category_order: 0,
        preview_url: None,
        provenance: None,
        manual_classification: false,
    };
    let meta = localize_package_meta(lib, meta);
    Some(WorkshopResource {
        category: meta.category,
        category_name: meta.category_name,
        entity_id: meta.entity_id,
        entity_name: meta.entity_name,
        section_key: meta.section_key,
        section_name: meta.section_name,
        canonical: meta.canonical,
        resource_name: meta.resource_name,
    })
}

fn read_http(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|e| format!("请求失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("下载失败：{e}"))?;
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("远端文件超过大小限制".into());
    }
    let mut out = Vec::new();
    let mut chunk = [0u8; 32768];
    loop {
        let n = response.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if out.len() as u64 + n as u64 > limit {
            return Err("远端文件超过大小限制".into());
        }
        out.extend_from_slice(&chunk[..n]);
    }
    Ok(out)
}

fn catalog(base: &str) -> Result<Vec<Work>, String> {
    let bytes = read_http(&format!("{base}/manifest.json"), MAX_MANIFEST)?;
    #[cfg(debug_assertions)]
    let local_test = base.starts_with("http://127.0.0.1:") || base.starts_with("http://localhost:");
    #[cfg(not(debug_assertions))]
    let local_test = false;
    if !local_test {
        let signature = read_http(&format!("{base}/manifest.sig"), 128)?;
        verify_manifest(&bytes, &signature)?;
    }
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("作品清单无效：{e}"))?;
    if manifest.schema != 1 || manifest.works.len() > 500 {
        return Err("不支持的作品清单".into());
    }
    let mut ids = HashSet::new();
    let mut published_ids = HashSet::new();
    for work in &manifest.works {
        if !safe_name(&work.id)
            || !ids.insert(work.id.clone())
            || work.title.trim().is_empty()
            || work.version.trim().is_empty()
            || !safe_name(&work.archive)
            || !safe_name(&work.cover)
            || !valid_hash(&work.sha256)
            || work.bytes == 0
            || work.bytes > MAX_ARCHIVE
            || work.files.is_empty()
            || work.files.len() > 512
            || work.tags.len() > 16
        {
            return Err("作品清单含无效条目".into());
        }
        if let Some(public_id) = &work.published_id {
            if public_id.is_empty()
                || public_id.len() > 32
                || !public_id.bytes().all(|b| b.is_ascii_digit())
                || !published_ids.insert(public_id.clone())
            {
                return Err("作品公开编号无效或重复".into());
            }
        }
        let mut tag_names = HashSet::new();
        for tag in &work.tags {
            let name = tag.name.trim();
            if name.is_empty()
                || name.chars().count() > 24
                || !tag_names.insert(name.to_lowercase())
            {
                return Err("作品标签无效".into());
            }
        }
        let mut names = HashSet::new();
        let mut total = 0u64;
        for file in &work.files {
            if !safe_name(&file.name)
                || !file.name.to_ascii_lowercase().ends_with(".png")
                || !valid_hash(&file.sha256)
                || file.bytes == 0
                || parse_filename(&file.name).is_none()
                || !names.insert(file.name.clone())
            {
                return Err(format!("作品 {} 含无效 PNG", work.id));
            }
            total = total.saturating_add(file.bytes);
        }
        if total > MAX_UNPACKED {
            return Err("作品解压大小超限".into());
        }
    }
    Ok(manifest.works)
}

#[tauri::command]
pub(crate) async fn workshop_list() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let Some(base) = base_url() else {
            return serde_json::to_value(Listing {
                available: false,
                message: "暂无在线作品；本地测试源未启用".into(),
                works: Vec::new(),
            })
            .map_err(|e| e.to_string());
        };
        let lib = load_library();
        let installed = &lib.workshop;
        let works = catalog(&base)?
            .into_iter()
            .map(|mut work| {
                for file in &mut work.files {
                    file.resource = resource_for_file(&lib, file);
                }
                if let Ok(cover) = read_http(&format!("{base}/{}", work.cover), 2 * 1024 * 1024) {
                    if cover.starts_with(b"\x89PNG\r\n\x1a\n") {
                        work.cover_data_url = Some(format!(
                            "data:image/png;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(cover)
                        ));
                    }
                }
                let installed_version = installed.get(&work.id).map(|w| w.version.clone());
                ListedWork {
                    work,
                    installed_version,
                }
            })
            .collect();
        serde_json::to_value(Listing {
            available: true,
            message: String::new(),
            works,
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn cache_archive(base: &str, work: &Work, app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let state = app.state::<crate::tasks::TaskManager>();
    let flag = state
        .current()
        .map(|(_, flag)| flag)
        .unwrap_or_else(|| std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    crate::downloads::fetch(
        crate::downloads::Download {
            identity: &format!("workshop:{}:{}", work.id, work.version),
            urls: vec![format!("{base}/{}", work.archive)],
            bytes: work.bytes,
            sha256: &work.sha256,
        },
        &flag,
        |done, total, speed| state.download_progress(app, done, total, speed),
    )
}

fn unpack_checked(work: &Work, archive: &Path, stage: &Path) -> Result<Vec<PathBuf>, String> {
    let file = fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("作品 ZIP 损坏：{e}"))?;
    if zip.len() != work.files.len() {
        return Err("ZIP 文件数与清单不符".into());
    }
    ensure_dir(stage).map_err(|e| e.to_string())?;
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if entry.is_dir() || !safe_name(&name) || !seen.insert(name.clone()) {
            return Err("ZIP 含非法路径或重复文件".into());
        }
        let meta = work
            .files
            .iter()
            .find(|item| item.name == name)
            .ok_or("ZIP 含清单外文件")?;
        if entry.size() != meta.bytes {
            return Err(format!("{} 大小不符", name));
        }
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(meta.bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != meta.bytes
            || !sha256_bytes(&bytes).eq_ignore_ascii_case(&meta.sha256)
            || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        {
            return Err(format!("{} 校验失败", name));
        }
        let path = stage.join(name);
        fs::write(&path, bytes).map_err(|e| e.to_string())?;
        paths.push(path);
    }
    Ok(paths)
}

fn all_rels(lib: &Library) -> HashSet<String> {
    lib.heroes
        .values()
        .chain(lib.monsters.values())
        .flat_map(|hero| hero.sections.values())
        .flat_map(|section| section.resources.values())
        .flat_map(|resource| resource.files.iter().map(|f| f.rel_path.clone()))
        .collect()
}

fn resource_mut<'a>(lib: &'a mut Library, owned: &OwnedFile) -> Option<&'a mut ResourceEntry> {
    let cat = category_of(&owned.section_key);
    lib_map_mut(lib, &cat)
        .get_mut(&owned.hero_id)?
        .sections
        .get_mut(&owned.section_key)?
        .resources
        .get_mut(&owned.canonical)
}

fn remove_owned(lib: &mut Library, owned: &OwnedFile, allow_active: bool) -> bool {
    let Some(res) = resource_mut(lib, owned) else {
        return false;
    };
    if res.disabled || (!allow_active && res.active_hash.as_deref() == Some(owned.hash.as_str())) {
        return false;
    }
    let Some(index) = res.files.iter().position(|f| {
        f.hash == owned.hash && f.source_name == owned.source_name && f.rel_path == owned.rel_path
    }) else {
        return false;
    };
    let path = vault_dir().join(&owned.rel_path);
    if !path.exists() || sha256_file(&path).ok().as_deref() != Some(&owned.hash) {
        return false;
    }
    res.files.remove(index);
    if res.active_hash.as_deref() == Some(&owned.hash) {
        res.active_hash = None;
    }
    true
}

fn apply_transaction(
    work: &Work,
    paths: &[PathBuf],
    old: &Library,
    stage: &Path,
) -> Result<(Library, usize), String> {
    let mut lib = old.clone();
    let previous = old.workshop.get(&work.id).cloned();
    let mut created = Vec::new();
    let mut owned_files = Vec::new();
    let mut touched = HashSet::new();
    for path in paths {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let parsed = parse_filename(&name).ok_or("作品含无法归类的 PNG")?;
        touched.insert(parsed.canonical.clone());
    }
    if let Some(prev) = &previous {
        for f in &prev.files {
            touched.insert(f.canonical.clone());
        }
    }
    let modimg = PathBuf::from(lib.settings.modimg_path.as_str());
    let backup = if !lib.settings.modimg_path.is_empty() {
        backup_modimg_for_queue(&modimg, &touched, stage)?
    } else {
        Vec::new()
    };
    let result = (|| -> Result<usize, String> {
        for path in paths {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let parsed = parse_filename(&name).ok_or("作品含无法归类的 PNG")?;
            let expected_hash = sha256_file(path)?;
            let (target, _) = place_in_vault(
                &vault_dir(),
                &category_of(&parsed.section_key),
                &parsed.hero_id,
                &parsed.section_key,
                &name,
                &expected_hash,
                ".png",
            )?;
            let target_existed = target.exists();
            let (imported, first) = import_single(
                &mut lib,
                &vault_dir(),
                &path.to_string_lossy(),
                &parsed.hero_id,
                &parsed.section_key,
                &parsed.canonical,
                None,
            )
            .map_err(|r| r.reason.unwrap_or_else(|| "导入失败".into()))?;
            if !imported.duplicate {
                let rel = imported.rel_path.clone().ok_or("缺少资源路径")?;
                if !target_existed {
                    created.push(rel.clone());
                }
                owned_files.push(OwnedFile {
                    hero_id: parsed.hero_id.clone(),
                    section_key: parsed.section_key.clone(),
                    canonical: parsed.canonical.clone(),
                    hash: imported.hash.clone().unwrap_or_default(),
                    source_name: name,
                    rel_path: rel,
                });
            }
            if first && !lib.settings.modimg_path.is_empty() {
                activate_resource(
                    &mut lib,
                    &parsed.hero_id,
                    &parsed.section_key,
                    &parsed.canonical,
                    imported.hash.as_deref().unwrap_or(""),
                )?;
            }
        }
        let mut retained = Vec::new();
        let incoming: HashSet<(String, String)> = paths
            .iter()
            .filter_map(|path| {
                let name = path.file_name()?.to_string_lossy().to_string();
                let parsed = parse_filename(&name)?;
                let hash = work.files.iter().find(|f| f.name == name)?.sha256.clone();
                Some((parsed.canonical, hash.to_ascii_lowercase()))
            })
            .collect();
        if let Some(prev) = previous {
            for file in prev.files {
                if incoming.contains(&(file.canonical.clone(), file.hash.clone())) {
                    retained.push(file);
                    continue;
                }
                if !remove_owned(&mut lib, &file, false) {
                    retained.push(file);
                }
            }
        }
        let count = owned_files.len();
        owned_files.extend(retained);
        lib.workshop.insert(
            work.id.clone(),
            InstalledWork {
                version: work.version.clone(),
                files: owned_files,
            },
        );
        save_library(&lib)?;
        Ok(count)
    })();
    if result.is_err() {
        if !lib.settings.modimg_path.is_empty() {
            restore_modimg_queue_backup(&modimg, &touched, &backup)
                .map_err(|e| format!("回滚失败：{e}；备份保留于 {}", stage.display()))?;
        }
        for rel in created {
            let _ = fs::remove_file(vault_dir().join(rel));
        }
    }
    result.map(|count| (lib, count))
}

#[tauri::command]
pub(crate) async fn workshop_install(
    id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || {
        let base = base_url().ok_or("工坊下载源未启用")?;
        let work = catalog(&base)?
            .into_iter()
            .find(|w| w.id == id)
            .ok_or("作品不存在")?;
        let archive = cache_archive(&base, &work, &app)?;
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let stage = crate::staging::create("workshop-install")?;
        let result = (|| {
            let paths = unpack_checked(&work, &archive, &stage)?;
            let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
            let original = load_library();
            if original
                .workshop
                .get(&id)
                .is_some_and(|v| v.version == work.version)
            {
                return Ok(0);
            }
            let (updated, count) = apply_transaction(&work, &paths, &original, &stage)?;
            let remaining = all_rels(&updated);
            for rel in all_rels(&original).difference(&remaining) {
                let _ = fs::remove_file(vault_dir().join(rel));
            }
            let _ = app.emit(
                "workshop-progress",
                serde_json::json!({"phase":"complete","message":"安装完成"}),
            );
            Ok(count)
        })();
        if result.is_ok() {
            let _ = crate::staging::remove(&stage);
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn workshop_uninstall(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
        let mut lib = load_library();
        let original = lib.clone();
        let installed = lib.workshop.get(&id).cloned().ok_or("作品尚未安装")?;
        let touched: HashSet<String> = installed
            .files
            .iter()
            .map(|f| f.canonical.clone())
            .collect();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let stage = crate::staging::create("workshop-uninstall")?;
        ensure_dir(&stage).map_err(|e| e.to_string())?;
        let modimg = PathBuf::from(lib.settings.modimg_path.as_str());
        let backup = if !lib.settings.modimg_path.is_empty() {
            backup_modimg_for_queue(&modimg, &touched, &stage)?
        } else {
            Vec::new()
        };
        let mut removed = Vec::new();
        let mut retained = Vec::new();
        let result = (|| -> Result<(), String> {
            for file in installed.files {
                let active = resource_mut(&mut lib, &file)
                    .is_some_and(|r| r.active_hash.as_deref() == Some(file.hash.as_str()));
                if active {
                    if lib.settings.modimg_path.is_empty() {
                        retained.push(file);
                        continue;
                    }
                    let game_files = canonical_modimg_files(&modimg, &file.canonical)?;
                    if game_files
                        .iter()
                        .any(|path| sha256_file(path).ok().as_deref() != Some(file.hash.as_str()))
                    {
                        retained.push(file);
                        continue;
                    }
                }
                if remove_owned(&mut lib, &file, true) {
                    if active && !lib.settings.modimg_path.is_empty() {
                        purge_modimg(&modimg, &file.canonical)?;
                    }
                    removed.push(file.rel_path);
                } else {
                    retained.push(file);
                }
            }
            if retained.is_empty() {
                lib.workshop.remove(&id);
            } else {
                lib.workshop.insert(
                    id.clone(),
                    InstalledWork {
                        version: installed.version,
                        files: retained.clone(),
                    },
                );
            }
            save_library(&lib)?;
            Ok(())
        })();
        if result.is_err() {
            if !original.settings.modimg_path.is_empty() {
                restore_modimg_queue_backup(&modimg, &touched, &backup)
                    .map_err(|e| format!("回滚失败：{e}；备份保留于 {}", stage.display()))?;
            }
        } else {
            let remaining = all_rels(&lib);
            for rel in &removed {
                if !remaining.contains(rel) {
                    let _ = fs::remove_file(vault_dir().join(rel));
                }
            }
        }
        if result.is_ok() {
            let _ = crate::staging::remove(&stage);
        }
        result.map(|_| serde_json::json!({"removed":removed.len(), "retained":retained.len()}))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_work(name: &str, bytes: &[u8]) -> Work {
        Work {
            id: "test".into(),
            title: "Test".into(),
            description: String::new(),
            category: "头像".into(),
            version: "1".into(),
            cover: "cover.png".into(),
            archive: "test.zip".into(),
            sha256: "0".repeat(64),
            bytes: 1,
            tags: Vec::new(),
            files: vec![WorkFile {
                name: name.into(),
                sha256: sha256_bytes(bytes),
                bytes: bytes.len() as u64,
                resource: None,
            }],
            cover_data_url: None,
            published_id: None,
        }
    }

    fn sample_zip(path: &Path, name: &str, bytes: &[u8]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn archive_validation_rejects_extra_path_and_changed_png() {
        let root = std::env::temp_dir().join(format!(
            "astral-workshop-test-{}-{}",
            std::process::id(),
            unix_now()
        ));
        fs::create_dir_all(&root).unwrap();
        let name = "UT_Hero_ProfilePhoto_112.png";
        let work = sample_work(name, PLACEHOLDER_PNG);
        let archive = root.join("test.zip");
        sample_zip(&archive, name, PLACEHOLDER_PNG);
        assert_eq!(
            unpack_checked(&work, &archive, &root.join("good"))
                .unwrap()
                .len(),
            1
        );
        sample_zip(&archive, "../escape.png", PLACEHOLDER_PNG);
        assert!(unpack_checked(&work, &archive, &root.join("bad-path")).is_err());
        sample_zip(&archive, name, b"broken");
        assert!(unpack_checked(&work, &archive, &root.join("bad-hash")).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn older_work_without_tags_and_localized_resource_tree() {
        let name = "UT_Hero_ProfilePhoto_112_01.png";
        let work = sample_work(name, PLACEHOLDER_PNG);
        let mut json = serde_json::to_value(&work).unwrap();
        json.as_object_mut().unwrap().remove("tags");
        let old: Work = serde_json::from_value(json).unwrap();
        assert!(old.tags.is_empty());
        let resource = resource_for_file(&Library::default(), &old.files[0]).unwrap();
        assert_eq!(resource.category_name, "角色");
        assert_eq!(resource.entity_name, "璐璐");
        assert_eq!(resource.section_name, "小头像");
        assert_eq!(resource.resource_name, "皮肤一");
    }
}
