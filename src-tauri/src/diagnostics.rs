use serde_json::{json, Value};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;

fn log_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            if !local_app_data.trim().is_empty() {
                return PathBuf::from(local_app_data).join("ZError").join("logs");
            }
        }
        let username = crate::database::get_username().unwrap_or_else(|_| "Administrator".to_string());
        return PathBuf::from(format!(
            "C:\\Users\\{}\\AppData\\Local\\ZError\\logs",
            username
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("zerror")
            .join("logs")
    }
}

/// 追加一条 JSONL 诊断日志。
/// 调用方需自行避免写入 API Key、Authorization 等敏感字段。
pub fn append_event(
    level: &str,
    scope: &str,
    event: &str,
    request_id: Option<&str>,
    data: Value,
) -> Result<(), String> {
    let dir = log_dir();
    create_dir_all(&dir).map_err(|error| format!("创建日志目录失败: {error}"))?;

    let now = chrono::Local::now();
    let file_path = dir.join(format!("app-{}.jsonl", now.format("%Y%m%d")));

    if let Ok(metadata) = std::fs::metadata(&file_path) {
        if metadata.len() >= MAX_LOG_BYTES {
            let rotated = dir.join(format!(
                "app-{}-{}.jsonl",
                now.format("%Y%m%d"),
                now.format("%H%M%S")
            ));
            let _ = std::fs::rename(&file_path, rotated);
        }
    }

    let line = json!({
        "ts": now.to_rfc3339(),
        "level": level,
        "scope": scope,
        "event": event,
        "request_id": request_id,
        "data": data,
    });

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .map_err(|error| format!("打开日志文件失败: {error}"))?;

    writeln!(
        file,
        "{}",
        serde_json::to_string(&line).map_err(|error| format!("序列化日志失败: {error}"))?
    )
    .map_err(|error| format!("写入日志失败: {error}"))?;

    Ok(())
}

#[tauri::command]
pub fn append_diagnostic_log(
    level: String,
    scope: String,
    event: String,
    request_id: Option<String>,
    data: Option<Value>,
) -> Result<(), String> {
    append_event(
        &level,
        &scope,
        &event,
        request_id.as_deref(),
        data.unwrap_or_else(|| json!({})),
    )
}

#[tauri::command]
pub fn get_log_dir() -> Result<String, String> {
    let dir = log_dir();
    create_dir_all(&dir).map_err(|error| format!("创建日志目录失败: {error}"))?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn open_log_dir() -> Result<(), String> {
    let dir = log_dir();
    create_dir_all(&dir).map_err(|error| format!("创建日志目录失败: {error}"))?;

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(dir.to_string_lossy().to_string())
            .spawn()
            .map_err(|error| format!("打开日志目录失败: {error}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(dir.to_string_lossy().to_string())
            .spawn()
            .map_err(|error| format!("打开日志目录失败: {error}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(dir.to_string_lossy().to_string())
            .spawn()
            .map_err(|error| format!("打开日志目录失败: {error}"))?;
    }

    Ok(())
}
