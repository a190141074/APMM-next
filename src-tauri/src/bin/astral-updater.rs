#![cfg_attr(windows, windows_subsystem = "windows")]
//! 独立更新助手。便携更新使用文件事务，MSI 更新使用提权安装与普通权限观察。
#[path = "../helper_process.rs"]
mod helper_process;
#[path = "../installation.rs"]
mod installation;
#[path = "../msi_update.rs"]
mod msi_update;
#[path = "../portable_update.rs"]
mod portable_update;
#[path = "../staging.rs"]
mod staging;
use portable_update as update;
use std::path::Path;

fn execute(path: &Path, recover: bool) -> Result<(), String> {
    let mut job = update::load_job(path)?;
    if job.kind == "msi" {
        #[cfg(windows)]
        {
            return msi_update::observe(path, &mut job);
        }
        #[cfg(not(windows))]
        {
            return Err("当前系统不支持 MSI".into());
        }
    }
    if !recover {
        update::write_json(
            &path
                .parent()
                .unwrap()
                .join(format!("ready-{}.json", job.id)),
            &serde_json::json!({"id":job.id,"ready":true}),
        )?;
    }
    helper_process::wait_pid(job.pid)?;
    if recover {
        update::rollback(path, &mut job, "上次更新中断，恢复原版本")?;
        helper_process::launch(&job.destination)?;
        return Ok(());
    }
    if let Err(error) = update::backup(path, &mut job) {
        job.status = "failed-before-apply".into();
        job.message = error.clone();
        update::write_json(path, &job)?;
        update::write_json(&job.result_path(), &job)?;
        helper_process::launch(&job.destination)?;
        return Err(error);
    }
    if let Err(error) =
        update::apply(path, &mut job).and_then(|_| helper_process::launch_and_ack(&job))
    {
        update::rollback(path, &mut job, &error)?;
        helper_process::launch(&job.destination)?;
        return Err(error);
    }
    job.status = "complete".into();
    job.message = "新版工作区已成功启动".into();
    update::write_json(path, &job)?;
    update::write_json(&job.result_path(), &job)?;
    Ok(())
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        std::process::exit(2);
    }
    let path = Path::new(&args[1]);
    #[cfg(windows)]
    let result = if args.get(2).is_some_and(|s| s == "--msi-elevated") {
        msi_update::elevated(path)
    } else {
        execute(path, args.get(2).is_some_and(|s| s == "--recover"))
    };
    #[cfg(not(windows))]
    let result = execute(path, args.get(2).is_some_and(|s| s == "--recover"));
    if let Err(error) = result {
        let _ = std::fs::write(path.with_extension("error.log"), error);
        std::process::exit(2);
    }
}
