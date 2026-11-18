//! 离线投稿包恢复和公开基线同步；包内编号不代表作者身份或审核通过。
use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Package {
    schema: u32,
    #[serde(default)]
    work_id: Option<String>,
    #[serde(default)]
    submission_id: Option<String>,
    metadata: AuthorMetadata,
    metadata_only: bool,
    base_revision: u64,
    files: Vec<crate::workshop::WorkFile>,
    images: Vec<crate::workshop::WorkFile>,
}

/// 完整校验之后才写草稿；已编辑草稿不能被投稿包的元数据静默替换。
pub(super) fn import(id: &str, path: &Path) -> Result<Option<AuthorDraft>, String> {
    if fs::metadata(path).map_err(|_| "投稿包不存在")?.len() > 80 * 1024 * 1024 {
        return Err("投稿包超过 80 MiB".into());
    }
    let mut zip = zip::ZipArchive::new(fs::File::open(path).map_err(|_| "投稿包读取失败")?)
        .map_err(|_| "ZIP 无效")?;
    if zip.by_name("submission.json").is_err() {
        return Ok(None);
    }
    if zip.len() > 520 {
        return Err("投稿包条目过多".into());
    }
    let mut seen = HashSet::new();
    for index in 0..zip.len() {
        let entry = zip.by_index(index).map_err(|_| "投稿条目无效")?;
        if !entry
            .name()
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            || entry.name().contains("..")
            || entry.name().len() > 128
            || entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            || !seen.insert(entry.name().to_lowercase())
        {
            return Err("投稿包包含非法或重复路径".into());
        }
    }
    let mut bytes = Vec::new();
    zip.by_name("submission.json")
        .map_err(|_| "缺少投稿资料")?
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "投稿资料读取失败")?;
    if bytes.len() > 65536 {
        return Err("投稿资料过长".into());
    }
    let package: Package = serde_json::from_slice(&bytes).map_err(|_| "投稿资料无效")?;
    if package.schema != 1
        || package.files.len() > 512
        || package.images.len() > 7
        || package
            .files
            .iter()
            .any(|file| file.bytes > 256 * 1024 * 1024)
        || package.metadata_only != package.files.is_empty()
        || package.files.iter().map(|file| file.bytes).sum::<u64>() > 256 * 1024 * 1024
    {
        return Err("投稿资源范围无效".into());
    }
    if let Some(work) = &package.work_id {
        checked_id(work)?;
    }
    if let Some(submission) = &package.submission_id {
        checked_id(submission)?;
    }
    let mut payloads = Vec::new();
    let mut declared = HashSet::from(["submission.json".to_string()]);
    for (image, files) in [(false, &package.files), (true, &package.images)] {
        for file in files {
            if !declared.insert(file.name.to_lowercase())
                || !crate::workshop::safe_source(file.source())
                || (!image && !sources::classified(&file.canonical))
                || (image && (!file.canonical.is_empty() || file.bytes > 2 * 1024 * 1024))
            {
                return Err("投稿资源声明无效".into());
            }
            let entry = zip.by_name(&file.name).map_err(|_| "投稿资源缺失")?;
            if entry.size() != file.bytes {
                return Err("投稿资源长度不符".into());
            }
            let mut data = Vec::new();
            entry
                .take(file.bytes + 1)
                .read_to_end(&mut data)
                .map_err(|_| "投稿资源读取失败")?;
            if data.len() as u64 != file.bytes || sha256_bytes(&data) != file.sha256 {
                return Err("投稿资源摘要不符".into());
            }
            crate::workshop::validate_png(&data)?;
            payloads.push((image, file, data));
        }
    }
    if seen != declared {
        return Err("投稿包含未声明文件".into());
    }
    let _guard = lock()?;
    let mut state = load()?;
    let draft = draft_mut(&mut state, id)?;
    if !draft.files.is_empty()
        || !draft.images.is_empty()
        || !draft.metadata.title.is_empty()
        || draft.work_id.is_some()
    {
        return Err("请在空白新草稿中恢复投稿包".into());
    }
    for (image, file, bytes) in payloads {
        let restored = sources::file(file.source().to_string(), file.canonical.clone(), &bytes)?;
        if image {
            draft.images.push(restored);
        } else {
            draft.files.push(restored);
        }
    }
    draft.metadata = package.metadata;
    draft.metadata_only = package.metadata_only;
    draft.work_id = package.work_id;
    touch(draft);
    let result = draft.clone();
    if let Some(work_id) = &result.work_id {
        if !state.works.iter().any(|work| &work.id == work_id) {
            // 修订号必须从受信签名目录同步，不能信任离线包自报的基线。
            state.works.push(AuthorWork {
                id: work_id.clone(),
                title: result.metadata.title.clone(),
                ..Default::default()
            });
        }
    }
    let _untrusted_revision = package.base_revision;
    save(&state)?;
    Ok(Some(result))
}

/// 只同步签名静态目录中可确认的已发布版本，人工审核状态仍由收稿渠道通知。
#[tauri::command]
pub(crate) async fn author_sync_published(id: String) -> Result<AuthorState, String> {
    checked_id(&id)?;
    let listing = crate::workshop::workshop_list().await?;
    if !listing.available {
        return Err(listing.message);
    }
    let published = listing
        .works
        .into_iter()
        .find(|item| item.work.id == id)
        .ok_or("公开目录尚未收录此作品，请以维护者通知为准")?
        .work;
    let _guard = lock()?;
    let mut state = load()?;
    let work = state
        .works
        .iter_mut()
        .find(|work| work.id == id)
        .ok_or("本地作品不存在")?;
    work.revision = published.revision;
    work.title = published.title.clone();
    work.published = Some(published);
    save(&state)?;
    Ok(state)
}
