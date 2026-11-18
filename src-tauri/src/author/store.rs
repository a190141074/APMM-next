//! 草稿乐观并发控制和原子保存；后台回执不覆盖正在编辑的草稿。
use super::*;
static LOCK: Mutex<()> = Mutex::new(());
pub(super) fn lock() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    LOCK.lock().map_err(|_| "草稿锁不可用".into())
}
pub(super) fn root() -> PathBuf {
    data_dir().join("author")
}
pub(super) fn load() -> Result<AuthorState, String> {
    let path = root().join("state.json");
    if !path.exists() {
        return Ok(AuthorState::default());
    }
    serde_json::from_slice(&fs::read(path).map_err(|_| "无法读取作者记录")?)
        .map_err(|_| "作者记录损坏，请保留文件并恢复备份".into())
}
pub(super) fn save(state: &AuthorState) -> Result<(), String> {
    atomic_json(&root().join("state.json"), state)
}
pub(super) fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    ensure_dir(path.parent().ok_or("记录路径无效")?).map_err(|e| e.to_string())?;
    let tmp = path.with_extension(format!("{}.pending", uuid::Uuid::new_v4()));
    let data = serde_json::to_vec(value).map_err(|_| "记录编码失败")?;
    let mut file = fs::File::create(&tmp).map_err(|_| "记录暂存失败")?;
    file.write_all(&data)
        .and_then(|_| file.sync_all())
        .map_err(|_| "记录写入失败")?;
    drop(file);
    fs::rename(&tmp, path).map_err(|_| "记录提交失败".into())
}
pub(super) fn blob(hash: &str) -> Result<PathBuf, String> {
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("资源摘要无效".into());
    }
    Ok(root().join("blobs").join(format!("{hash}.png")))
}
pub(super) fn put_blob(bytes: &[u8]) -> Result<String, String> {
    let hash = sha256_bytes(bytes);
    let path = blob(&hash)?;
    ensure_dir(path.parent().unwrap()).map_err(|_| "无法创建草稿资源目录")?;
    if !path.exists() {
        fs::write(&path, bytes).map_err(|_| "无法保存草稿资源")?;
    }
    if sha256_file(&path)? != hash {
        return Err("草稿资源摘要不符".into());
    }
    Ok(hash)
}
pub(super) fn draft_mut<'a>(
    state: &'a mut AuthorState,
    id: &str,
) -> Result<&'a mut AuthorDraft, String> {
    state
        .drafts
        .iter_mut()
        .find(|draft| draft.id == id)
        .ok_or_else(|| "草稿不存在".into())
}
pub(super) fn touch(draft: &mut AuthorDraft) {
    draft.revision += 1;
    draft.updated_at = unix_now();
}
#[tauri::command]
pub(crate) fn author_state() -> Result<AuthorState, String> {
    let _guard = lock()?;
    load()
}

#[tauri::command]
pub(crate) fn author_new(work_id: Option<String>) -> Result<AuthorDraft, String> {
    let _guard = lock()?;
    let mut state = load()?;
    let mut draft = AuthorDraft {
        id: uuid::Uuid::new_v4().to_string(),
        updated_at: unix_now(),
        ..Default::default()
    };
    if let Some(id) = work_id {
        let work = state
            .works
            .iter()
            .find(|work| work.id == id)
            .ok_or("请先导入该作品的投稿包")?;
        draft.work_id = Some(id);
        if let Some(published) = &work.published {
            draft.metadata = AuthorMetadata::from(published);
            draft.metadata.change_note.clear();
            draft.metadata_only = true;
        }
    }
    state.drafts.push(draft.clone());
    save(&state)?;
    Ok(draft)
}

/// 只接受元数据、已存在文件的分类及保留清单；文件摘要和路径不能由界面改写。
#[tauri::command]
pub(crate) fn author_save(draft: AuthorDraft) -> Result<AuthorDraft, String> {
    let _guard = lock()?;
    let mut state = load()?;
    let current = draft_mut(&mut state, &draft.id)?;
    if draft.revision != current.revision {
        return Err("草稿已更新，请重新载入".into());
    }
    if serde_json::to_vec(&draft.metadata)
        .map_err(|_| "资料无效")?
        .len()
        > 32 * 1024
    {
        return Err("作品资料过长".into());
    }
    for file in &draft.files {
        if !current.files.iter().any(|f| f.id == file.id) {
            return Err("草稿文件不存在".into());
        }
        if !file.canonical.is_empty() && !sources::classified(&file.canonical) {
            return Err("资源分类无效".into());
        }
    }
    current.metadata = draft.metadata;
    current.metadata_only = draft.metadata_only;
    current
        .files
        .retain(|f| draft.files.iter().any(|next| next.id == f.id));
    for file in &mut current.files {
        file.canonical = draft
            .files
            .iter()
            .find(|f| f.id == file.id)
            .unwrap()
            .canonical
            .clone();
    }
    let mut images = Vec::new();
    for image in draft.images {
        images.push(
            current
                .images
                .iter()
                .find(|f| f.id == image.id)
                .ok_or("预览图不存在")?
                .clone(),
        );
    }
    if images.iter().map(|f| &f.id).collect::<HashSet<_>>().len() != images.len() {
        return Err("预览图不能重复引用".into());
    }
    if images.len() > 7 {
        return Err("最多一张封面和六张附图".into());
    }
    current.images = images;
    touch(current);
    let result = current.clone();
    save(&state)?;
    Ok(result)
}
