//! 按需安装与执行资产插件；已签名版本独立存放，运行时持有版本锁。
use crate::aux_manifest as manifest;
use crate::*;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{mpsc, OnceLock};
use tauri::Manager;

fn root() -> PathBuf {
    data_dir().join("auxiliary/asset-tool")
}
fn version_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
fn current() -> Result<PathBuf, String> {
    let pointer =
        fs::read_to_string(root().join("current")).map_err(|_| "请先安装 APMM 资产工具")?;
    if pointer.is_empty()
        || !pointer
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return Err("插件版本指针无效".into());
    }
    Ok(root().join("versions").join(pointer))
}
#[tauri::command]
pub fn auxiliary_status() -> Value {
    match current().and_then(|p| manifest::load(&p).map(|m| (p, m))) {
        Ok((p, m)) => {
            json!({"installed":true,"version":m.version,"bytes":m.files.iter().map(|f|f.bytes).sum::<u64>(),"path":p,"protocol":m.protocol,"capabilities":["extract","sync","check","inspect"],"message":"启动前校验签名与全部文件；提取成果独立保存。"})
        }
        Err(e) => json!({"installed":false,"message":e,"channel":manifest::config()["channel"]}),
    }
}
#[tauri::command]
pub async fn auxiliary_check_update() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(||{
        let cfg=manifest::config();let base=cfg["baseUrl"].as_str().ok_or("插件源未配置")?;
        let bytes=http_download(&format!("{base}/release.json"),2*1024*1024,None,"manifest",None,None,"检查插件更新")?;
        let release:Value=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
        let m=manifest::verify(release["manifest"].as_str().ok_or("缺少清单")?.as_bytes(),release["signature"].as_str().ok_or("缺少签名")?)?;
        Ok(json!({"availableVersion":m.version,"bytes":release["bytes"],"compatible":true,"protocol":m.protocol}))
    }).await.map_err(|e|e.to_string())?
}
pub async fn install(
    app: &tauri::AppHandle,
    id: &str,
    args: Value,
    _cancel: &AtomicBool,
) -> Result<Value, String> {
    let handle = app.clone();
    let id = id.to_string();
    let flag = app
        .state::<tasks::TaskManager>()
        .current()
        .ok_or("任务不存在")?
        .1;
    tauri::async_runtime::spawn_blocking(move || install_blocking(&handle, &id, args, &flag))
        .await
        .map_err(|e| e.to_string())?
}
fn install_blocking(
    app: &tauri::AppHandle,
    id: &str,
    args: Value,
    cancel: &AtomicBool,
) -> Result<Value, String> {
    let _lock = version_lock().lock().map_err(|_| "插件版本锁不可用")?;
    let archive = if let Some(path) = args["path"].as_str() {
        PathBuf::from(path)
    } else {
        let cfg = manifest::config();
        let base = cfg["baseUrl"].as_str().ok_or("插件源未配置")?;
        let raw = http_download(
            &format!("{base}/release.json"),
            2 * 1024 * 1024,
            None,
            "manifest",
            None,
            Some(cancel),
            "检查插件更新",
        )?;
        let release: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        let signed = release["manifest"].as_str().ok_or("插件源缺少签名清单")?;
        let m = manifest::verify(
            signed.as_bytes(),
            release["signature"].as_str().ok_or("插件源缺少签名")?,
        )?;
        let urls = release["urls"]
            .as_array()
            .ok_or("插件源缺少下载地址")?
            .iter()
            .map(|u| u.as_str().unwrap_or("").to_string())
            .collect::<Vec<_>>();
        if urls.iter().any(|u| !u.starts_with(&format!("{base}/"))) {
            return Err("插件下载地址不在受控来源内".into());
        }
        crate::downloads::fetch(
            crate::downloads::Download {
                identity: &format!("asset-tool:{}", m.version),
                urls,
                bytes: release["bytes"].as_u64().ok_or("缺少包大小")?,
                sha256: release["sha256"].as_str().ok_or("缺少包摘要")?,
            },
            cancel,
            |d, t, s| {
                app.state::<tasks::TaskManager>()
                    .download_progress(app, d, t, s)
            },
        )?
    };
    let stage = crate::staging::create("aux-install")?;
    let mut created = Vec::new();
    let result = (|| {
        let m = manifest::unpack(&archive, &stage, cancel)?;
        app.state::<tasks::TaskManager>().update(
            app,
            id,
            "install",
            "插件已通过签名及文件校验",
            0.,
            None,
        );
        let version = format!("{}-{}", m.version, uuid::Uuid::new_v4());
        let destination = root().join("versions").join(&version);
        ensure_dir(&destination).map_err(|e| e.to_string())?;
        for relative in m
            .files
            .iter()
            .map(|f| f.path.as_str())
            .chain(["plugin.json", "plugin.sig"])
        {
            if cancel.load(Ordering::Acquire) {
                return Err("插件安装已取消，当前版本未改变".into());
            }
            let target = destination.join(relative);
            ensure_dir(target.parent().unwrap()).map_err(|e| e.to_string())?;
            created.push(target.clone());
            fs::copy(stage.join(relative), target).map_err(|e| e.to_string())?;
        }
        manifest::validate_files(&destination, &m, cancel)?;
        crate::tasks::commit_phase(app)?;
        let pointer = root().join("current.new");
        fs::write(&pointer, version).map_err(|e| e.to_string())?;
        fs::rename(pointer, root().join("current")).map_err(|e| e.to_string())?;
        Ok(json!({"version":m.version,"installed":true,"path":destination}))
    })();
    if result.is_err() {
        for file in created.into_iter().rev() {
            if file.is_file() {
                fs::remove_file(&file)
                    .map_err(|e| format!("安装取消后清理未完成：{}：{e}", file.display()))?;
            }
        }
    }
    crate::staging::remove(stage)?;
    result
}
#[tauri::command]
pub fn auxiliary_uninstall() -> Result<Value, String> {
    let _guard = version_lock()
        .try_lock()
        .map_err(|_| "插件正在运行，请等待任务结束后卸载")?;
    let path = current()?;
    let m = manifest::load(&path)?;
    manifest::validate_files(&path, &m, &AtomicBool::new(false))?;
    fs::remove_file(root().join("current")).map_err(|e| e.to_string())?;
    // 仅删除签名清单拥有的文件，用户额外文件和提取成果不会被遍历清理。
    for relative in m
        .files
        .iter()
        .map(|f| f.path.as_str())
        .chain(["plugin.json", "plugin.sig"])
    {
        fs::remove_file(path.join(relative))
            .map_err(|e| format!("插件已停用，但部分文件未能删除：{e}"))?;
    }
    Ok(json!({"uninstalled":true,"assetsPreserved":true}))
}
pub async fn run(
    app: &tauri::AppHandle,
    id: &str,
    args: Value,
    _cancel: &AtomicBool,
) -> Result<Value, String> {
    let handle = app.clone();
    let task = id.to_string();
    let flag = app
        .state::<tasks::TaskManager>()
        .current()
        .ok_or("任务不存在")?
        .1;
    tauri::async_runtime::spawn_blocking(move || run_blocking(&handle, &task, args, &flag))
        .await
        .map_err(|e| e.to_string())?
}
fn run_blocking(
    app: &tauri::AppHandle,
    id: &str,
    args: Value,
    cancel: &AtomicBool,
) -> Result<Value, String> {
    let _guard = version_lock().lock().map_err(|_| "插件版本锁不可用")?;
    let path = current()?;
    let m = manifest::load(&path)?;
    manifest::validate_files(&path, &m, cancel)?;
    let command = args["command"].as_str().unwrap_or("check");
    if !matches!(command, "check" | "sync" | "extract" | "inspect") {
        return Err("不支持的资产命令".into());
    }
    let lib = load_library();
    let game = Path::new(&lib.settings.modimg_path)
        .parent()
        .filter(|p| p.is_dir())
        .ok_or("请先连接游戏目录")?;
    let output = data_dir().join("assets");
    let diagnostics = data_dir().join("logs");
    ensure_dir(&diagnostics).map_err(|e| e.to_string())?;
    let error_log =
        fs::File::create(diagnostics.join("asset-tool-last.log")).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(path.join(&m.entrypoint));
    cmd.current_dir(&path).arg(command);
    if command == "sync" {
        cmd.arg(args["category"].as_str().unwrap_or("all"));
    }
    if matches!(command, "extract" | "inspect") {
        cmd.arg(args["name"].as_str().ok_or("缺少资源名称")?);
    }
    cmd.arg("--game-dir")
        .arg(game)
        .arg("--output")
        .arg(output)
        .args(["--jsonl", "--yes"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(error_log);
    let platform = lib.settings.plugin_platform.as_deref().unwrap_or(crate::PLUGIN_DEFAULT_PLATFORM);
    // 资产工具连接的是工作区游戏；只借用同一个安装对应的平台缓存目录。
    let same_install = crate::apx_directories::platform_game_root(&lib.settings, platform)
        .and_then(|root| root.canonicalize().ok())
        .zip(game.canonicalize().ok())
        .is_some_and(|(platform_root, workspace_root)| platform_root == workspace_root);
    if same_install {
        if let Some(locallow) = crate::apx_directories::configured_directory(&lib.settings, platform, "locallow") {
            cmd.arg("--locallow-dir").arg(locallow);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.spawn().map_err(|e| format!("插件启动失败：{e}"))?;
    let mut stdin = child.stdin.take().ok_or("插件取消通道不可用")?;
    let stdout = child.stdout.take().ok_or("插件进度通道不可用")?;
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if send.send(line).is_err() {
                break;
            }
        }
    });
    let mut sent = false;
    let mut report = None;
    let mut protocol_error = None;
    loop {
        if cancel.load(Ordering::Acquire) && !sent {
            let _ = stdin.write_all(b"{\"protocol\":1,\"type\":\"cancel\"}\n");
            let _ = stdin.flush();
            sent = true;
            app.state::<tasks::TaskManager>().update(
                app,
                id,
                "cancel",
                "等待插件安全回滚",
                0.,
                None,
            );
        }
        match receive.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(line)) => match serde_json::from_str::<Value>(&line) {
                Ok(v) if v["protocol"] == 1 => {
                    if v["type"] == "result" {
                        report = Some(v["result"].clone());
                    } else if v["type"] == "progress" {
                        app.state::<tasks::TaskManager>().update(
                            app,
                            id,
                            "extract",
                            v["message"].as_str().unwrap_or("提取中"),
                            v["done"].as_f64().unwrap_or(0.),
                            v["total"].as_f64(),
                        );
                    }
                }
                _ => protocol_error = Some("插件返回了无效协议".to_string()),
            },
            Ok(Err(e)) => protocol_error = Some(e.to_string()),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let _ = reader.join();
    if let Some(error) = protocol_error {
        return Err(error);
    }
    let report = report.ok_or("插件未返回结果，请查看诊断日志")?;
    fs::write(
        diagnostics.join("asset-tool-last-result.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("资产工具未完整完成：{}", report));
    }
    Ok(report)
}
