//! MSI 使用 Windows Installer 事务；普通权限观察进程负责启动，提权进程只执行安装。
use crate::portable_update::{self as update, Job};
use std::{fs, path::Path, time::Duration};

#[cfg(windows)]
fn property(file: &Path, name: &str) -> Result<String, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::System::ApplicationInstallationAndServicing::*,
    };
    struct Handle(MSIHANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                MsiCloseHandle(self.0);
            }
        }
    }
    let wide: Vec<u16> = file.as_os_str().encode_wide().chain(Some(0)).collect();
    let sql: Vec<u16> = format!("SELECT `Value` FROM `Property` WHERE `Property` = '{name}'")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    unsafe {
        let mut database = MSIHANDLE::default();
        if MsiOpenDatabaseW(PCWSTR(wide.as_ptr()), MSIDBOPEN_READONLY, &mut database) != 0 {
            return Err("无法读取 MSI 数据库".into());
        }
        let database = Handle(database);
        let mut view = MSIHANDLE::default();
        if MsiDatabaseOpenViewW(database.0, PCWSTR(sql.as_ptr()), &mut view) != 0 {
            return Err("MSI 属性查询失败".into());
        }
        let view = Handle(view);
        if MsiViewExecute(view.0, MSIHANDLE::default()) != 0 {
            return Err("MSI 属性执行失败".into());
        }
        let mut record = MSIHANDLE::default();
        if MsiViewFetch(view.0, &mut record) != 0 {
            return Err(format!("MSI 缺少 {name}"));
        }
        let record = Handle(record);
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        if MsiRecordGetStringW(
            record.0,
            1,
            Some(PWSTR(buffer.as_mut_ptr())),
            Some(&mut length),
        ) != 0
        {
            return Err("MSI 属性过长或无效".into());
        }
        Ok(String::from_utf16_lossy(&buffer[..length as usize]))
    }
}

/// 同时校验下载摘要与安装产品身份；不允许把别的 MSI 当成 APMM 更新。
pub fn validate_package(path: &Path, bytes: u64, hash: &str, version: &str) -> Result<(), String> {
    if bytes == 0
        || fs::metadata(path).map_err(|e| e.to_string())?.len() != bytes
        || update::hash(path)? != hash
    {
        return Err("MSI 摘要或大小不匹配".into());
    }
    #[cfg(windows)]
    {
        if property(path, "ProductName")? != "APMM"
            || property(path, "ProductVersion")? != version
            || property(path, "UpgradeCode")?
                .trim_matches(['{', '}'])
                .to_ascii_uppercase()
                != crate::installation::UPGRADE_CODE
        {
            return Err("MSI 产品、版本或升级标识不匹配".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = version;
        Err("当前系统不支持 MSI 安装".into())
    }
}

#[cfg(windows)]
pub fn elevated(path: &Path) -> Result<(), String> {
    use std::os::windows::fs::OpenOptionsExt;
    let job = update::load_job(path)?;
    if job.kind != "msi" {
        return Err("安装类型不是 MSI".into());
    }
    let package = job.installer.as_ref().ok_or("缺少 MSI 安装记录")?;
    let installer = job.source.join("installer.msi");
    // 保持只读共享句柄，验证后不允许另一进程改写即将安装的包。
    let _lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&installer)
        .map_err(|e| e.to_string())?;
    validate_package(&installer, package.bytes, &package.sha256, &job.version)?;
    let parent = path.parent().ok_or("缺少更新目录")?;
    update::write_json(
        &parent.join("elevation-ready.json"),
        &serde_json::json!({"id":job.id,"ready":true}),
    )?;
    crate::helper_process::wait_pid(job.pid)?;
    let status = std::process::Command::new("msiexec.exe")
        .arg("/i")
        .arg(&installer)
        .args(["/passive", "/norestart", "/l*v"])
        .arg(parent.join("installer.log"))
        .arg(format!("INSTALLDIR={}", job.destination.display()))
        // MSI 条件以属性是否为空判断，字符串 0 仍为真；必须显式清空自动启动属性。
        .args(["AUTOLAUNCHAPP=", "WIXUI_EXITDIALOGOPTIONALCHECKBOX="])
        .status()
        .map_err(|e| e.to_string())?;
    let code = status.code().unwrap_or(-1);
    update::write_json(
        &parent.join("installer-result.json"),
        &serde_json::json!({"code":code}),
    )?;
    if !matches!(code, 0 | 3010) {
        return Err(format!("Windows Installer 返回 {code}，详见 installer.log"));
    }
    Ok(())
}

#[cfg(windows)]
pub fn observe(path: &Path, job: &mut Job) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::{w, PCWSTR},
        Win32::{
            Foundation::*,
            System::Threading::*,
            UI::{Shell::*, WindowsAndMessaging::SW_HIDE},
        },
    };
    let executable: Vec<u16> = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let args: Vec<u16> = format!("\"{}\" --msi-elevated", path.display())
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let parent = path.parent().ok_or("缺少更新目录")?;
    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: w!("runas"),
            lpFile: PCWSTR(executable.as_ptr()),
            lpParameters: PCWSTR(args.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };
        ShellExecuteExW(&mut info).map_err(|e| format!("未获得安装授权，程序仍保持打开：{e}"))?;
        struct Process(HANDLE);
        impl Drop for Process {
            fn drop(&mut self) {
                unsafe {
                    let _ = CloseHandle(self.0);
                }
            }
        }
        let process = Process(info.hProcess);
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            let ready = fs::read(parent.join("elevation-ready.json"))
                .ok()
                .and_then(|v| serde_json::from_slice::<serde_json::Value>(&v).ok())
                .is_some_and(|v| v["id"] == job.id && v["ready"] == true);
            if ready {
                break;
            }
            if WaitForSingleObject(process.0, 100) != WAIT_TIMEOUT
                || std::time::Instant::now() > deadline
            {
                return Err("安装准备失败，程序尚未退出，请查看更新日志".into());
            }
        }
        update::write_json(
            &parent.join(format!("ready-{}.json", job.id)),
            &serde_json::json!({"id":job.id,"ready":true}),
        )?;
        while WaitForSingleObject(process.0, 250) == WAIT_TIMEOUT {}
        let mut code = 0;
        GetExitCodeProcess(process.0, &mut code).map_err(|e| e.to_string())?;
        let result: serde_json::Value = fs::read(parent.join("installer-result.json"))
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default();
        let installer_success = code == 0 && matches!(result["code"].as_i64(), Some(0 | 3010));
        let installed = installer_success
            && crate::installation::kind_at(&job.destination) == "msi"
            && update::manifest(&job.destination, &job.version).is_ok();
        let success = installer_success && installed;
        job.status = if success {
            "awaiting-startup"
        } else {
            "installer-failed"
        }
        .into();
        job.message = if success {
            "MSI 已安装，正在等待工作区启动".into()
        } else if installer_success {
            "MSI 已执行，但安装后的文件校验未通过。请重新运行安装包修复。".into()
        } else {
            format!(
                "MSI 安装未完成（{}），请查看 {}",
                result["code"],
                parent.join("installer.log").display()
            )
        };
        update::write_json(path, job)?;
        if !success {
            update::write_json(&job.result_path(), job)?;
            if !installer_success && crate::helper_process::wait_pid(job.pid).is_ok() {
                let _ = crate::helper_process::launch(&job.destination);
            }
            return Err(job.message.clone());
        }
        // MSI 自行处理安装回滚；安装后的启动异常只记录诊断，不用便携文件覆盖程序。
        match crate::helper_process::launch_and_ack(job) {
            Ok(()) => {
                job.status = "complete".into();
                job.message = "新版工作区已成功启动".into();
            }
            Err(error) => {
                job.status = "launch-failed".into();
                job.message = format!("MSI 已安装，但自动启动未确认：{error}。可重新打开程序。");
            }
        }
        update::write_json(path, job)?;
        update::write_json(&job.result_path(), job)?;
    }
    Ok(())
}
