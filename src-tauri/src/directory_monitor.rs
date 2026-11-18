//! 应用级增量监听与后台同步状态。监听生命周期不随页面卸载，退出时取消并释放句柄。
use crate::directory_observation::Observation;
use crate::directory_watch::WatchGroup;
use crate::*;
use tauri::Manager;
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct DirectorySyncStatus {
    pub enabled: bool,
    pub running: bool,
    pub paths: Vec<String>,
    pub revision: u32,
    pub last_error: Option<String>,
    pub phase: String,
    pub checked: u32,
    pub total: u32,
}
#[derive(Default)]
pub(crate) struct DirectoryMonitor {
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<DirectorySyncStatus>>,
}
impl Drop for DirectoryMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}
impl DirectoryMonitor {
    pub(crate) fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}
#[tauri::command]
pub(crate) fn directory_sync_status(state: tauri::State<DirectoryMonitor>) -> DirectorySyncStatus {
    state.status.lock().map(|s| s.clone()).unwrap_or_default()
}
fn publish(app: &tauri::AppHandle, edit: impl FnOnce(&mut DirectorySyncStatus)) {
    let state = app.state::<DirectoryMonitor>();
    if let Ok(mut status) = state.status.lock() {
        edit(&mut status);
        status.revision = status.revision.wrapping_add(1);
        let _ = app.emit("workspace-changed", status.clone());
    };
}
pub(crate) fn progress(app: &tauri::AppHandle, phase: &str, done: usize, total: usize) {
    publish(app, |s| {
        s.phase = phase.into();
        s.running = true;
        s.checked = done as u32;
        s.total = total as u32;
    });
}
pub(crate) fn finished(app: &tauri::AppHandle, error: Option<String>) {
    publish(app, |s| {
        s.phase = "idle".into();
        s.running = false;
        s.last_error = error;
    });
}
fn display_roots(lib: &Library) -> Vec<PathBuf> {
    let mut roots = vec![data_dir().join("assets"), preview_cache_dir()];
    roots.extend(effective_outimg_dir(&lib.settings));
    roots.retain(|p| p.is_dir());
    roots.sort();
    roots.dedup();
    roots
}
fn observation_token(value: &Observation) -> String {
    let contents = value
        .roots
        .iter()
        .flat_map(|(root, values)| {
            values
                .files
                .iter()
                .map(move |(p, v)| (root, p, v.as_ref().map(|f| &f.hash).map_err(String::as_str)))
        })
        .collect::<Vec<_>>();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&contents).unwrap_or_default())
    )
}

pub(crate) fn start(app: &tauri::AppHandle) {
    let stop = app.state::<DirectoryMonitor>().stop.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let mut config = String::new();
        let mut watchers: Option<WatchGroup> = None;
        let mut scan_roots = Vec::new();
        let mut cache = crate::file_verification::VerificationCache::default();
        let mut previous: Option<Observation> = None;
        let mut next_config = Instant::now();
        let mut pending = None;
        let mut dirty = Vec::<PathBuf>::new();
        let mut force = false;
        let mut last_attempt = String::new();
        let mut known_library = String::new();
        let mut check_originals = true;
        while !stop.load(Ordering::Acquire) {
            let state = app.state::<AppState>();
            if Instant::now() >= next_config {
                next_config = Instant::now() + Duration::from_secs(1);
                // 设置变化只在取得短锁时读取，不和正在提交的事务争抢窗口线程。
                let lib = state.mutation.try_lock().ok().map(|_guard| load_library());
                if let Some(lib) = lib {
                    let library_version = crate::file_verification::library_token(&lib);
                    let roots = crate::directory_scan::roots(&lib);
                    let views = display_roots(&lib);
                    let new_config = format!(
                        "{}:{:?}:{:?}",
                        lib.settings.monitor_directories, roots, views
                    );
                    if new_config != config {
                        config = new_config;
                        watchers = None;
                        previous = None;
                        dirty.clear();
                        force = false;
                        last_attempt.clear();
                        scan_roots = roots;
                        cache = crate::directory_scan::cache(&scan_roots);
                        let enabled = lib.settings.monitor_directories;
                        let mut watched = views;
                        if enabled {
                            watched.extend(scan_roots.clone());
                        }
                        watched.sort();
                        watched.dedup();
                        watchers = Some(WatchGroup::start(watched));
                        pending = enabled.then(Instant::now);
                        check_originals = true;
                        publish(&app, |s| {
                            s.enabled = enabled;
                            s.running = enabled;
                            s.phase = if enabled { "waiting" } else { "idle" }.into();
                            s.checked = 0;
                            s.total = 0;
                            s.last_error = None;
                            s.paths = scan_roots
                                .iter()
                                .map(|p| p.to_string_lossy().into_owned())
                                .collect();
                        });
                    } else if library_version != known_library {
                        // 手动完整校验或其他事务可能更新校验缓存，旧观察不能反向覆盖新结果。
                        previous = None;
                        cache = crate::directory_scan::cache(&scan_roots);
                        if lib.settings.monitor_directories {
                            pending = Some(Instant::now());
                        }
                    }
                    known_library = library_version;
                    if let Err(e) = crate::workspace_cache::get(&app, &state.mutation, false) {
                        write_log("warn", "workspace.refresh", &e, None);
                    }
                }
            }
            if let Some(watch) = &watchers {
                while let Ok(event) = watch.events.try_recv() {
                    if event.full {
                        force = true;
                    }
                    if event.error.is_some() {
                        publish(&app, |s| s.last_error = event.error.clone());
                    }
                    check_originals |= event.full
                        || event
                            .paths
                            .iter()
                            .any(|p| !scan_roots.iter().any(|r| p.starts_with(r)));
                    dirty.extend(
                        event
                            .paths
                            .into_iter()
                            .filter(|p| scan_roots.iter().any(|r| p.starts_with(r))),
                    );
                    if app
                        .state::<DirectoryMonitor>()
                        .status
                        .lock()
                        .map(|s| s.enabled)
                        .unwrap_or(false)
                        && (event.full || !dirty.is_empty() || cfg!(not(windows)))
                    {
                        pending = Some(Instant::now());
                    }
                }
            }
            if check_originals {
                let _ = crate::workspace_cache::get(&app, &state.mutation, true);
                check_originals = false;
            }
            let checkpoint = || {
                if stop.load(Ordering::Acquire) {
                    Err("目录同步已取消".into())
                } else {
                    Ok(())
                }
            };
            match crate::original_cache::warm(checkpoint) {
                Ok(true) => {
                    let _ = crate::workspace_cache::get(&app, &state.mutation, true);
                }
                Err(e) if !stop.load(Ordering::Acquire) => {
                    write_log("warn", "originals.verify", &e, None)
                }
                _ => {}
            }
            if pending.is_some_and(|at: Instant| at.elapsed() >= Duration::from_millis(700))
                && !crate::update_gate::closed()
            {
                let manager = app.state::<tasks::TaskManager>();
                if let Ok(_permit) = manager.lane.clone().try_acquire_owned() {
                    let generation = watchers
                        .as_ref()
                        .map(|w| w.generation.load(Ordering::Acquire))
                        .unwrap_or(0);
                    dirty.sort();
                    dirty.dedup();
                    let started = Instant::now();
                    progress(&app, if force { "full" } else { "incremental" }, 0, 0);
                    let outcome = (|| -> Result<(), String> {
                        let baseline = crate::directory_scan::read_library(&app)?;
                        let prior = if cfg!(not(windows)) {
                            None
                        } else {
                            previous.as_ref()
                        };
                        let observation = Observation::collect(
                            &scan_roots,
                            &mut cache,
                            prior,
                            &dirty,
                            force,
                            checkpoint,
                            |done, total| {
                                progress(
                                    &app,
                                    if force { "full" } else { "incremental" },
                                    done,
                                    total,
                                )
                            },
                        )?;
                        let stable_generation = || {
                            checkpoint()?;
                            if watchers
                                .as_ref()
                                .is_some_and(|w| w.generation.load(Ordering::Acquire) != generation)
                            {
                                Err(crate::directory_scan::RETRY.into())
                            } else {
                                Ok(())
                            }
                        };
                        stable_generation()?;
                        let attempt = format!(
                            "{}:{}",
                            crate::file_verification::library_token(&baseline),
                            observation_token(&observation)
                        );
                        let error = if attempt != last_attempt || force {
                            let result = crate::directory_scan::commit(
                                &app,
                                &baseline,
                                &observation,
                                stable_generation,
                                checkpoint,
                            )?;
                            known_library = crate::file_verification::library_token(
                                &crate::directory_scan::read_library(&app)?,
                            );
                            last_attempt =
                                format!("{}:{}", known_library, observation_token(&observation));
                            if result.errors.is_empty() {
                                None
                            } else {
                                Some(result.errors.join("\n"))
                            }
                        } else {
                            app.state::<DirectoryMonitor>()
                                .status
                                .lock()
                                .ok()
                                .and_then(|s| s.last_error.clone())
                        };
                        observation.trim_cache(&mut cache);
                        if let Err(e) = cache.save(&crate::directory_scan::cache_path()) {
                            write_log("warn", "directory.cache", &e, None);
                        }
                        let detail = serde_json::json!({"elapsedMs":started.elapsed().as_millis(),"force":force,"stats":observation.stats});
                        write_log(
                            "info",
                            "directory.incremental",
                            "后台目录核对完成",
                            Some(&detail.to_string()),
                        );
                        previous = Some(observation);
                        pending = None;
                        dirty.clear();
                        force = false;
                        finished(&app, error);
                        crate::workspace_cache::get(&app, &state.mutation, false)?;
                        Ok(())
                    })();
                    if let Err(error) = outcome {
                        finished(&app, Some(error.clone()));
                        if error != crate::directory_scan::RETRY {
                            write_log("warn", "directory.incremental", &error, None);
                        }
                        previous = None;
                        pending = Some(
                            Instant::now()
                                + Duration::from_secs(if error == crate::directory_scan::RETRY {
                                    0
                                } else {
                                    4
                                }),
                        );
                    }
                };
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        drop(watchers);
    });
}
