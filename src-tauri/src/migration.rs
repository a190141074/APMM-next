//! 旧库先复制并验证，再事务提交；源目录和游戏目录不会被写入。
use crate::*;
use serde_json::{json, Value};
use tauri::Manager;

fn walk(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            let mut linked = meta.file_type().is_symlink();
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                linked |= meta.file_attributes() & 0x400 != 0;
            }
            if linked {
                return Err(format!(
                    "迁移目录含链接，请选择真实数据目录：{}",
                    path.display()
                ));
            }
            if meta.is_dir() {
                pending.push(path)
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}
fn files_to_copy(root: &Path) -> Result<Vec<PathBuf>, String> {
    Ok(walk(root)?
        .into_iter()
        .filter(|p| {
            let first = p
                .strip_prefix(root)
                .ok()
                .and_then(|r| r.components().next())
                .and_then(|c| c.as_os_str().to_str())
                .unwrap_or("");
            !matches!(
                first,
                "logs" | "updates" | "staging" | "auxiliary" | "tasks" | "migration-pending.json"
            )
        })
        .collect())
}
#[tauri::command]
pub fn migration_inspect(source: String) -> Result<Value, String> {
    let root = PathBuf::from(source)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let raw = fs::read(root.join("library.json")).map_err(|_| "所选目录没有 library.json")?;
    let library: Library = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let files = files_to_copy(&root)?;
    let bytes = files
        .iter()
        .map(|p| fs::metadata(p).map(|m| m.len()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum::<u64>();
    let mods = [&library.heroes, &library.monsters]
        .iter()
        .flat_map(|m| m.values())
        .flat_map(|h| h.sections.values())
        .flat_map(|s| s.resources.values())
        .map(|r| r.files.len())
        .sum::<usize>();
    Ok(
        json!({"source":root,"files":files.len(),"mods":mods,"bytes":bytes,"destination":data_dir()}),
    )
}
fn normalized(path: &str) -> String {
    path.trim_start_matches(r"\\?\")
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}
fn rewrite(value: &mut Value, old: &str, new: &str) {
    match value {
        Value::String(s) => {
            let path = normalized(s);
            let base = normalized(old);
            if path.len() >= base.len()
                && path
                    .get(..base.len())
                    .is_some_and(|v| v.eq_ignore_ascii_case(&base))
                && (path.len() == base.len() || path.as_bytes()[base.len()] == b'/')
            {
                *s = format!("{}{}", normalized(new), &path[base.len()..]);
            }
        }
        Value::Array(a) => {
            for v in a {
                rewrite(v, old, new)
            }
        }
        Value::Object(o) => {
            for v in o.values_mut() {
                rewrite(v, old, new)
            }
        }
        _ => {}
    }
}
pub fn copy_library(
    source: &Path,
    app: &tauri::AppHandle,
    id: &str,
    cancel: &AtomicBool,
) -> Result<Value, String> {
    let state = app.state::<AppState>();
    let _guard = state.mutation.lock().map_err(|_| "迁移锁不可用")?;
    copy_to(source, &data_dir(), cancel, |done, total, path| {
        app.state::<tasks::TaskManager>().update(
            app,
            id,
            "copy",
            path,
            done as f64,
            Some(total as f64),
        )
    })
}
fn copy_to(
    source: &Path,
    destination: &Path,
    cancel: &AtomicBool,
    progress: impl Fn(usize, usize, &str),
) -> Result<Value, String> {
    let root = source.canonicalize().map_err(|e| e.to_string())?;
    ensure_dir(destination).map_err(|e| e.to_string())?;
    let destination = destination.canonicalize().map_err(|e| e.to_string())?;
    crate::migration_journal::recover(&destination)?;
    if root.starts_with(&destination) || destination.starts_with(&root) {
        return Err("源目录与目标目录不能相同或互相包含".into());
    }
    let raw = fs::read(root.join("library.json")).map_err(|e| e.to_string())?;
    serde_json::from_slice::<Library>(&raw).map_err(|e| e.to_string())?;
    if destination.join("library.json").exists() {
        let current: Library = serde_json::from_slice(
            &fs::read(destination.join("library.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if !current.heroes.is_empty()
            || !current.monsters.is_empty()
            || !current.workshop.is_empty()
            || !current.settings.presets.is_empty()
        {
            return Err("新版资源库非空，请使用包导入以保留已有资源和预设".into());
        }
    }
    let files = files_to_copy(&root)?;
    let stage = crate::staging::create("migration")?;
    let backup = crate::staging::create("migration-backup")?;
    let mut journal = crate::migration_journal::Journal::begin(&destination, &stage, &backup)?;
    let result = (|| {
        let mut fingerprints = Vec::new();
        for (index, path) in files.iter().enumerate() {
            if cancel.load(Ordering::Acquire) {
                return Err("迁移已取消".into());
            }
            let rel = path
                .strip_prefix(&root)
                .map_err(|e| e.to_string())?
                .to_path_buf();
            let hash = sha256_file(path)?;
            let target = stage.join("new").join(&rel);
            ensure_dir(target.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::copy(path, &target).map_err(|e| e.to_string())?;
            if sha256_file(&target)? != hash || sha256_file(path)? != hash {
                return Err("复制期间源文件变化，请关闭旧版后重试".into());
            }
            fingerprints.push((rel, hash));
            progress(index + 1, files.len(), &path.to_string_lossy());
        }
        if files_to_copy(&root)? != files {
            return Err("复制期间源目录内容发生变化".into());
        }
        for (rel, hash) in &fingerprints {
            if sha256_file(&root.join(rel))? != *hash {
                return Err("复制期间源文件变化，请重试".into());
            }
        }
        for path in walk(&stage.join("new"))? {
            if path.extension().is_some_and(|x| x == "json") {
                if let Ok(mut value) =
                    serde_json::from_slice::<Value>(&fs::read(&path).map_err(|e| e.to_string())?)
                {
                    rewrite(
                        &mut value,
                        &root.to_string_lossy(),
                        &destination.to_string_lossy(),
                    );
                    fs::write(
                        &path,
                        serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        let report = json!({"source":root,"files":fingerprints.len(),"completedAt":unix_now(),"gameModsApplied":false});
        fs::write(stage.join("new/migration.json"), report.to_string())
            .map_err(|e| e.to_string())?;
        let mut commit_files = walk(&stage.join("new"))?;
        commit_files.sort_by_key(
            |p| match p.file_name().and_then(|s| s.to_str()).unwrap_or("") {
                "library.json" => 1,
                "migration.json" => 2,
                _ => 0,
            },
        );
        for path in commit_files {
            if cancel.load(Ordering::Acquire) {
                return Err("迁移已取消".into());
            }
            let rel = path.strip_prefix(stage.join("new")).unwrap().to_path_buf();
            journal.copy(&rel)?;
        }
        if cancel.load(Ordering::Acquire) {
            return Err("迁移已取消".into());
        }
        journal.complete()?;
        Ok(report)
    })();
    if result.is_err() {
        crate::migration_journal::recover(&destination)
            .map_err(|e| format!("迁移失败且恢复未完成：{e}；备份：{}", backup.display()))?;
    }
    crate::staging::remove(stage)?;
    crate::staging::remove(backup)?;
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copies_without_changing_source_and_rewrites_path_boundaries() {
        let root = crate::staging::create("migration-test").unwrap();
        let source = root.join("old");
        let destination = root.join("new");
        ensure_dir(&source).unwrap();
        let mut library = serde_json::to_value(Library::default()).unwrap();
        library["legacyPath"] = json!(source.join("vault/file.png").to_string_lossy());
        fs::write(source.join("library.json"), library.to_string()).unwrap();
        fs::write(source.join("image.png"), b"fixture").unwrap();
        let before = sha256_file(&source.join("library.json")).unwrap();
        copy_to(&source, &destination, &AtomicBool::new(false), |_, _, _| {}).unwrap();
        assert_eq!(sha256_file(&source.join("library.json")).unwrap(), before);
        let actual: Value =
            serde_json::from_slice(&fs::read(destination.join("library.json")).unwrap()).unwrap();
        assert!(actual["legacyPath"]
            .as_str()
            .unwrap()
            .contains("/new/vault/"));
        let mut unrelated = json!("C:/data-other/file.png");
        rewrite(&mut unrelated, "C:/data", "D:/new");
        assert_eq!(unrelated, "C:/data-other/file.png");
        crate::staging::remove(root).unwrap();
    }
    #[test]
    fn changing_source_during_copy_leaves_target_unpublished() {
        let root = crate::staging::create("migration-change-test").unwrap();
        let source = root.join("old");
        let target = root.join("new");
        ensure_dir(&source).unwrap();
        fs::write(
            source.join("library.json"),
            serde_json::to_vec(&Library::default()).unwrap(),
        )
        .unwrap();
        assert!(
            copy_to(&source, &target, &AtomicBool::new(false), |_, _, _| {
                fs::write(source.join("library.json"), b"changed").unwrap();
            })
            .is_err()
        );
        assert!(!target.join("library.json").exists());
        crate::staging::remove(root).unwrap();
    }
}
