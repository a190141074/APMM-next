//! 固定来源的签名目录与按需图片读取。
use super::*;
pub(super) fn base_url() -> Option<String> {
    // 测试源仅在调试构建可用，正式构建只读取固定配置。
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
        serde_json::from_str(include_str!("../../../config/cnb.json")).ok()?;
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

pub(super) fn verify_manifest(bytes: &[u8], signature: &[u8]) -> Result<(), String> {
    #[cfg(debug_assertions)]
    let test_key = std::env::var("APMM_WORKSHOP_TEST_PUBLIC_KEY").ok();
    #[cfg(not(debug_assertions))]
    let test_key: Option<String> = None;
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(test_key.as_deref().unwrap_or(PUBLISHER_KEY_B64))
        .map_err(|_| "发布者公钥无效")?;
    let key: [u8; 32] = key_bytes.try_into().map_err(|_| "发布者公钥无效")?;
    let verifier = VerifyingKey::from_bytes(&key).map_err(|_| "发布者公钥无效")?;
    let signature = Signature::from_slice(signature).map_err(|_| "作品清单签名无效")?;
    verifier
        .verify(bytes, &signature)
        .map_err(|_| "作品清单签名验证失败".into())
}

pub(super) fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.contains("..")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

pub(super) fn valid_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

pub(super) fn resource_for_file(lib: &Library, file: &WorkFile) -> Option<WorkshopResource> {
    let parsed = file.parsed().ok()?;
    let meta = PackageFileMeta {
        category: category_of(&parsed.section_key),
        entity_id: parsed.hero_id,
        section_key: parsed.section_key,
        canonical: parsed.canonical,
        hash: file.sha256.clone(),
        ext: ".png".into(),
        source_name: file.source().to_string(),
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

pub(super) fn read_http(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .redirect(Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|_| "工坊连接失败，请重试")?
        .error_for_status()
        .map_err(|_| "工坊文件下载失败")?;
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

pub(super) fn catalog(base: &str) -> Result<Vec<Work>, String> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .redirect(Policy::none())
        .build()
        .map_err(|_| "无法创建目录连接")?;
    let response = client
        .get(format!("{base}/manifest-v2.json"))
        .send()
        .map_err(|_| "无法连接工坊目录")?;
    let manifest: Manifest = if response.status() == reqwest::StatusCode::NOT_FOUND {
        let bytes = read_http(&format!("{base}/manifest.json"), MAX_MANIFEST)?;
        let signature = read_http(&format!("{base}/manifest.sig"), 128)?;
        verify_manifest(&bytes, &signature)?;
        let value: Manifest = serde_json::from_slice(&bytes).map_err(|_| "作品清单无效")?;
        if value.schema != 1 {
            return Err("不支持的旧目录版本".into());
        }
        value
    } else {
        let mut response = response
            .error_for_status()
            .map_err(|_| "目录服务返回错误")?;
        let mut bytes = Vec::new();
        response
            .by_ref()
            .take(MAX_MANIFEST * 2 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "目录读取中断")?;
        if bytes.len() as u64 > MAX_MANIFEST * 2 {
            return Err("作品清单超限".into());
        }
        let envelope: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "签名目录无效")?;
        let decode = |key: &str| {
            base64::engine::general_purpose::STANDARD
                .decode(envelope[key].as_str().unwrap_or(""))
                .map_err(|_| "目录编码无效")
        };
        let payload = decode("payload")?;
        if payload.len() as u64 > MAX_MANIFEST {
            return Err("作品清单超限".into());
        }
        verify_manifest(&payload, &decode("signature")?)?;
        let value: Manifest = serde_json::from_slice(&payload).map_err(|_| "作品清单无效")?;
        if value.schema != 2 {
            return Err("不支持的作品目录版本".into());
        }
        value
    };
    validate_manifest(&manifest)?;
    Ok(manifest.works)
}

pub(super) fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.works.len() > 500 {
        return Err("作品清单超限".into());
    }
    let mut ids = HashSet::new();
    let mut published_ids = HashSet::new();
    for work in &manifest.works {
        attribution::validate(
            &work.author,
            &work.artist,
            &work.author_urls,
            &work.artist_urls,
            &work.source_url,
        )?;
        if !safe_name(&work.id)
            || !ids.insert(work.id.clone())
            || work.title.trim().is_empty()
            || work.version.trim().is_empty()
            || !safe_name(&work.archive)
            || (!work.cover.is_empty() && !safe_name(&work.cover))
            || work.previews.len() > 6
            || work.previews.iter().any(|name| !safe_name(name))
            || work.title.chars().count() > 120
            || work.description.chars().count() > 10000
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
                || !safe_source(file.source())
                || file.parsed().is_err()
                || !names.insert(file.name.to_lowercase())
            {
                return Err(format!("作品 {} 含无效 PNG", work.id));
            }
            total = total.saturating_add(file.bytes);
        }
        if total > MAX_UNPACKED {
            return Err("作品解压大小超限".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn workshop_list() -> Result<Listing, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let Some(base) = base_url() else {
            return Ok(Listing {
                available: false,
                status: "unconfigured".into(),
                message: "工坊来源尚未配置".into(),
                works: vec![],
            });
        };
        let entries = match catalog(&base) {
            Ok(entries) => entries,
            Err(error) => {
                return Ok(Listing {
                    available: false,
                    status: "unavailable".into(),
                    message: error,
                    works: vec![],
                })
            }
        };
        let lib = load_library();
        let works = entries
            .into_iter()
            .map(|mut work| {
                for file in &mut work.files {
                    file.resource = resource_for_file(&lib, file);
                }
                let installed = lib.workshop.get(&work.id);
                let locally_modified = installed.is_some_and(|entry| {
                    entry.files.iter().any(|f| {
                        sha256_file(&vault_dir().join(&f.rel_path)).ok().as_deref()
                            != Some(f.hash.as_str())
                    }) || (entry.archive_hash == work.sha256
                        && work.files.iter().any(|file| {
                            let Ok(parsed) = file.parsed() else {
                                return true;
                            };
                            let resource = lib_map(&lib, &category_of(&parsed.section_key))
                                .get(&parsed.hero_id)
                                .and_then(|hero| hero.sections.get(&parsed.section_key))
                                .and_then(|section| section.resources.get(&parsed.canonical));
                            !resource.is_some_and(|resource| {
                                resource.files.iter().any(|candidate| {
                                    candidate.hash == file.sha256
                                        && sha256_file(&vault_path(&vault_dir(), candidate))
                                            .ok()
                                            .as_deref()
                                            == Some(file.sha256.as_str())
                                })
                            })
                        }))
                });
                let update_available = installed.is_some_and(|entry| {
                    entry.version != work.version
                        || (!entry.archive_hash.is_empty() && entry.archive_hash != work.sha256)
                });
                ListedWork {
                    installed_version: installed.map(|v| v.version.clone()),
                    locally_modified,
                    update_available,
                    work,
                }
            })
            .collect();
        Ok(Listing {
            available: true,
            status: "ready".into(),
            message: String::new(),
            works,
        })
    })
    .await
    .map_err(|_| "工坊读取任务中断")?
}

/// 仅接受已签名目录里的图片，按内容地址缓存，读取与列表请求解耦。
#[tauri::command]
pub(crate) async fn workshop_image(id: String, name: String) -> Result<String, String> {
    static LANE: std::sync::OnceLock<Arc<tokio::sync::Semaphore>> = std::sync::OnceLock::new();
    let _permit = LANE
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(3)))
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| "图片读取已停止")?;
    tauri::async_runtime::spawn_blocking(move || {
        let base = base_url().ok_or("工坊来源尚未配置")?;
        let work = catalog(&base)?
            .into_iter()
            .find(|work| work.id == id)
            .ok_or("作品不存在")?;
        if name != work.cover && !work.previews.contains(&name) {
            return Err("图片不在作品清单中".into());
        }
        let cache = data_dir().join("cache/workshop");
        ensure_dir(&cache).map_err(|e| e.to_string())?;
        let key = sha256_bytes(format!("{base}/{name}:{}", work.revision).as_bytes());
        let path = cache.join(format!("{key}.png"));
        let bytes = match fs::read(&path) {
            Ok(bytes) if validate_png(&bytes).is_ok() => bytes,
            _ => {
                let bytes = read_http(&format!("{base}/{name}"), 2 * 1024 * 1024)?;
                validate_png(&bytes)?;
                let pending = cache.join(format!("{key}-{}.pending", uuid::Uuid::new_v4()));
                fs::write(&pending, &bytes).map_err(|e| e.to_string())?;
                if let Err(error) = fs::rename(&pending, &path) {
                    let _ = fs::remove_file(&pending);
                    if !path.exists() {
                        return Err(error.to_string());
                    }
                }
                bytes
            }
        };
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    })
    .await
    .map_err(|_| "图片读取任务中断")?
}

/// 展示文件名可以包含中文，但不能包含路径、控制字符或 Windows 保留名称。
pub(crate) fn safe_source(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    !name.is_empty()
        && name.len() <= 240
        && !name.ends_with([' ', '.'])
        && !name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
        && name.to_lowercase().ends_with(".png")
}
