//! 预设命令保持旧入口兼容，状态和应用事务由预设领域处理。
use crate::management_types::*;
use crate::preset_domain::*;
use crate::*;

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preset_create(
    name: String,
    scope: String,
    selected: Vec<ResourceSlot>,
    state: tauri::State<AppState>,
) -> Result<GlobalPreset, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    let game = PathBuf::from(&lib.settings.modimg_path);
    crate::directory_sync::synchronize(&mut lib, &vault_dir(), &game)?;
    let p = capture(&lib, scope, selected, name)?;
    lib.settings.presets.push(p.clone());
    save_library(&lib)?;
    Ok(p)
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preset_save(
    name: String,
    state: tauri::State<AppState>,
) -> Result<GlobalPreset, String> {
    preset_create(name, "global".into(), vec![], state)
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn preset_detail(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PresetPreview, String> {
    let mutation = state.mutation.clone();
    // 预设页可以在目录同步期间打开，读取时等待库锁不能阻塞窗口消息。
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
        let lib = load_library();
        Ok(preview(&lib, find(&lib, &id)?))
    })
    .await
    .map_err(|error| format!("读取预设失败：{error}"))?
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preset_update(
    id: String,
    name: String,
    items: Vec<PresetItem>,
    state: tauri::State<AppState>,
) -> Result<GlobalPreset, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    editable(find(&lib, &id)?)?;
    let p = lib
        .settings
        .presets
        .iter_mut()
        .find(|p| p.id == id)
        .unwrap();
    p.name = name_checked(name)?;
    let mut seen = HashSet::new();
    if items.iter().any(|i| !seen.insert(i.canonical.clone())) {
        return Err("预设存在重复槽位".into());
    }
    p.items = items;
    let result = p.clone();
    save_library(&lib)?;
    Ok(result)
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preset_copy(
    id: String,
    state: tauri::State<AppState>,
) -> Result<GlobalPreset, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    let mut p = find(&lib, &id)?.clone();
    p.id = uuid::Uuid::new_v4().to_string();
    p.name = format!("{} 副本", p.name);
    p.locked = false;
    lib.settings.presets.push(p.clone());
    save_library(&lib)?;
    Ok(p)
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preset_lock(
    id: String,
    locked: bool,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    find(&lib, &id)?;
    lib.settings
        .presets
        .iter_mut()
        .find(|p| p.id == id)
        .unwrap()
        .locked = locked;
    save_library(&lib)
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preset_delete(id: String, state: tauri::State<AppState>) -> Result<(), String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    editable(find(&lib, &id)?)?;
    lib.settings.presets.retain(|p| p.id != id);
    save_library(&lib)
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preset_apply(id: String, state: tauri::State<AppState>) -> Result<(), String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    let p = find(&lib, &id)?.clone();
    apply(&mut lib, &p)
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn preset_restore_all(state: tauri::State<AppState>) -> Result<(), String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    apply(
        &mut lib,
        &GlobalPreset {
            scope: "global".into(),
            ..Default::default()
        },
    )
}
