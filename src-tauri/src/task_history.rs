//! 保存任务历史和可重试参数；密码只留在当前进程，重启后不恢复包队列。
use crate::dto::TaskSnapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct Record {
    pub snapshot: TaskSnapshot,
    pub args: Option<Value>,
}

pub fn retry_args(kind: &str, args: &Value) -> Option<Value> {
    fn contains_secret(v: &Value) -> bool {
        match v {
            Value::Object(fields) => fields.iter().any(|(key, value)| {
                ["password", "credential", "authorization", "token", "secret"]
                    .iter()
                    .any(|word| key.to_ascii_lowercase().contains(word))
                    || contains_secret(value)
            }),
            Value::Array(values) => values.iter().any(contains_secret),
            _ => false,
        }
    }
    if kind == "package-commit" || contains_secret(args) {
        None
    } else {
        Some(args.clone())
    }
}

pub fn load(path: &Path) -> Result<Vec<Record>, String> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut records: Vec<Record> =
        serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    for record in &mut records {
        let s = &mut record.snapshot;
        if matches!(s.status.as_str(), "queued" | "running") {
            s.status = "failed".into();
            s.phase = "interrupted".into();
            s.message = "程序在任务结束前退出。确认来源仍有效后重试。".into();
            s.revision = s.revision.saturating_add(1);
        }
        s.can_cancel = false;
        s.can_retry =
            record.args.is_some() && matches!(s.status.as_str(), "failed" | "cancelled" | "paused");
        if !s.can_retry && matches!(s.status.as_str(), "failed" | "cancelled" | "paused") {
            s.message
                .push_str(" 请重新打开导入或导出窗口；密码和临时队列不会保存到历史。");
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passwords_and_ephemeral_queues_are_never_persisted() {
        assert!(retry_args("package-export", &serde_json::json!({"password":"secret"})).is_none());
        assert!(retry_args("package-commit", &serde_json::json!({"queueId":"gone"})).is_none());
        assert!(retry_args("aux-run", &serde_json::json!({"command":"sync"})).is_some());
    }
    #[test]
    fn restart_marks_running_work_interrupted_with_a_new_revision() {
        let root = crate::staging::create("task-history-test").unwrap();
        let path = root.join("history.json");
        std::fs::write(&path, serde_json::json!([{
            "snapshot":{"id":"task", "title":"提取", "kind":"aux-run", "status":"running", "phase":"extract", "message":"", "done":2, "total":10, "speed":null,"revision":7,"canCancel":true,"result":null},
            "args":{"command":"sync","category":"cursor"}
        }]).to_string()).unwrap();
        let rows = load(&path).unwrap();
        assert_eq!(rows[0].snapshot.status, "failed");
        assert_eq!(rows[0].snapshot.revision, 8);
        assert!(rows[0].snapshot.can_retry);
        assert!(!rows[0].snapshot.can_cancel);
        crate::staging::remove(root).unwrap();
    }
}
