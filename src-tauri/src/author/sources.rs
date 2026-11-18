//! 本地投稿源解析和预览副本生成，内容 PNG 始终保持原始字节。
use super::*;
use base64::Engine;
const MAX_BYTES: u64 = 256 * 1024 * 1024;

pub(super) fn classified(canonical: &str) -> bool {
    parse_filename(&format!("{canonical}.png")).is_some_and(|parsed| parsed.canonical == canonical)
}
pub(super) fn file(name: String, canonical: String, bytes: &[u8]) -> Result<AuthorFile, String> {
    if !crate::workshop::safe_source(&name) {
        return Err("资源展示名无效".into());
    }
    crate::workshop::validate_png(bytes)?;
    Ok(AuthorFile {
        id: uuid::Uuid::new_v4().to_string(),
        source_name: name,
        canonical,
        sha256: put_blob(bytes)?,
        bytes: bytes.len() as u64,
    })
}
pub(super) fn png_zip(path: &Path) -> Result<Vec<AuthorFile>, String> {
    if fs::metadata(path).map_err(|_| "ZIP 不存在")?.len() > 64 * 1024 * 1024 {
        return Err("ZIP 超过 64 MiB".into());
    }
    let mut zip = zip::ZipArchive::new(fs::File::open(path).map_err(|_| "ZIP 读取失败")?)
        .map_err(|_| "ZIP 无效")?;
    if zip.is_empty() || zip.len() > 512 {
        return Err("ZIP 需要包含 1 至 512 个 PNG".into());
    }
    let mut out = Vec::new();
    let mut total = 0;
    let mut seen = HashSet::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|_| "ZIP 条目无效")?;
        let name = entry.name().replace('\\', "/");
        if name.starts_with('/')
            || name
                .split('/')
                .any(|part| part == ".." || part.is_empty() || part.contains(':'))
            || entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            || !seen.insert(name.to_lowercase())
        {
            return Err("ZIP 含非法路径、重复条目或非 PNG 内容".into());
        }
        total += entry.size();
        if total > MAX_BYTES {
            return Err("ZIP 展开超过 256 MiB".into());
        }
        let size = entry.size();
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(size + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "ZIP 读取失败")?;
        if bytes.len() as u64 != size {
            return Err("ZIP 长度不符".into());
        }
        let display = name.rsplit('/').next().unwrap().to_string();
        let canonical = parse_filename(&display)
            .map(|p| p.canonical)
            .unwrap_or_default();
        out.push((display, canonical, bytes));
    }
    out.into_iter()
        .map(|(name, canonical, bytes)| file(name, canonical, &bytes))
        .collect()
}
fn attach(id: &str, files: Vec<AuthorFile>, images: bool) -> Result<AuthorDraft, String> {
    let _guard = lock()?;
    let mut state = load()?;
    let draft = draft_mut(&mut state, id)?;
    if images {
        if draft.images.len() + files.len() > 7 {
            return Err("最多一张封面和六张附图".into());
        }
        draft.images.extend(files);
    } else {
        if draft.files.len() + files.len() > 512
            || draft
                .files
                .iter()
                .chain(&files)
                .map(|f| f.bytes)
                .sum::<u64>()
                > MAX_BYTES
        {
            return Err("草稿资源数或大小超限".into());
        }
        draft.files.extend(files);
    }
    touch(draft);
    let result = draft.clone();
    save(&state)?;
    Ok(result)
}

/// 密码仅存在本次调用栈，解析结果不含密码或原始来源路径。
#[tauri::command]
pub(crate) async fn author_add_source(
    id: String,
    path: String,
    password: String,
    preview: bool,
) -> Result<AuthorDraft, String> {
    checked_id(&id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let path = Path::new(&path);
        let files = if preview {
            vec![cover::prepare(path)?]
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("apmod"))
        {
            crate::packages::author_package_files(&path.to_string_lossy(), &password)?
                .into_iter()
                .map(|(meta, bytes)| {
                    let canonical = parse_filename(&format!("{}.png", meta.canonical))
                        .map(|p| p.canonical)
                        .unwrap_or_default();
                    file(meta.source_name, canonical, &bytes)
                })
                .collect::<Result<Vec<_>, _>>()?
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        {
            if let Some(draft) = offline::import(&id, path)? {
                return Ok(draft);
            }
            png_zip(path)?
        } else {
            if fs::metadata(path).map_err(|_| "资源不存在")?.len() > MAX_BYTES {
                return Err("资源文件过大".into());
            }
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let canonical = parse_filename(&name)
                .map(|p| p.canonical)
                .unwrap_or_default();
            vec![file(
                name,
                canonical,
                &fs::read(path).map_err(|_| "资源读取失败")?,
            )?]
        };
        attach(&id, files, preview)
    })
    .await
    .map_err(|_| "解析任务中断")?
}

#[tauri::command]
pub(crate) async fn author_add_library(
    id: String,
    paths: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<AuthorDraft, String> {
    if paths.is_empty() || paths.len() > 512 {
        return Err("请选择 1 至 512 个资源".into());
    }
    checked_id(&id)?;
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = mutation.lock().map_err(|_| "资源库锁不可用")?;
        let lib = load_library();
        let mut files = Vec::new();
        let mut total = 0u64;
        for path in paths {
            let mut found = None;
            for hero in lib.heroes.values().chain(lib.monsters.values()) {
                for section in hero.sections.values() {
                    for (canonical, resource) in &section.resources {
                        for entry in &resource.files {
                            let stored = vault_path(&vault_dir(), entry);
                            if stored == Path::new(&path) {
                                total = total
                                    .checked_add(
                                        fs::metadata(&stored).map_err(|_| "库资源读取失败")?.len(),
                                    )
                                    .ok_or("资源大小溢出")?;
                                if total > MAX_BYTES {
                                    return Err("所选资源超过 256 MiB".into());
                                }
                                let bytes = fs::read(stored).map_err(|_| "库资源读取失败")?;
                                if sha256_bytes(&bytes) != entry.hash {
                                    return Err("资源已变化，请刷新资源库".into());
                                }
                                found = Some(file(
                                    entry.source_name.clone(),
                                    if parse_filename(&format!("{canonical}.png")).is_some() {
                                        canonical.clone()
                                    } else {
                                        String::new()
                                    },
                                    &bytes,
                                )?);
                            }
                        }
                    }
                }
            }
            files.push(found.ok_or("所选文件已不在资源库中")?);
        }
        attach(&id, files, false)
    })
    .await
    .map_err(|_| "库资源读取中断")?
}

#[tauri::command]
pub(crate) fn author_image(sha256: String) -> Result<String, String> {
    let path = blob(&sha256)?;
    if fs::metadata(&path).map_err(|_| "图片不存在")?.len() > 2 * 1024 * 1024 {
        return Err("图片超过预览限制".into());
    }
    let bytes = fs::read(path).map_err(|_| "图片读取失败")?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}
