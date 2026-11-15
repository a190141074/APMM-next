//! 更新助手的进程等待与就绪握手；正常启动沿用未提权观察进程的权限。
use crate::portable_update::Job;
use std::{path::Path, process::Command, time::Duration};
pub fn launch(destination: &Path) -> Result<std::process::Child, String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        match Command::new(destination.join("astral-mod-manager.exe"))
            .current_dir(destination)
            .spawn()
        {
            Err(e)
                if matches!(e.raw_os_error(), Some(5 | 32 | 33))
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(300))
            }
            result => return result.map_err(|e| e.to_string()),
        }
    }
}
pub fn wait_pid(pid: u32) -> Result<(), String> {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::{Foundation::*, System::Threading::*};
        match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            Ok(handle) => {
                let result = WaitForSingleObject(handle, 60000);
                let _ = CloseHandle(handle);
                if result == WAIT_OBJECT_0 {
                    Ok(())
                } else {
                    Err("旧程序未退出，更新未替换文件".into())
                }
            }
            Err(e) if e.code().0 as u32 == 0x80070057 => Ok(()),
            Err(e) => Err(format!("无法检查旧进程状态：{e}")),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        Err("更新助手目前只支持 Windows".into())
    }
}
pub fn acknowledged(job: &Job) -> bool {
    std::fs::read(
        job.data()
            .join("updates")
            .join(format!("ack-{}.json", job.id)),
    )
    .ok()
    .and_then(|v| serde_json::from_slice::<serde_json::Value>(&v).ok())
    .is_some_and(|v| v["ready"] == true && v["version"] == job.version)
}
pub fn launch_and_ack(job: &Job) -> Result<(), String> {
    let ack = job
        .data()
        .join("updates")
        .join(format!("ack-{}.json", job.id));
    if ack.exists() {
        std::fs::remove_file(ack).map_err(|e| e.to_string())?;
    }
    let mut child = launch(&job.destination)?;
    for _ in 0..240 {
        if acknowledged(job) {
            return Ok(());
        }
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    // 只有便携版回滚需要先释放新进程，MSI 的已安装文件由 Installer 管理。
    if job.kind == "portable" {
        let _ = child.kill();
        let _ = child.wait();
    }
    Err("新版未完成启动确认".into())
}
