//! 更新交接期间拒绝新写入，已有任务先完成；失败时由守卫恢复入口。
use std::sync::atomic::{AtomicBool, Ordering};
static CLOSED: AtomicBool = AtomicBool::new(false);
pub struct Guard;
impl Guard {
    pub fn enter() -> Result<Self, String> {
        if CLOSED.swap(true, Ordering::AcqRel) {
            Err("正在准备更新，请等待当前交接完成".into())
        } else {
            Ok(Self)
        }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        CLOSED.store(false, Ordering::Release);
    }
}
pub fn closed() -> bool {
    CLOSED.load(Ordering::Acquire)
}
pub fn allowed(command: &str) -> bool {
    !closed()
        || matches!(
            command,
            "workspace_snapshot"
                | "tasks_list"
                | "task_cancel"
                | "task_pause"
                | "update_download_status"
                | "update_startup_status"
                | "log_client_error"
        )
}
