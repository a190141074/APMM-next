use std::fs::{self, OpenOptions};
use std::io::Write;

use serde_json::json;

/// Append a structured diagnostic record and rotate the active JSONL file.
/// The caller is responsible for keeping secrets and file contents out of the
/// message/detail arguments.
pub(crate) fn append(
    data_dir: std::path::PathBuf,
    timestamp: u64,
    level: &str,
    operation: &str,
    message: &str,
    detail: Option<&str>,
) {
    let dir = data_dir.join("logs");
    let _ = fs::create_dir_all(&dir);
    let active = dir.join("astral-mod-manager.jsonl");
    if active
        .metadata()
        .map(|m| m.len() > 1_048_576)
        .unwrap_or(false)
    {
        for index in (1..=4).rev() {
            let from = dir.join(format!("astral-mod-manager.{}.jsonl", index));
            let to = dir.join(format!("astral-mod-manager.{}.jsonl", index + 1));
            if from.exists() {
                let _ = fs::rename(from, to);
            }
        }
        let _ = fs::rename(&active, dir.join("astral-mod-manager.1.jsonl"));
    }
    let entry = json!({
        "timestamp": timestamp,
        "level": level,
        "operation": operation,
        "message": message,
        "detail": detail
    });
    if let Ok(line) = serde_json::to_string(&entry) {
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(active)
            .and_then(|mut file| writeln!(file, "{}", line));
    }
}
