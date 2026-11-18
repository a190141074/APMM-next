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
    #[ignore = "需要工作盘下全新的 ASTRAL_DEV_DATA_DIR，使用 --include-ignored 验收"]
    fn migrated_library_applies_and_replaces_same_slot_with_rollback() {
        use crate::{dto::ResourceAction, management_tests::png};

        let destination = PathBuf::from(std::env::var("ASTRAL_DEV_DATA_DIR").unwrap());
        assert_eq!(data_dir(), destination);
        let root = destination.parent().unwrap().canonicalize().unwrap();
        let work = crate::staging::work_root().canonicalize().unwrap();
        assert!(root.starts_with(&work) && root != work);
        let source = root.join("old-data");
        let game = root.join("test-game/modimg");
        assert!(!destination.exists() && !source.exists() && !game.exists());

        const SLOT: &str = "UT_Hero_Bust_101_01_0";
        let old_vault = source.join("vault");
        let first_fixture = root.join("first.png");
        png(&first_fixture, 10);
        let first_hash = sha256_file(&first_fixture).unwrap();
        let second_rel = "heroes/101/busts/second.png";
        ensure_dir(&old_vault.join(second_rel).parent().unwrap()).unwrap();
        fs::rename(first_fixture, old_vault.join(format!("{first_hash}.png"))).unwrap();
        png(&old_vault.join(second_rel), 20);
        let second_hash = sha256_file(&old_vault.join(second_rel)).unwrap();

        // 按旧版字段写入历史扁平 Vault 和树状 Vault 的两个同槽位候选。
        let legacy = json!({
            "version": 1,
            "settings": { "modimgPath": game },
            "heroes": { "101": { "sections": { "busts": { "resources": { (SLOT): {
                "activeHash": first_hash,
                "files": [
                    { "hash": first_hash, "sourceName": "first.png", "ext": ".png" },
                    { "hash": second_hash, "sourceName": "second.png", "ext": ".png", "relPath": second_rel }
                ]
            } } } } } }
        });
        let old_library = source.join("library.json");
        fs::write(&old_library, serde_json::to_vec(&legacy).unwrap()).unwrap();
        let old_hash = sha256_file(&old_library).unwrap();
        ensure_dir(&game).unwrap();
        let other_file = game.join("UT_Hero_Bust_101_01_1.png");
        let output = game.join(format!("{SLOT}.png"));
        png(&other_file, 30);
        let other_hash = sha256_file(&other_file).unwrap();
        let report = copy_to(&source, &destination, &AtomicBool::new(false), |_, _, _| {}).unwrap();
        assert_eq!(report["gameModsApplied"], false);
        assert_eq!(PathBuf::from(load_library().settings.modimg_path), game);
        assert_eq!(sha256_file(&old_library).unwrap(), old_hash);
        assert_eq!(sha256_file(&other_file).unwrap(), other_hash);
        assert!(!output.exists());

        let mutation = Mutex::new(());
        let apply = |hash: &str| {
            crate::workspace::apply_items(
                vec![ResourceAction {
                    entity_id: "101".into(),
                    section: "busts".into(),
                    canonical: SLOT.into(),
                    hash: Some(hash.into()),
                }],
                &mutation,
            )
        };
        let read_slot = || load_library().heroes["101"].sections["busts"].resources[SLOT].clone();
        assert_eq!(
            read_slot().active_hash.as_deref(),
            Some(first_hash.as_str())
        );
        apply(&first_hash).unwrap();
        assert_eq!(sha256_file(&output).unwrap(), first_hash);
        apply(&second_hash).unwrap();
        assert_eq!(sha256_file(&output).unwrap(), second_hash);
        assert_eq!(
            read_slot().active_hash.as_deref(),
            Some(second_hash.as_str())
        );
        assert_eq!(read_slot().files.len(), 2);

        // 用目录占住库文件的暂存路径，验证发布索引失败会恢复游戏文件。
        fs::create_dir(destination.join("library.json.part")).unwrap();
        assert!(apply(&first_hash)
            .unwrap_err()
            .contains("写入库暂存文件失败"));
        assert_eq!(sha256_file(&output).unwrap(), second_hash);
        assert_eq!(
            read_slot().active_hash.as_deref(),
            Some(second_hash.as_str())
        );
        assert_eq!(sha256_file(&other_file).unwrap(), other_hash);
        assert_eq!(sha256_file(&old_library).unwrap(), old_hash);
    }

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
