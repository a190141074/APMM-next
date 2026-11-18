//! 后台观察后短暂提交。版本冲突或扫描期间的新通知会废弃观察，不覆盖较新的库状态。
use crate::directory_observation::Observation;
use crate::file_verification::{library_token, VerificationCache};
use crate::*;
use tauri::Manager;

pub(crate) const RETRY: &str = "目录或资源库在扫描期间发生变化，重新排队";

pub(crate) fn can_commit(baseline: &Library, current: &Library, observation: &Observation) -> bool {
    library_token(current) == library_token(baseline) && observation.stable()
}

pub(crate) fn roots(lib: &Library) -> Vec<PathBuf> {
    let mut roots = vec![vault_dir()];
    if !lib.settings.modimg_path.is_empty() {
        roots.push(PathBuf::from(&lib.settings.modimg_path));
    }
    roots
}
pub(crate) fn cache_path() -> PathBuf {
    data_dir().join("cache/directory-verification-v1.json")
}
pub(crate) fn cache(roots: &[PathBuf]) -> VerificationCache {
    let context = format!(
        "png-image-0.25.9-v1:{}",
        roots
            .iter()
            .map(|p| { format!("{}:{:?}", p.display(), p.canonicalize().ok()) })
            .collect::<Vec<_>>()
            .join("|")
    );
    VerificationCache::load(&cache_path(), context)
}

pub(crate) fn read_library(app: &tauri::AppHandle) -> Result<Library, String> {
    let state = app.state::<AppState>();
    let _guard = state.mutation.lock().map_err(|_| "库写入锁不可用")?;
    Ok(load_library())
}

/// 调用者已经取得写任务通道；进入提交前观察必须仍匹配当前库及目录。
pub(crate) fn commit(
    app: &tauri::AppHandle,
    baseline: &Library,
    observation: &Observation,
    validate_generation: impl Fn() -> Result<(), String>,
    checkpoint: impl Fn() -> Result<(), String>,
) -> Result<crate::directory_sync::SyncResult, String> {
    checkpoint()?;
    validate_generation()?;
    let state = app.state::<AppState>();
    let _guard = state.mutation.lock().map_err(|_| "库写入锁不可用")?;
    let mut lib = load_library();
    if !can_commit(baseline, &lib, observation) {
        return Err(RETRY.into());
    }
    checkpoint()?;
    validate_generation()?;
    crate::tasks::commit_phase(app)?;
    let game = PathBuf::from(&lib.settings.modimg_path);
    let result = crate::directory_sync::synchronize_observed(
        &mut lib,
        &vault_dir(),
        &game,
        observation,
        save_library,
        checkpoint,
    )?;
    if result.changed {
        save_library(&lib)?;
    }
    Ok(result)
}

/// 手动扫描始终重读和解码；任务通道由任务调度器持有，校验期间不持库锁。
pub(crate) fn full_scan(app: &tauri::AppHandle) -> Result<crate::ReconcileResult, String> {
    for _ in 0..3 {
        crate::tasks::checkpoint(app)?;
        let lib = read_library(app)?;
        let roots = roots(&lib);
        let mut cache = cache(&roots);
        let observed = Observation::collect(
            &roots,
            &mut cache,
            None,
            &[],
            true,
            || crate::tasks::checkpoint(app),
            |done, total| crate::directory_monitor::progress(app, "full", done, total),
        )?;
        match commit(
            app,
            &lib,
            &observed,
            || Ok(()),
            || crate::tasks::checkpoint(app),
        ) {
            Err(e) if e == RETRY => continue,
            Err(e) => return Err(e),
            Ok(result) => {
                observed.trim_cache(&mut cache);
                if let Err(e) = cache.save(&cache_path()) {
                    write_log("warn", "directory.cache", &e, None);
                }
                let current = read_library(app)?;
                let added = crate::candidate_management::candidates(&current)
                    .len()
                    .saturating_sub(crate::candidate_management::candidates(&lib).len());
                write_log(
                    "info",
                    "directory.scan",
                    "完整校验完成",
                    serde_json::to_string(&observed.stats).ok().as_deref(),
                );
                if let Err(e) =
                    crate::workspace_cache::get(app, &app.state::<AppState>().mutation, true)
                {
                    write_log("warn", "workspace.refresh", &e, None);
                }
                return Ok(crate::ReconcileResult {
                    added,
                    errors: result.errors,
                    ..Default::default()
                });
            }
        }
    }
    Err(RETRY.into())
}
