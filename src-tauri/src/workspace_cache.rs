//! 展示快照的派生缓存和版本事件；资源库仍是唯一可写状态，发布前核对库版本。
use crate::dto::{WorkspaceDelta, WorkspaceSnapshot};
use crate::file_verification::{library_token, write_cache};
use crate::*;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Serialize, Deserialize)]
struct Saved {
    schema: u32,
    key: String,
    snapshot: WorkspaceSnapshot,
}
fn path() -> PathBuf {
    data_dir().join("cache/workspace-snapshot-v1.json")
}
fn state() -> &'static Mutex<Option<Saved>> {
    static STATE: OnceLock<Mutex<Option<Saved>>> = OnceLock::new();
    STATE.get_or_init(|| {
        let saved = fs::read(path())
            .ok()
            .filter(|b| b.len() < 64 * 1024 * 1024)
            .and_then(|b| serde_json::from_slice::<Saved>(&b).ok())
            .filter(|s| s.schema == 1);
        Mutex::new(saved)
    })
}
fn key(lib: &Library) -> String {
    let mut hasher = Sha256::new();
    hasher.update(library_token(lib));
    hasher.update(env!("CARGO_PKG_VERSION"));
    hasher.update(data_dir().to_string_lossy().as_bytes());
    for dir in std::iter::once(data_dir().join("assets/maps")).chain(outimg_dirs(&lib.settings)) {
        hasher.update(dir.to_string_lossy().as_bytes());
        if let Ok(entries) = fs::read_dir(&dir) {
            let mut files = entries.flatten().map(|v| v.path()).collect::<Vec<_>>();
            files.sort();
            for p in files {
                hasher.update(p.to_string_lossy().as_bytes());
                if p.extension().is_some_and(|e| e == "json") {
                    if let Ok(bytes) = fs::read(&p) {
                        hasher.update(bytes);
                    }
                } else if let Ok(meta) = fs::metadata(&p) {
                    hasher.update(meta.len().to_le_bytes());
                    hasher.update(format!("{:?}", meta.modified().ok()));
                }
            }
        }
    }
    format!("{:x}", hasher.finalize())
}

pub(crate) fn delta(old: &WorkspaceSnapshot, next: &WorkspaceSnapshot) -> WorkspaceDelta {
    let old_rows = old
        .resources
        .iter()
        .map(|r| (&r.key, r))
        .collect::<BTreeMap<_, _>>();
    let new_keys = next
        .resources
        .iter()
        .map(|r| &r.key)
        .collect::<HashSet<_>>();
    WorkspaceDelta {
        base_revision: old.revision,
        revision: next.revision,
        resources: next
            .resources
            .iter()
            .filter(|r| {
                old_rows.get(&r.key).is_none_or(|old| {
                    serde_json::to_value(old).ok() != serde_json::to_value(r).ok()
                })
            })
            .cloned()
            .collect(),
        removed_keys: old
            .resources
            .iter()
            .filter(|r| !new_keys.contains(&r.key))
            .map(|r| r.key.clone())
            .collect(),
        full_refresh: serde_json::to_value(&old.settings).ok()
            != serde_json::to_value(&next.settings).ok()
            || old.data_dir != next.data_dir
            || old.game_dir != next.game_dir
            || old.migrated != next.migrated
            || old.version != next.version,
    }
}

/// 构建只读取元数据和缓存，图片验证由独立后台阶段完成。
pub(crate) fn get(
    app: &tauri::AppHandle,
    mutation: &Mutex<()>,
    revalidate: bool,
) -> Result<WorkspaceSnapshot, String> {
    if !revalidate {
        let lib = {
            let _lock = mutation.lock().map_err(|_| "库写入锁不可用")?;
            load_library()
        };
        let cache_key = key(&lib);
        // 有效快照的只读返回不能等待后台重建锁，否则快速启动仍会被原图遍历拖住。
        if let Some(saved) = state()
            .lock()
            .map_err(|_| "展示缓存锁不可用")?
            .as_ref()
            .filter(|s| s.key == cache_key)
        {
            return Ok(saved.snapshot.clone());
        }
    }
    static BUILD: OnceLock<Mutex<()>> = OnceLock::new();
    let _builder = BUILD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "工作区构建锁不可用")?;
    for _ in 0..3 {
        let lib = {
            let _lock = mutation.lock().map_err(|_| "库写入锁不可用")?;
            load_library()
        };
        let token = library_token(&lib);
        let cache_key = key(&lib);
        if !revalidate {
            if let Some(saved) = state()
                .lock()
                .map_err(|_| "展示缓存锁不可用")?
                .as_ref()
                .filter(|s| s.key == cache_key)
            {
                return Ok(saved.snapshot.clone());
            }
        }
        let mut next = crate::workspace::snapshot_from_library(&lib);
        let guard = mutation.lock().map_err(|_| "库写入锁不可用")?;
        if library_token(&load_library()) != token {
            continue;
        }
        let mut cache = state().lock().map_err(|_| "展示缓存锁不可用")?;
        let change = if let Some(saved) = cache.as_ref() {
            next.revision = saved.snapshot.revision;
            let mut event = delta(&saved.snapshot, &next);
            if event.resources.is_empty() && event.removed_keys.is_empty() && !event.full_refresh {
                None
            } else {
                next.revision = next.revision.saturating_add(1);
                event.revision = next.revision;
                Some(event)
            }
        } else {
            next.revision = 1;
            None
        };
        let saved = Saved {
            schema: 1,
            key: cache_key,
            snapshot: next.clone(),
        };
        drop(guard);
        if let Err(e) = write_cache(&path(), &saved) {
            write_log("warn", "workspace.cache", &e, None);
        }
        *cache = Some(saved);
        drop(cache);
        if let Some(event) = change {
            let _ = app.emit("workspace-updated", event);
        }
        return Ok(next);
    }
    Err("资源库正在变化，请稍后重试".into())
}
