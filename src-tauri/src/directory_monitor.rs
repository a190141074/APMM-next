//! 应用级目录监听。合并原生通知、等待文件稳定后进入任务协调通道；退出时释放句柄。
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

#[cfg(windows)]
struct Watch(windows::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Watch {
    fn open(path: &Path) -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;
        use windows::{core::PCWSTR, Win32::Storage::FileSystem::*};
        let name = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // 句柄仅由监听线程使用和释放；递归通知覆盖原子替换、重命名和禁用后缀变化。
        unsafe {
            FindFirstChangeNotificationW(
                PCWSTR(name.as_ptr()),
                true,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_DIR_NAME
                    | FILE_NOTIFY_CHANGE_SIZE
                    | FILE_NOTIFY_CHANGE_LAST_WRITE,
            )
            .map(Self)
            .map_err(|e| e.to_string())
        }
    }
    fn changed(&self) -> Result<bool, String> {
        use windows::Win32::{
            Foundation::*, Storage::FileSystem::FindNextChangeNotification,
            System::Threading::WaitForSingleObject,
        };
        unsafe {
            match WaitForSingleObject(self.0, 0) {
                WAIT_OBJECT_0 => {
                    FindNextChangeNotification(self.0).map_err(|e| e.to_string())?;
                    Ok(true)
                }
                WAIT_TIMEOUT => Ok(false),
                _ => Err("目录监听句柄失效".into()),
            }
        }
    }
}
#[cfg(windows)]
impl Drop for Watch {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Storage::FileSystem::FindCloseChangeNotification(self.0);
        }
    }
}

fn fingerprint(paths: &[String]) -> Vec<(PathBuf, u64, u128)> {
    let mut result = Vec::new();
    for root in paths {
        if let Ok(files) = crate::directory_files::images_under(Path::new(root)) {
            for path in files {
                if let Ok(meta) = fs::metadata(&path) {
                    result.push((
                        path,
                        meta.len(),
                        meta.modified()
                            .ok()
                            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                            .map(|d| d.as_nanos())
                            .unwrap_or(0),
                    ));
                }
            }
        }
    }
    result
}
// 内容摘要用于识别自身事务的回滚通知；仍重算字节哈希，不能用大小和时间跳过外部编辑。
fn content_digest(paths: &[String]) -> Vec<(PathBuf, String)> {
    let mut result = Vec::new();
    for root in paths {
        if let Ok(files) = crate::directory_files::images_under(Path::new(root)) {
            for path in files {
                result.push((
                    path.clone(),
                    sha256_file(&path).unwrap_or_else(|e| format!("error:{e}")),
                ));
            }
        }
    }
    result
}
fn publish(
    app: &tauri::AppHandle,
    status: &Arc<Mutex<DirectorySyncStatus>>,
    error: Option<String>,
    running: bool,
) {
    if let Ok(mut s) = status.lock() {
        s.last_error = error;
        s.running = running;
        s.revision = s.revision.wrapping_add(1);
        let _ = app.emit("workspace-changed", s.clone());
    }
}
pub(crate) fn start(app: &tauri::AppHandle) {
    let state = app.state::<DirectoryMonitor>();
    let stop = state.stop.clone();
    let status = state.status.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        #[cfg(windows)]
        let mut watchers: Vec<Watch> = Vec::new();
        let mut config = String::new();
        let mut paths = Vec::<String>::new();
        let mut next_config = Instant::now();
        let mut pending = Some(Instant::now());
        let mut stable = None;
        let mut prior_error = None;
        let mut last_content = None;
        while !stop.load(Ordering::Acquire) {
            if Instant::now() >= next_config {
                next_config = Instant::now() + Duration::from_secs(1);
                let state = app.state::<AppState>();
                if let Ok(_guard) = state.mutation.try_lock() {
                    let settings = load_library().settings;
                    let new_config =
                        format!("{}:{}", settings.monitor_directories, settings.modimg_path);
                    if new_config != config {
                        config = new_config;
                        #[cfg(windows)]
                        watchers.clear();
                        paths.clear();
                        stable = None;
                        last_content = None;
                        if settings.monitor_directories {
                            paths.push(vault_dir().to_string_lossy().into_owned());
                            if Path::new(&settings.modimg_path).is_dir() {
                                paths.push(settings.modimg_path);
                            }
                            #[cfg(windows)]
                            for path in &paths {
                                match Watch::open(Path::new(path)) {
                                    Ok(w) => watchers.push(w),
                                    Err(e) => publish(&app, &status, Some(e), false),
                                }
                            }
                            pending = Some(Instant::now());
                        } else {
                            pending = None;
                        }
                        if let Ok(mut s) = status.lock() {
                            s.enabled = settings.monitor_directories;
                            s.paths = paths.clone();
                        }
                        publish(&app, &status, None, false);
                    }
                };
            }
            #[cfg(windows)]
            for watcher in &watchers {
                match watcher.changed() {
                    Ok(true) => {
                        pending = Some(Instant::now());
                        stable = None;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        if prior_error.as_ref() != Some(&e) {
                            publish(&app, &status, Some(e.clone()), false);
                            prior_error = Some(e);
                        }
                    }
                }
            }
            #[cfg(not(windows))]
            if !paths.is_empty() && pending.is_none() {
                pending = Some(Instant::now());
            }
            if pending.is_some_and(|at| at.elapsed() >= Duration::from_millis(700)) {
                let current = fingerprint(&paths);
                if stable.as_ref() != Some(&current) {
                    stable = Some(current);
                    pending = Some(Instant::now());
                } else {
                    let digest = content_digest(&paths);
                    if last_content.as_ref() == Some(&digest) {
                        pending = None;
                        stable = None;
                        continue;
                    }
                    let manager = app.state::<tasks::TaskManager>();
                    if crate::update_gate::closed() { continue; }
                    if let Ok(_permit) = manager.lane.clone().try_acquire_owned() {
                        let state = app.state::<AppState>();
                        if let Ok(_guard) = state.mutation.try_lock() {
                            if stop.load(Ordering::Acquire) {
                                break;
                            }
                            publish(&app, &status, None, true);
                            let mut lib = load_library();
                            let game = PathBuf::from(&lib.settings.modimg_path);
                            let outcome = directory_sync::synchronize_controlled(
                                &mut lib,
                                &vault_dir(),
                                &game,
                                save_library,
                                || {
                                    if stop.load(Ordering::Acquire) {
                                        Err("目录同步已取消".into())
                                    } else {
                                        Ok(())
                                    }
                                },
                            )
                            .and_then(|r| {
                                if stop.load(Ordering::Acquire) {
                                    return Err("目录同步已取消".into());
                                }
                                if r.changed {
                                    save_library(&lib)?;
                                }
                                Ok(r)
                            });
                            let error = match outcome {
                                Ok(r) if r.errors.is_empty() => None,
                                Ok(r) => Some(r.errors.join("\n")),
                                Err(e) => Some(e),
                            };
                            if error != prior_error {
                                if let Some(e) = &error {
                                    write_log("error", "directory.sync", e, None);
                                    let (id, _) = manager.begin(
                                        "目录同步异常",
                                        "directory-sync",
                                        serde_json::json!({}),
                                    );
                                    manager.finish(&app, &id, Err(e.clone()));
                                }
                                prior_error = error.clone();
                            }
                            // 只确认本次扫描开始前的摘要，扫描期间的新编辑必须在下一轮读取。
                            // 自身发布候选最多再扫描一次；失败回滚到相同内容时不会无限重试。
                            last_content = Some(digest);
                            publish(&app, &status, error, false);
                            pending = None;
                            stable = None;
                        };
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        // 原生句柄在此线程的 Watch 析构中释放，后续不会再次进入扫描。
    });
}
