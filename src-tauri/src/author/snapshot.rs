//! 提交预检冻结元数据和文件清单；同一快照保持相同幂等编号，草稿继续编辑不改变上传内容。
use super::*;
use serde_json::json;

pub(super) fn path(id: &str) -> Result<PathBuf, String> {
    checked_id(id)?;
    Ok(root().join("submissions").join(id))
}
pub(super) fn read(id: &str) -> Result<SubmissionSnapshot, String> {
    serde_json::from_slice(
        &fs::read(path(id)?.join("snapshot.json")).map_err(|_| "投稿快照不存在")?,
    )
    .map_err(|_| "投稿快照无效".into())
}
fn validate(draft: &AuthorDraft) -> Result<(), String> {
    let meta = &draft.metadata;
    crate::workshop::attribution::validate(
        &meta.author,
        &meta.artist,
        &meta.author_urls,
        &meta.artist_urls,
        &meta.source_url,
    )?;
    if meta.title.trim().is_empty()
        || meta.title.chars().count() > 120
        || meta.description.trim().is_empty()
        || meta.description.chars().count() > 10000
        || meta.version.trim().is_empty()
        || meta.version.len() > 64
        || meta.author.chars().count() > 120
        || meta.category.chars().count() > 64
        || meta.game_version.len() > 256
        || meta.change_note.len() > 16000
        || meta.tags.len() > 16
        || meta
            .tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.chars().count() > 24)
    {
        return Err("请补全名称、简介、版本，并检查资料长度".into());
    }
    if !draft.metadata_only && (draft.files.is_empty() || draft.files.len() > 512) {
        return Err("请选择 1 至 512 个资源".into());
    }
    if !draft.metadata_only
        && draft
            .files
            .iter()
            .any(|f| !sources::classified(&f.canonical))
    {
        return Err("请先为所有资源完成分类".into());
    }
    if draft.files.iter().map(|f| f.bytes).sum::<u64>() > 256 * 1024 * 1024 {
        return Err("资源展开超过 256 MiB".into());
    }
    if meta
        .tags
        .iter()
        .map(|t| t.to_lowercase())
        .collect::<HashSet<_>>()
        .len()
        != meta.tags.len()
    {
        return Err("标签不能重复".into());
    }
    if draft.images.len() > 7 {
        return Err("最多一张封面和六张附图".into());
    }
    Ok(())
}
fn wire_file(file: &AuthorFile, image: bool) -> serde_json::Value {
    json!({"name":format!("{}.png", file.id),"sourceName":file.source_name,"canonical":if image {""} else {&file.canonical},"sha256":file.sha256,"bytes":file.bytes})
}
pub(super) fn create(draft: AuthorDraft, work: &AuthorWork) -> Result<SubmissionSnapshot, String> {
    validate(&draft)?;
    if draft.metadata_only && work.published.is_none() {
        return Err("作品发布后才可仅更新资料".into());
    }
    let identity =
        serde_json::to_vec(&json!({"draft":draft,"baseRevision":work.revision,"format":2}))
            .map_err(|_| "快照编码失败")?;
    let hash = Sha256::digest(identity);
    let mut id_bytes = [0; 16];
    id_bytes.copy_from_slice(&hash[..16]);
    let id = uuid::Uuid::from_bytes(id_bytes).to_string();
    if path(&id)?.join("snapshot.json").exists() {
        return read(&id);
    }
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    let previous = work
        .published
        .as_ref()
        .map(|p| p.files.as_slice())
        .unwrap_or(&[]);
    if !draft.metadata_only {
        // 同名独立候选逐项匹配；每个已发布候选最多消费一次。
        let mut remaining: Vec<_> = previous.iter().collect();
        for file in &draft.files {
            let exact = remaining.iter().position(|old| {
                old.canonical == file.canonical
                    && old.source() == file.source_name
                    && old.sha256 == file.sha256
            });
            let slot = exact.or_else(|| {
                remaining.iter().position(|old| {
                    old.canonical == file.canonical && old.source() == file.source_name
                })
            });
            match slot {
                Some(index) => {
                    let old = remaining.remove(index);
                    if old.sha256 != file.sha256 {
                        changed.push(file.source_name.clone());
                    }
                }
                None => added.push(file.source_name.clone()),
            }
        }
        removed.extend(remaining.iter().map(|file| file.source().to_string()));
    }
    let dir = path(&id)?;
    ensure_dir(&dir).map_err(|_| "快照目录创建失败")?;
    let archive = dir.join("upload.zip");
    let temp = dir.join("upload.pending");
    let mut zip = zip::ZipWriter::new(fs::File::create(&temp).map_err(|_| "无法创建投稿包")?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("submission.json", options)
        .map_err(|_| "投稿包写入失败")?;
    zip.write_all(&serde_json::to_vec(&json!({"schema":1,"workId":work.id,"submissionId":id,"metadata":draft.metadata,"metadataOnly":draft.metadata_only,"baseRevision":work.revision,
        "files":if draft.metadata_only {vec![]} else {draft.files.iter().map(|file| wire_file(file,false)).collect::<Vec<_>>()},
        "images":draft.images.iter().map(|file| wire_file(file,true)).collect::<Vec<_>>() })).map_err(|_| "投稿资料编码失败")?).map_err(|_| "投稿资料写入失败")?;
    let content = if draft.metadata_only {
        &[][..]
    } else {
        draft.files.as_slice()
    };
    for file in content.iter().chain(&draft.images) {
        let bytes = fs::read(blob(&file.sha256)?).map_err(|_| "草稿资源丢失")?;
        if bytes.len() as u64 != file.bytes || sha256_bytes(&bytes) != file.sha256 {
            return Err("草稿资源已变化，请重新添加".into());
        }
        crate::workshop::validate_png(&bytes)?;
        zip.start_file(format!("{}.png", file.id), options)
            .map_err(|_| "投稿包写入失败")?;
        zip.write_all(&bytes).map_err(|_| "投稿资源写入失败")?;
    }
    zip.finish()
        .map_err(|_| "投稿包写入失败")?
        .sync_all()
        .map_err(|_| "投稿包保存失败")?;
    let size = fs::metadata(&temp).map_err(|_| "投稿包读取失败")?.len();
    if size > 80 * 1024 * 1024 {
        return Err("投稿包超过 80 MiB".into());
    }
    fs::rename(temp, &archive).map_err(|_| "投稿包提交失败")?;
    let metadata_changed = work
        .published
        .as_ref()
        .is_none_or(|old| AuthorMetadata::from(old) != draft.metadata);
    let snapshot = SubmissionSnapshot {
        preview: AuthorPreview {
            id,
            draft,
            bytes: size,
            added,
            changed,
            removed,
            metadata_changed,
        },
        base_revision: work.revision,
        work_id: work.id.clone(),
        archive_hash: sha256_file(&archive)?,
    };
    atomic_json(&dir.join("snapshot.json"), &snapshot)?;
    Ok(snapshot)
}

#[tauri::command]
pub(crate) async fn author_preflight(id: String) -> Result<AuthorPreview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = lock()?;
        let mut state = load()?;
        let draft = draft_mut(&mut state, &id)?;
        validate(draft)?;
        if draft.work_id.is_none() {
            draft.work_id = Some(uuid::Uuid::new_v4().to_string());
            touch(draft);
        }
        let draft = draft.clone();
        let work_id = draft.work_id.as_ref().unwrap();
        if !state.works.iter().any(|w| &w.id == work_id) {
            state.works.push(AuthorWork {
                id: work_id.clone(),
                title: draft.metadata.title.clone(),
                ..Default::default()
            });
        }
        save(&state)?;
        let work = state.works.iter().find(|work| &work.id == work_id).unwrap();
        Ok(create(draft, work)?.preview)
    })
    .await
    .map_err(|_| "投稿预检中断")?
}

/// 导出冻结后的本地投稿包，不要求服务在线；恢复时仍需核对分类和原始摘要。
#[tauri::command]
pub(crate) fn author_export(id: String, destination: String) -> Result<(), String> {
    let snapshot = read(&id)?;
    let source = path(&id)?.join("upload.zip");
    if sha256_file(&source)? != snapshot.archive_hash {
        return Err("投稿快照已损坏".into());
    }
    let mut out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| "目标已存在或无法创建")?;
    std::io::copy(
        &mut fs::File::open(source).map_err(|_| "投稿包读取失败")?,
        &mut out,
    )
    .map_err(|_| "投稿包导出失败")?;
    out.sync_all().map_err(|_| "投稿包保存失败")?;
    let _guard = lock()?;
    let mut state = load()?;
    if !state.receipts.iter().any(|receipt| receipt.id == id) {
        state.receipts.push(AuthorReceipt {
            id,
            work_id: snapshot.work_id,
            draft_id: snapshot.preview.draft.id,
            snapshot_hash: snapshot.archive_hash,
            status: "exported".into(),
            message: "已导出，请通过约定渠道交给维护者。".into(),
            created_at: unix_now(),
        });
    }
    save(&state)
}
