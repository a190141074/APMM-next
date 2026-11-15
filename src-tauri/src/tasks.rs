//! 长任务统一所有权；快照修订号、排队、重试和取消不依赖页面生命周期。
use crate::dto::TaskSnapshot;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{Emitter, Listener, Manager};

struct Entry {
    snapshot: TaskSnapshot,
    cancel: Arc<AtomicBool>,
    args: Option<Value>,
    pause: bool,
}
pub struct TaskManager {
    entries: Mutex<BTreeMap<String, Entry>>,
    pub lane: Arc<tokio::sync::Semaphore>,
}
impl Default for TaskManager {
    fn default() -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            lane: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
}
impl TaskManager {
    pub fn restore() -> Self {
        let manager = Self::default();
        match crate::task_history::load(&crate::data_dir().join("tasks/history.json")) {
            Ok(records) => {
                let mut entries = manager.entries.lock().unwrap();
                for record in records {
                    entries.insert(
                        record.snapshot.id.clone(),
                        Entry {
                            snapshot: record.snapshot,
                            args: record.args,
                            cancel: Arc::new(AtomicBool::new(false)),
                            pause: false,
                        },
                    );
                }
            }
            Err(error) => crate::write_log("error", "tasks.restore", &error, None),
        }
        manager
    }
    fn persist(&self) {
        let entries = self.entries.lock().unwrap();
        let records: Vec<_> = entries
            .values()
            .rev()
            .take(200)
            .map(|entry| crate::task_history::Record {
                snapshot: entry.snapshot.clone(),
                args: entry
                    .args
                    .as_ref()
                    .and_then(|a| crate::task_history::retry_args(&entry.snapshot.kind, a)),
            })
            .collect();
        if let Err(error) = crate::portable_update::write_json(
            &crate::data_dir().join("tasks/history.json"),
            &records,
        ) {
            crate::write_log("error", "tasks.persist", &error, None);
        }
    }
    pub fn begin(&self, title: &str, kind: &str, args: Value) -> (String, Arc<AtomicBool>) {
        let id = format!("{:020}-{}", crate::unix_now(), uuid::Uuid::new_v4());
        let cancel = Arc::new(AtomicBool::new(false));
        let task = TaskSnapshot {
            id: id.clone(),
            title: title.into(),
            kind: kind.into(),
            status: "queued".into(),
            phase: "prepare".into(),
            message: "等待前面的任务结束".into(),
            done: 0.,
            total: None,
            speed: None,
            revision: 1,
            can_cancel: true,
            can_retry: false,
            result: None,
        };
        self.entries.lock().unwrap().insert(
            id.clone(),
            Entry {
                snapshot: task,
                cancel: cancel.clone(),
                args: Some(args),
                pause: false,
            },
        );
        self.persist();
        (id, cancel)
    }
    pub fn start(&self, app: &tauri::AppHandle, id: &str) {
        let snapshot = {
            let mut entries = self.entries.lock().unwrap();
            let entry = entries.get_mut(id).unwrap();
            entry.snapshot.status = "running".into();
            entry.snapshot.message = "准备中".into();
            entry.snapshot.revision += 1;
            entry.snapshot.can_cancel =
                !matches!(entry.snapshot.kind.as_str(), "workshop-uninstall");
            entry.snapshot.clone()
        };
        self.persist();
        let _ = app.emit("task-progress", snapshot);
    }
    pub fn update(
        &self,
        app: &tauri::AppHandle,
        id: &str,
        phase: &str,
        message: &str,
        done: f64,
        total: Option<f64>,
    ) {
        let snapshot = {
            let mut entries = self.entries.lock().unwrap();
            entries.get_mut(id).map(|entry| {
                let s = &mut entry.snapshot;
                s.phase = phase.into();
                s.message = message.into();
                s.done = done;
                s.total = total;
                s.revision += 1;
                s.clone()
            })
        };
        if let Some(s) = snapshot {
            let _ = app.emit("task-progress", s);
        }
    }
    pub fn current(&self) -> Option<(String, Arc<AtomicBool>)> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .find(|(_, e)| e.snapshot.status == "running")
            .map(|(id, e)| (id.clone(), e.cancel.clone()))
    }
    pub fn download_progress(&self, app: &tauri::AppHandle, done: u64, total: u64, speed: f64) {
        if let Some((id, _)) = self.current() {
            if let Some(e) = self.entries.lock().unwrap().get_mut(&id) {
                e.snapshot.speed = Some(speed);
            }
            self.update(
                app,
                &id,
                "download",
                "正在下载并校验",
                done as f64,
                Some(total as f64),
            );
        }
    }
    pub fn finish(&self, app: &tauri::AppHandle, id: &str, result: Result<Value, String>) {
        let snapshot = {
            let mut entries = self.entries.lock().unwrap();
            let entry = entries.get_mut(id).unwrap();
            let s = &mut entry.snapshot;
            match result {
                Ok(v) => {
                    s.status = "completed".into();
                    s.message = "已完成".into();
                    s.result = Some(v)
                }
                Err(e) => {
                    s.status = if entry.pause {
                        "paused"
                    } else if entry.cancel.load(Ordering::Relaxed) {
                        "cancelled"
                    } else {
                        "failed"
                    }
                    .into();
                    s.message = e;
                }
            }
            s.can_cancel = false;
            s.can_retry = entry.args.is_some()
                && matches!(s.status.as_str(), "failed" | "cancelled" | "paused");
            s.revision += 1;
            s.clone()
        };
        self.persist();
        let _ = app.emit("task-progress", snapshot);
    }
    pub fn active(&self) -> bool {
        self.entries
            .lock()
            .unwrap()
            .values()
            .any(|e| matches!(e.snapshot.status.as_str(), "queued" | "running"))
    }
}
/// 旧命令可直接调用；仅在统一任务上下文存在时检查取消信号。
pub fn cancellation(app: &tauri::AppHandle) -> Option<Arc<AtomicBool>> {
    app.try_state::<TaskManager>()
        .and_then(|s| s.current().map(|(_, flag)| flag))
}
pub fn checkpoint(app: &tauri::AppHandle) -> Result<(), String> {
    if cancellation(app).is_some_and(|f| f.load(Ordering::Acquire)) {
        return Err("任务已取消，尚未提交的修改已放弃".into());
    }
    Ok(())
}
/// 最后一次取消检查之后进入短暂的原子提交区间，前端同步禁用取消。
pub fn commit_phase(app: &tauri::AppHandle) -> Result<(), String> {
    checkpoint(app)?;
    if let Some(state) = app.try_state::<TaskManager>() {
        if let Some((id, _)) = state.current() {
            let snapshot = {
                let mut entries = state.entries.lock().unwrap();
                let e = entries.get_mut(&id).unwrap();
                e.snapshot.can_cancel = false;
                e.snapshot.phase = "commit".into();
                e.snapshot.revision += 1;
                e.snapshot.clone()
            };
            let _ = app.emit("task-progress", snapshot);
        }
    }
    Ok(())
}
#[tauri::command]
pub fn tasks_list(state: tauri::State<TaskManager>) -> Vec<TaskSnapshot> {
    state
        .entries
        .lock()
        .unwrap()
        .values()
        .map(|e| e.snapshot.clone())
        .collect()
}
#[tauri::command]
pub fn task_cancel(id: String, app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<TaskManager>();
    let mut entries = state.entries.lock().unwrap();
    let e = entries.get_mut(&id).ok_or("任务不存在")?;
    if !e.snapshot.can_cancel {
        return Err("当前任务已结束或正在不可取消的提交阶段".into());
    }
    e.cancel.store(true, Ordering::Release);
    if e.snapshot.status == "running" {
        let legacy = app.state::<crate::AppState>();
        match e.snapshot.kind.as_str() {
            "preview" => legacy.preview_cancel.store(true, Ordering::Release),
            "apx-install" | "apx-restore" | "apx-cache" | "apx-export" => {
                legacy.plugin_cancel.store(true, Ordering::Release)
            }
            "update-download" => legacy.update_cancel.store(true, Ordering::Release),
            _ => {}
        }
    }
    Ok(())
}
#[tauri::command]
pub fn task_pause(id: String, app: tauri::AppHandle) -> Result<(), String> {
    {
        let state = app.state::<TaskManager>();
        let mut entries = state.entries.lock().unwrap();
        let e = entries.get_mut(&id).ok_or("任务不存在")?;
        if !matches!(
            e.snapshot.kind.as_str(),
            "preview" | "apx-cache" | "update-download" | "aux-install"
        ) {
            return Err("此任务不支持暂停".into());
        }
        e.pause = true;
    }
    task_cancel(id, app)
}
#[tauri::command]
pub fn task_retry(id: String, app: tauri::AppHandle) -> Result<String, String> {
    let (kind, title, args) = {
        let state = app.state::<TaskManager>();
        let entries = state.entries.lock().unwrap();
        let e = entries.get(&id).ok_or("任务不存在")?;
        if !matches!(
            e.snapshot.status.as_str(),
            "failed" | "cancelled" | "paused"
        ) {
            return Err("只可重试已停止的任务".into());
        }
        (
            e.snapshot.kind.clone(),
            e.snapshot.title.clone(),
            e.args
                .clone()
                .ok_or("此任务的密码或临时队列未保存，请重新打开导入或导出窗口")?,
        )
    };
    crate::task_dispatch::task_execute(kind, title, args, app)
}
pub fn bridge(app: &tauri::AppHandle) {
    for event in [
        "mod-scan-progress",
        "preview-progress",
        "plugin-progress",
        "workshop-progress",
        "package-progress",
        "app-update-progress",
    ] {
        let handle = app.clone();
        app.listen(event, move |e| {
            if let (Ok(v), Some((id, _))) = (
                serde_json::from_str::<Value>(e.payload()),
                handle.state::<TaskManager>().current(),
            ) {
                let done = v["done"]
                    .as_f64()
                    .or(v["downloaded"].as_f64())
                    .or(v["percent"].as_f64())
                    .unwrap_or(0.);
                let total = v["total"]
                    .as_f64()
                    .or_else(|| v["percent"].as_f64().map(|_| 100.));
                handle.state::<TaskManager>().update(
                    &handle,
                    &id,
                    v["phase"].as_str().unwrap_or("working"),
                    v["message"].as_str().unwrap_or("处理中"),
                    done,
                    total,
                );
            }
        });
    }
}
