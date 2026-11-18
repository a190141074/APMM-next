//! 安装确认绑定目录和本地文件快照，界面不能直接构造应用目标。
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct InstallEntry {
    pub name: String,
    pub source_name: String,
    pub canonical: String,
    pub sha256: String,
    pub current_hash: Option<String>,
    pub duplicate: bool,
    pub disabled: bool,
    pub game_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct InstallPlan {
    pub work: Work,
    pub work_fingerprint: String,
    pub local_fingerprint: String,
    pub can_apply: bool,
    pub entries: Vec<InstallEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct InstallRequest {
    pub id: String,
    pub work_fingerprint: String,
    pub local_fingerprint: String,
    pub apply_files: Vec<String>,
}

pub(super) fn fingerprint(work: &Work) -> Result<String, String> {
    let mut canonical = work.clone();
    canonical.cover_data_url = None;
    for file in &mut canonical.files {
        file.resource = None;
    }
    serde_json::to_vec(&canonical)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|e| e.to_string())
}

pub(super) fn build(work: Work, lib: &Library) -> Result<InstallPlan, String> {
    let mut entries = Vec::new();
    let mut local = Vec::new();
    for file in &work.files {
        let parsed = file.parsed()?;
        let resource = lib_map(lib, &category_of(&parsed.section_key))
            .get(&parsed.hero_id)
            .and_then(|hero| hero.sections.get(&parsed.section_key))
            .and_then(|section| section.resources.get(&parsed.canonical));
        let mut game_hashes = Vec::new();
        if !lib.settings.modimg_path.is_empty() {
            for path in
                canonical_modimg_files(Path::new(&lib.settings.modimg_path), &parsed.canonical)?
            {
                game_hashes.push(format!(
                    "{}:{}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    sha256_file(&path)?
                ));
            }
        }
        game_hashes.sort();
        let mut stored = Vec::new();
        if let Some(resource) = resource {
            for item in &resource.files {
                stored.push(format!(
                    "{}:{}:{}",
                    item.rel_path,
                    item.hash,
                    sha256_file(&vault_path(&vault_dir(), item))
                        .unwrap_or_else(|_| "missing".into())
                ));
            }
        }
        stored.sort();
        local.push((parsed.canonical.clone(), stored));
        entries.push(InstallEntry {
            name: file.name.clone(),
            source_name: file.source().into(),
            canonical: parsed.canonical,
            sha256: file.sha256.clone(),
            current_hash: resource.and_then(|r| r.active_hash.clone()),
            duplicate: resource.is_some_and(|r| r.files.iter().any(|f| f.hash == file.sha256)),
            disabled: resource.is_some_and(|r| r.disabled),
            game_hashes,
        });
    }
    local.sort();
    let encoded = serde_json::to_vec(&(
        &entries,
        &local,
        &lib.settings.modimg_path,
        lib.workshop.get(&work.id),
    ))
    .map_err(|e| e.to_string())?;
    Ok(InstallPlan {
        work_fingerprint: fingerprint(&work)?,
        work,
        entries,
        local_fingerprint: sha256_bytes(&encoded),
        can_apply: !lib.settings.modimg_path.is_empty(),
    })
}

pub(super) fn validate(request: &InstallRequest, plan: &InstallPlan) -> Result<(), String> {
    if request.work_fingerprint != plan.work_fingerprint
        || request.local_fingerprint != plan.local_fingerprint
    {
        return Err("作品或本地资源已变化，请重新检查并确认".into());
    }
    if !request.apply_files.is_empty() && !plan.can_apply {
        return Err("请先配置游戏目录".into());
    }
    let mut slots = HashSet::new();
    for name in &request.apply_files {
        let entry = plan
            .entries
            .iter()
            .find(|file| &file.name == name)
            .ok_or("应用项不在确认清单中")?;
        if entry.disabled || !slots.insert(entry.canonical.clone()) {
            return Err("同一槽位只能应用一个已启用候选".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn workshop_preflight(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<InstallPlan, String> {
    let mutation = Arc::clone(&state.mutation);
    tauri::async_runtime::spawn_blocking(move || {
        let base = base_url().ok_or("工坊未配置")?;
        let work = catalog(&base)?
            .into_iter()
            .find(|work| work.id == id)
            .ok_or("作品已下架或不存在")?;
        let _guard = mutation.lock().map_err(|_| "库锁不可用")?;
        build(work, &load_library())
    })
    .await
    .map_err(|_| "安装预检中断")?
}
