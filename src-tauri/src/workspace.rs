//! 工作台查询与成组修改适配；复用旧库和应用事务，不维护第二份 Mod 状态。
use crate::dto::*;
use crate::*;
use std::collections::BTreeMap;

pub(crate) fn snapshot() -> WorkspaceSnapshot {
    snapshot_from_library(&load_library())
}
pub(crate) fn snapshot_from_library(lib: &Library) -> WorkspaceSnapshot {
    let vd = vault_dir();
    let mut rows: BTreeMap<String, ResourceRecord> = BTreeMap::new();
    let mut originals = crate::workspace_catalog::entries().clone();
    originals.extend(crate::originals::external_names_cached(&lib.settings));
    if let Ok(entries) = fs::read_dir(data_dir().join("assets/maps")) {
        let mut paths = entries.flatten().map(|e| e.path()).collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            if let Ok(bytes) = fs::read(path) {
                if let Ok(local) = serde_json::from_slice::<Vec<serde_json::Value>>(&bytes) {
                    originals.extend(local);
                }
            }
        }
    }
    for value in originals {
        let Some(cn) = value["canonical"].as_str() else {
            continue;
        };
        let Some(parsed) = parse_filename(cn) else {
            continue;
        };
        let mut row =
            crate::workspace_catalog::record(&lib, &parsed.section_key, cn, &parsed.hero_id);
        let (path, status) = crate::originals::resolve_cached(cn, &value, &lib.settings);
        if path.is_some() || !rows.contains_key(&row.key) {
            row.original_url = path;
            row.original_status = status;
            rows.insert(row.key.clone(), row);
        }
    }
    for map in [&lib.heroes, &lib.monsters] {
        for (id, entity) in map {
            for (section, sec) in &entity.sections {
                for (cn, res) in &sec.resources {
                    let key = format!("{section}/{cn}");
                    let entry = rows.entry(key).or_insert_with(|| {
                        let mut row = crate::workspace_catalog::record(&lib, section, cn, id);
                        let (path, status) = crate::originals::resolve_cached(
                            cn,
                            &serde_json::json!({}),
                            &lib.settings,
                        );
                        row.original_url = path;
                        row.original_status = status;
                        row
                    });
                    entry.active_hash = res.active_hash.clone();
                    entry.disabled = res.disabled;
                    entry.files = res
                        .files
                        .iter()
                        .map(|f| ModFile {
                            hash: f.hash.clone(),
                            source_name: f.source_name.clone(),
                            path: vault_path(&vd, f).to_string_lossy().into_owned(),
                            info: Some(f.info.clone()),
                            classification: Some(f.classification.clone()),
                        })
                        .collect();
                }
            }
        }
    }
    let outimg_roots = outimg_roots(&lib.settings);
    for row in rows.values_mut() {
        row.outimg_path = locate_outimg(&outimg_roots, &row.canonical, row.original_url.as_deref())
            .map(|path| path.to_string_lossy().into_owned());
    }
    WorkspaceSnapshot {
        revision: 0,
        version: env!("CARGO_PKG_VERSION").into(),
        data_dir: data_dir().to_string_lossy().into_owned(),
        game_dir: Path::new(&lib.settings.modimg_path)
            .parent()
            .filter(|p| p.is_dir())
            .map(|p| p.to_string_lossy().into_owned()),
        resources: rows.into_values().collect(),
        settings: WorkspaceSettings::from(&lib.settings),
        migrated: data_dir().join("migration.json").is_file(),
    }
}
#[tauri::command]
pub(crate) async fn workspace_snapshot(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<WorkspaceSnapshot, String> {
    let mutation = state.mutation.clone();
    // 目录同步可能长时间持锁；锁等待、原图校验和快照组装均不能占用窗口线程。
    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let value = crate::workspace_cache::get(&app, &mutation, false)?;
        if started.elapsed() >= Duration::from_millis(500) {
            let detail = serde_json::json!({
                "totalMs": started.elapsed().as_millis(),
                "resources": value.resources.len(),
            })
            .to_string();
            write_log(
                "info",
                "workspace.snapshot",
                "工作区快照耗时",
                Some(&detail),
            );
        }
        Ok(value)
    })
    .await
    .map_err(|error| format!("读取工作区失败：{error}"))?
}

/// 同一批操作先备份涉及的游戏文件，任何一步失败都恢复文件及库状态。
#[tauri::command]
pub(crate) async fn resources_apply(
    items: Vec<ResourceAction>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mutation = state.mutation.clone();
    // 同步提交期间的锁等待和应用前的内容校验都不能占用窗口线程。
    tauri::async_runtime::spawn_blocking(move || apply_items(items, &mutation))
        .await
        .map_err(|e| format!("应用资源失败：{e}"))?
}

pub(super) fn apply_items(items: Vec<ResourceAction>, mutation: &Mutex<()>) -> Result<(), String> {
    let _lock = mutation.lock().map_err(|_| "库写入锁不可用")?;
    let mut lib = load_library();
    if lib.settings.modimg_path.is_empty() {
        return Err("请先在设置中选择游戏目录".into());
    }
    if items.is_empty() {
        return Err("请选择资源".into());
    }
    let modimg = PathBuf::from(&lib.settings.modimg_path);
    let vault = vault_dir().canonicalize().map_err(|e| e.to_string())?;
    let mut prepared = Vec::new();
    for item in &items {
        let parsed = parse_filename(&item.canonical).ok_or("不支持的应用槽位")?;
        if parsed.hero_id != item.entity_id || parsed.section_key != item.section {
            return Err("资源身份与应用槽位不匹配".into());
        }
        let source = if let Some(hash) = &item.hash {
            let resource = lib_map(&lib, &category_of(&item.section))
                .get(&item.entity_id)
                .and_then(|h| h.sections.get(&item.section))
                .and_then(|s| s.resources.get(&item.canonical))
                .ok_or("资源不存在")?;
            if resource.disabled {
                return Err("请先启用被禁用的资源".into());
            }
            let entry = resource
                .files
                .iter()
                .find(|f| &f.hash == hash)
                .ok_or("所选 Mod 不存在")?;
            let path = vault_path(&vault, entry)
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !path.starts_with(&vault) || sha256_file(&path)? != *hash {
                return Err("Mod 文件路径或内容校验失败".into());
            }
            Some(path)
        } else {
            None
        };
        let resource = get_resource(
            &mut lib,
            &category_of(&item.section),
            &item.entity_id,
            &item.section,
            &item.canonical,
        );
        resource.active_hash = item.hash.clone();
        resource.disabled = false;
        prepared.push((item.canonical.clone(), source));
    }
    crate::resource_transaction::apply_batch(&modimg, &prepared, || save_library(&lib))
}
