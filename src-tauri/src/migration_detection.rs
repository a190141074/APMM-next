//! 只读识别就地接续与相邻旧安装；不凭迁移标记推断候选齐全，不自动写入用户数据。
use crate::*;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct MigrationSource {
    pub source: String,
    pub version: Option<String>,
    pub mods: usize,
    pub new_mods: usize,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub(crate) struct MigrationStatus {
    pub mode: String,
    pub destination: String,
    pub mods: usize,
    pub multiple_slots: usize,
    pub sources: Vec<MigrationSource>,
}

fn read_library(root: &Path) -> Option<Library> {
    serde_json::from_slice(&fs::read(root.join("library.json")).ok()?).ok()
}

fn counts(lib: &Library) -> (usize, usize) {
    let resources = [&lib.heroes, &lib.monsters]
        .into_iter()
        .flat_map(|map| map.values())
        .flat_map(|entity| entity.sections.values())
        .flat_map(|section| section.resources.values());
    resources.fold((0, 0), |(mods, multiple), r| {
        (
            mods + r.files.len(),
            multiple + usize::from(r.files.len() > 1),
        )
    })
}

/// 两端都按真实路径比较，Windows 大小写和扩展路径前缀不能让同一 data 被当成另一份库。
pub(super) fn same_path(a: &Path, b: &Path) -> bool {
    let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let a = real(a);
    let b = real(b);
    if cfg!(windows) {
        a.to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/")
            .trim_end_matches('/')
            .eq_ignore_ascii_case(
                b.to_string_lossy()
                    .trim_start_matches(r"\\?\")
                    .replace('\\', "/")
                    .trim_end_matches('/'),
            )
    } else {
        a == b
    }
}

fn installed_data(directory: &Path) -> Option<PathBuf> {
    let legacy = directory.join("astral-mod-manager.exe").is_file()
        || directory.join("astral-updater.exe").is_file()
        || directory.join("release-manifest.json").is_file();
    legacy
        .then(|| directory.join("data"))
        .filter(|p| p.join("library.json").is_file())
}

/// 只查当前安装附近两层的直接子目录和迁移记录来源，不遍历磁盘或读取游戏资源。
fn detect(destination: &Path, installation: &Path) -> MigrationStatus {
    let current = read_library(destination).unwrap_or_default();
    let (mods, multiple_slots) = counts(&current);
    let marker = fs::read(destination.join("migration.json"))
        .ok()
        .and_then(|raw| serde_json::from_slice::<serde_json::Value>(&raw).ok());
    let mut candidates = Vec::new();
    if let Some(source) = marker.as_ref().and_then(|v| v["source"].as_str()) {
        candidates.push(PathBuf::from(source));
    }
    let mut roots = Vec::new();
    for start in [installation, destination.parent().unwrap_or(destination)] {
        for root in start.ancestors().skip(1).take(2) {
            if !roots.iter().any(|p: &PathBuf| same_path(p, root)) {
                roots.push(root.to_path_buf());
            }
        }
    }
    for root in roots {
        if let Some(data) = installed_data(&root) {
            candidates.push(data);
        }
        if let Ok(children) = fs::read_dir(root) {
            for child in children.take(128).flatten() {
                if let Some(data) = installed_data(&child.path()) {
                    candidates.push(data);
                }
            }
        }
    }
    let mut sources: Vec<MigrationSource> = Vec::new();
    for path in candidates {
        if same_path(&path, destination)
            || sources
                .iter()
                .any(|s| same_path(Path::new(&s.source), &path))
        {
            continue;
        }
        let Some(old) = read_library(&path) else {
            continue;
        };
        let version = path
            .parent()
            .and_then(|p| fs::read(p.join("release-manifest.json")).ok())
            .and_then(|raw| serde_json::from_slice::<serde_json::Value>(&raw).ok())
            .and_then(|v| v["version"].as_str().map(str::to_string));
        sources.push(MigrationSource {
            source: path.to_string_lossy().into_owned(),
            version,
            mods: counts(&old).0,
            new_mods: crate::migration_merge::missing_count(&old, &current),
        });
    }
    sources.sort_by(|a, b| b.new_mods.cmp(&a.new_mods).then(a.source.cmp(&b.source)));
    MigrationStatus {
        mode: if marker.is_some() {
            "migrated"
        } else if mods > 0 {
            "inherited"
        } else {
            "fresh"
        }
        .into(),
        destination: destination.to_string_lossy().into_owned(),
        mods,
        multiple_slots,
        sources,
    }
}

#[tauri::command]
pub(crate) async fn migration_status() -> Result<MigrationStatus, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let installation = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_default();
        Ok(detect(&data_dir(), &installation))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "migration_detection_tests.rs"]
mod tests;
