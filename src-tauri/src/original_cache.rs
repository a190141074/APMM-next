//! 原图校验独立于工作区读取；只发布当前文件的成功结果，坏图不伪装成可用原图。
use crate::file_verification::{Stamp, VerificationCache, VerificationStats};
use crate::*;
use std::{collections::BTreeMap, sync::OnceLock};

struct State {
    cache: VerificationCache,
    pending: BTreeMap<PathBuf, bool>,
    failures: BTreeMap<PathBuf, Stamp>,
}
fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(State {
            cache: VerificationCache::load(
                &path(),
                format!("originals-v1:{}", data_dir().display()),
            ),
            pending: BTreeMap::new(),
            failures: BTreeMap::new(),
        })
    })
}
fn path() -> PathBuf {
    data_dir().join("cache/original-verification-v1.json")
}

/// None 表示正在等待工作线程；只做元数据读取，不在调用方计算图片摘要。
pub(crate) fn lookup(path: &Path, png: bool) -> Option<Result<String, String>> {
    let stamp = match Stamp::read(path) {
        Ok(v) => v,
        Err(e) => return Some(Err(e)),
    };
    let mut state = state().lock().ok()?;
    if let Some(hit) = state.cache.cached(path, &stamp, png) {
        return Some(Ok(hit.hash.clone()));
    }
    if state.failures.get(path) == Some(&stamp) {
        return Some(Err("原图校验失败".into()));
    }
    state
        .pending
        .entry(path.to_path_buf())
        .and_modify(|v| *v |= png)
        .or_insert(png);
    None
}

/// 哈希和解码期间不持缓存锁或库锁；扫描关闭时保留未完成项。
pub(crate) fn warm(checkpoint: impl Fn() -> Result<(), String>) -> Result<bool, String> {
    let (jobs, mut cache) = {
        let mut state = state().lock().map_err(|_| "原图缓存锁不可用")?;
        (std::mem::take(&mut state.pending), state.cache.clone())
    };
    if jobs.is_empty() {
        return Ok(false);
    }
    let mut stats = VerificationStats::default();
    for (path, png) in &jobs {
        if let Err(e) = checkpoint() {
            state()
                .lock()
                .map_err(|_| "原图缓存锁不可用")?
                .pending
                .extend(jobs.clone());
            return Err(e);
        }
        let value = cache.verify(path, *png, false, &mut stats);
        let mut state = state().lock().map_err(|_| "原图缓存锁不可用")?;
        match value {
            Ok(v) => {
                state.cache.entries.insert(path.clone(), v);
                state.failures.remove(path);
            }
            Err(_) => {
                if let Ok(stamp) = Stamp::read(path) {
                    state.failures.insert(path.clone(), stamp);
                }
            }
        }
        state.pending.remove(path);
    }
    let saved = state()
        .lock()
        .map_err(|_| "原图缓存锁不可用")?
        .cache
        .clone();
    if let Err(e) = saved.save(&path()) {
        write_log("warn", "originals.cache", &e, None);
    }
    write_log(
        "info",
        "originals.verify",
        "原图后台校验完成",
        serde_json::to_string(&stats).ok().as_deref(),
    );
    Ok(true)
}
