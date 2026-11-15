//! APX 按平台保存目录；探测只返回候选，确认后仅修改所选平台的单个路径。
use crate::{
    configured_plugin_dir, load_library, normalize_windows_path, plugin_platform_info,
    save_library, AppState, Settings,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct PluginDirectories {
    pub(crate) locallow: Option<String>,
    pub(crate) data: Option<String>,
    pub(crate) root: Option<String>,
}

/// 旧版的全局路径只迁给原来选中的渠道，不覆盖已经保存的渠道配置。
pub(crate) fn migrate_plugin_directories(settings: &mut Settings) {
    let platform = settings
        .plugin_platform
        .as_deref()
        .filter(|key| plugin_platform_info(key).is_some())
        .unwrap_or(crate::PLUGIN_DEFAULT_PLATFORM)
        .to_string();
    let legacy = PluginDirectories {
        locallow: settings.plugin_locallow_dir.take(),
        data: settings.plugin_data_dir.take(),
        root: settings.plugin_root_dir.take(),
    };
    if legacy.locallow.is_none() && legacy.data.is_none() && legacy.root.is_none() {
        return;
    }
    let paths = settings.plugin_directories.entry(platform).or_default();
    if paths.locallow.is_none() {
        paths.locallow = legacy.locallow;
    }
    if paths.data.is_none() {
        paths.data = legacy.data;
    }
    if paths.root.is_none() {
        paths.root = legacy.root;
    }
}

/// 单个路径变更不能连带改写同平台其他路径或其他平台配置。
pub(crate) fn set_directory(
    settings: &mut Settings,
    platform: &str,
    which: &str,
    path: String,
) -> Result<(), String> {
    validate_request(platform, which)?;
    let value = Some(normalize_windows_path(&path).to_string_lossy().into_owned());
    let paths = settings
        .plugin_directories
        .entry(platform.into())
        .or_default();
    match which {
        "locallow" => paths.locallow = value,
        "data" => paths.data = value,
        "root" => paths.root = value,
        _ => unreachable!(),
    }
    Ok(())
}

fn validate_request(platform: &str, which: &str) -> Result<(), String> {
    if plugin_platform_info(platform).is_none() {
        return Err("未知的平台".into());
    }
    if platform == "CN_ANDROID" {
        return Err("Android 不设置本机游戏目录".into());
    }
    if !matches!(which, "locallow" | "data" | "root") {
        return Err("未知的目录类型".into());
    }
    Ok(())
}

pub(crate) fn configured_directory(
    settings: &Settings,
    platform: &str,
    which: &str,
) -> Option<PathBuf> {
    let paths = settings.plugin_directories.get(platform)?;
    configured_plugin_dir(match which {
        "locallow" => paths.locallow.clone(),
        "data" => paths.data.clone(),
        "root" => paths.root.clone(),
        _ => None,
    })
}

/// 游戏根目录只读取当前平台配置或该平台唯一匹配的安装，不借用其他平台的 modimg。
pub(crate) fn platform_game_root(settings: &Settings, platform: &str) -> Option<PathBuf> {
    configured_directory(settings, platform, "root")
        .or_else(|| {
            configured_directory(settings, platform, "data")
                .and_then(|data| data.parent().map(Path::to_path_buf))
        })
        .or_else(|| {
            let roots = detected_game_roots(settings, platform);
            (roots.len() == 1).then(|| roots[0].clone())
        })
}

/// 同名 Data 目录不能区分 TapTap、哔哩哔哩和 WeGame，只接受它们各自配置的根目录。
fn detected_game_roots(settings: &Settings, platform: &str) -> Vec<PathBuf> {
    let Some((_, _, _, data_name)) = plugin_platform_info(platform) else {
        return Vec::new();
    };
    if data_name.is_empty() {
        return Vec::new();
    }
    let mut roots = Vec::new();
    let mut add = |root: PathBuf| {
        if root.join(data_name).is_dir() && !roots.contains(&root) {
            roots.push(root);
        }
    };
    if let Some(root) = configured_directory(settings, platform, "root") {
        add(root);
    }
    if let Some(data) = configured_directory(settings, platform, "data") {
        if let Some(root) = data.parent() {
            add(root.into());
        }
    }
    if matches!(platform, "CN_STEAM" | "INT_STEAM") {
        if let Some(root) = Path::new(&settings.modimg_path).parent() {
            add(root.into());
        }
        for library in crate::game::steam_libraries() {
            let install = library.join("common").join("Astral Party");
            add(install.clone());
            if let Ok(entries) = std::fs::read_dir(install) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        add(entry.path());
                    }
                }
            }
        }
    }
    roots.sort();
    roots
}

/// 返回当前平台的存在目录，多个安装保留为候选，不选择首项，也不创建游戏目录。
pub(crate) fn directory_candidates(
    settings: &Settings,
    platform: &str,
    which: &str,
    locallow_base: Option<PathBuf>,
) -> Result<Vec<String>, String> {
    validate_request(platform, which)?;
    let (_, _, locallow_name, data_name) = plugin_platform_info(platform).unwrap();
    let mut candidates = Vec::new();
    if let Some(path) = configured_directory(settings, platform, which) {
        candidates.push(path);
    }
    if which == "locallow" {
        if let Some(base) = locallow_base {
            candidates.push(base.join("feimo").join(locallow_name));
        }
    } else {
        for root in detected_game_roots(settings, platform) {
            candidates.push(if which == "data" {
                root.join(data_name)
            } else {
                root
            });
        }
    }
    candidates.retain(|p| p.is_dir());
    candidates.sort();
    candidates.dedup();
    Ok(candidates
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn plugin_match_directory(
    platform: String,
    which: String,
) -> Result<Vec<String>, String> {
    // 扫描在阻塞线程执行；读取候选不写入配置、不改变工作区游戏路径。
    tauri::async_runtime::spawn_blocking(move || {
        directory_candidates(
            &load_library().settings,
            &platform,
            &which,
            crate::locallow_root(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn plugin_set_directory(
    platform: String,
    which: String,
    path: String,
    state: tauri::State<AppState>,
) -> Result<PluginDirectories, String> {
    validate_request(&platform, &which)?;
    let target = normalize_windows_path(&path);
    if target.as_os_str().is_empty() || !target.is_absolute() || !target.is_dir() {
        return Err("请选择存在的绝对目录路径".into());
    }
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    let mut lib = load_library();
    set_directory(&mut lib.settings, &platform, &which, path)?;
    save_library(&lib)?;
    Ok(lib
        .settings
        .plugin_directories
        .get(&platform)
        .cloned()
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn apx_directories_migrate_only_original_platform_and_roundtrip() {
        let mut settings: Settings = serde_json::from_value(serde_json::json!({"pluginPlatform":"CN_TAPTAP", "pluginRootDir":"B:/tap", "pluginLocallowDir":"B:/tap-cache"})).unwrap();
        migrate_plugin_directories(&mut settings);
        assert_eq!(settings.plugin_directories.len(), 1);
        assert_eq!(
            settings.plugin_directories["CN_TAPTAP"].root.as_deref(),
            Some("B:/tap")
        );
        assert!(configured_directory(&settings, "CN_STEAM", "root").is_none());
        assert!(settings.plugin_root_dir.is_none());
        let mut restored: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        migrate_plugin_directories(&mut restored);
        assert_eq!(restored.plugin_directories.len(), 1);

        let mut existing: Settings = serde_json::from_value(serde_json::json!({
            "pluginPlatform": "CN_TAPTAP",
            "pluginRootDir": "B:/legacy-tap",
            "pluginLocallowDir": "B:/legacy-cache",
            "pluginDirectories": {
                "CN_TAPTAP": {"root": "B:/current-tap"},
                "CN_STEAM": {"root": "B:/steam"}
            }
        }))
        .unwrap();
        migrate_plugin_directories(&mut existing);
        assert_eq!(existing.plugin_directories.len(), 2);
        assert_eq!(
            existing.plugin_directories["CN_TAPTAP"].root.as_deref(),
            Some("B:/current-tap")
        );
        assert_eq!(
            existing.plugin_directories["CN_TAPTAP"].locallow.as_deref(),
            Some("B:/legacy-cache")
        );
        assert_eq!(
            existing.plugin_directories["CN_STEAM"].root.as_deref(),
            Some("B:/steam")
        );
        assert!(existing.plugin_root_dir.is_none());
    }
    #[test]
    fn apx_directories_changes_preserve_other_fields_and_platforms() {
        let mut settings = Settings::default();
        set_directory(&mut settings, "CN_STEAM", "root", "B:/steam".into()).unwrap();
        set_directory(&mut settings, "CN_TAPTAP", "root", "B:/tap".into()).unwrap();
        set_directory(&mut settings, "CN_STEAM", "locallow", "B:/cache".into()).unwrap();
        assert_eq!(
            settings.plugin_directories["CN_TAPTAP"].root.as_deref(),
            Some("B:/tap")
        );
        assert_eq!(
            settings.plugin_directories["CN_STEAM"].root.as_deref(),
            Some("B:/steam")
        );
        assert!(settings.plugin_directories["CN_STEAM"].data.is_none());
        assert!(set_directory(&mut settings, "BAD", "root", "B:/bad".into()).is_err());
        assert!(set_directory(&mut settings, "CN_ANDROID", "root", "B:/bad".into()).is_err());
        assert!(set_directory(&mut settings, "CN_STEAM", "bad", "B:/bad".into()).is_err());
        assert_eq!(settings.plugin_directories.len(), 2);
    }
    #[test]
    fn apx_directories_detection_keeps_channels_separate_and_does_not_write() {
        let root =
            crate::staging::work_root().join(format!("apx-path-test-{}", uuid::Uuid::new_v4()));
        let tap = root.join("tap");
        let bili = root.join("bili");
        std::fs::create_dir_all(tap.join("吉星派对_Data")).unwrap();
        std::fs::create_dir_all(bili.join("吉星派对_Data")).unwrap();
        let mut settings = Settings::default();
        set_directory(
            &mut settings,
            "CN_TAPTAP",
            "root",
            tap.to_string_lossy().into(),
        )
        .unwrap();
        set_directory(
            &mut settings,
            "CN_BILIBILI",
            "root",
            bili.to_string_lossy().into(),
        )
        .unwrap();
        let before = serde_json::to_string(&settings).unwrap();
        let paths = directory_candidates(&settings, "CN_TAPTAP", "data", None).unwrap();
        assert_eq!(
            paths,
            vec![tap.join("吉星派对_Data").to_string_lossy().into_owned()]
        );
        assert!(directory_candidates(&settings, "CN_WEGAME", "data", None)
            .unwrap()
            .is_empty());
        assert_eq!(before, serde_json::to_string(&settings).unwrap());
        assert_eq!(platform_game_root(&settings, "CN_BILIBILI"), Some(bili));
    }
}
