//! Diagnostic logging and export commands.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{data_dir, ensure_dir, unix_now};

pub(crate) fn write_log(level: &str, operation: &str, message: &str, detail: Option<&str>) {
    crate::logs::append(data_dir(), unix_now(), level, operation, message, detail);
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogsStatus {
    pub directory: String,
    pub files: usize,
    pub bytes: u64,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn logs_status() -> LogsStatus {
    let dir = crate::logs_dir();
    let mut files = 0usize;
    let mut bytes = 0u64;
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    files += 1;
                    bytes += meta.len();
                }
            }
        }
    }
    LogsStatus {
        directory: dir.to_string_lossy().to_string(),
        files,
        bytes,
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn logs_reveal() -> Result<(), String> {
    let dir = crate::logs_dir();
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("无法打开日志目录：{}", e))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogsExportArgs {
    pub destination: String,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn logs_export(args: LogsExportArgs) -> Result<String, String> {
    let mut target = PathBuf::from(args.destination);
    if target
        .extension()
        .and_then(|x| x.to_str())
        .map(|x| !x.eq_ignore_ascii_case("zip"))
        .unwrap_or(true)
    {
        target.set_extension("zip");
    }
    let parent = target.parent().ok_or("诊断包路径无效")?;
    ensure_dir(parent).map_err(|e| e.to_string())?;
    let temp = target.with_extension("zip.part");
    let file = fs::File::create(&temp).map_err(|e| format!("创建诊断包失败：{}", e))?;
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let status = logs_status();
    let meta = serde_json::json!({"appVersion": env!("CARGO_PKG_VERSION"), "createdAt": unix_now(), "logFiles": status.files, "logBytes": status.bytes, "note": "Contains diagnostic logs only; no passwords or Mod files are included."});
    archive
        .start_file("diagnostics.json", options)
        .map_err(|e| e.to_string())?;
    archive
        .write_all(&serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if let Ok(entries) = fs::read_dir(crate::logs_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if !path.is_file() || !name.ends_with(".jsonl") {
                continue;
            }
            archive
                .start_file(format!("logs/{}", name), options)
                .map_err(|e| e.to_string())?;
            archive
                .write_all(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
    }
    archive.finish().map_err(|e| e.to_string())?;
    if let Err(error) = fs::rename(&temp, &target) {
        let _ = fs::remove_file(&temp);
        return Err(format!("保存诊断包失败：{}", error));
    }
    write_log("info", "logs.export", "已导出诊断包", None);
    Ok(target.to_string_lossy().to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn log_client_error(
    operation: String,
    message: String,
    stack: Option<String>,
) -> Result<(), String> {
    write_log("error", &operation, &message, stack.as_deref());
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn log_mutation(
    operation: String,
    success: bool,
    message: Option<String>,
) -> Result<(), String> {
    let safe_message = message
        .as_deref()
        .map(|text| text.chars().take(512).collect::<String>());
    if success {
        write_log("info", &operation, "操作完成", None);
    } else {
        let detail = format!("{:?}", std::backtrace::Backtrace::force_capture());
        write_log(
            "error",
            &operation,
            safe_message.as_deref().unwrap_or("操作失败"),
            Some(&detail),
        );
    }
    Ok(())
}
