//! 迁移提交前持久化备份清单；重启恢复只处理本次事务拥有且内容未被用户改变的文件。
use crate::*;

#[derive(Serialize, Deserialize)]
struct FileChange {
    relative: PathBuf,
    before: Option<String>,
    after: String,
}

#[derive(Serialize, Deserialize)]
pub struct Journal {
    destination: PathBuf,
    stage: PathBuf,
    backup: PathBuf,
    status: String,
    files: Vec<FileChange>,
}

fn path(destination: &Path) -> PathBuf {
    destination.join("migration-pending.json")
}

fn regular_destination(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("迁移记录含非法相对路径".into());
    }
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        if let Ok(meta) = fs::symlink_metadata(&current) {
            let mut linked = meta.file_type().is_symlink();
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                linked |= meta.file_attributes() & 0x400 != 0;
            }
            if linked {
                return Err("迁移目标含链接，已保留恢复记录".into());
            }
        }
    }
    Ok(current)
}

impl Journal {
    pub fn begin(destination: &Path, stage: &Path, backup: &Path) -> Result<Self, String> {
        if path(destination).exists() {
            return Err("上次迁移尚未恢复，请重启后重试".into());
        }
        let journal = Self {
            destination: destination.to_path_buf(),
            stage: stage.to_path_buf(),
            backup: backup.to_path_buf(),
            status: "applying".into(),
            files: vec![],
        };
        journal.save()?;
        Ok(journal)
    }

    fn save(&self) -> Result<(), String> {
        crate::portable_update::write_json(&path(&self.destination), self)
    }

    pub fn copy(&mut self, relative: &Path) -> Result<(), String> {
        let target = regular_destination(&self.destination, relative)?;
        let source = self.stage.join("new").join(relative);
        let before = if target.exists() {
            let old = self.backup.join(relative);
            ensure_dir(old.parent().ok_or("备份路径无效")?).map_err(|e| e.to_string())?;
            fs::copy(&target, &old).map_err(|e| e.to_string())?;
            let hash = sha256_file(&target)?;
            if sha256_file(&old)? != hash {
                return Err("迁移备份校验失败".into());
            }
            fs::OpenOptions::new()
                .write(true)
                .open(&old)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            Some(hash)
        } else {
            None
        };
        let after = sha256_file(&source)?;
        self.files.push(FileChange {
            relative: relative.to_path_buf(),
            before,
            after: after.clone(),
        });
        self.save()?;
        ensure_dir(target.parent().ok_or("迁移目标路径无效")?).map_err(|e| e.to_string())?;
        // 原子替换使崩溃后的正式文件只可能是旧版或已校验的新版。
        let pending = target.with_file_name(format!(
            ".{}.{}.migration-pending",
            target.file_name().unwrap().to_string_lossy(),
            uuid::Uuid::new_v4()
        ));
        if pending.exists() {
            return Err(format!("迁移暂存文件已存在：{}", pending.display()));
        }
        fs::copy(&source, &pending).map_err(|e| e.to_string())?;
        let copied = (|| {
            if sha256_file(&pending)? != after {
                return Err("迁移提交校验失败".into());
            }
            fs::OpenOptions::new()
                .write(true)
                .open(&pending)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            fs::rename(&pending, &target).map_err(|e| e.to_string())
        })();
        if copied.is_err() {
            let _ = fs::remove_file(&pending);
        }
        copied
    }

    pub fn complete(&mut self) -> Result<(), String> {
        self.status = "complete".into();
        self.save()?;
        fs::remove_file(path(&self.destination)).map_err(|e| e.to_string())
    }

    fn rollback(&mut self) -> Result<(), String> {
        for record in self.files.iter().rev() {
            let target = regular_destination(&self.destination, &record.relative)?;
            let actual = if target.is_file() {
                Some(sha256_file(&target)?)
            } else {
                None
            };
            if actual == record.before {
                continue;
            }
            if actual.as_deref() != Some(&record.after) {
                return Err(format!(
                    "迁移后文件发生外部修改，需人工核对；备份：{}；文件：{}",
                    self.backup.display(),
                    target.display()
                ));
            }
            if let Some(hash) = &record.before {
                let old = self.backup.join(&record.relative);
                if sha256_file(&old)? != *hash {
                    return Err("迁移恢复备份校验失败".into());
                }
                let pending = target.with_file_name(format!(
                    ".{}.{}.migration-restore",
                    target.file_name().unwrap().to_string_lossy(),
                    uuid::Uuid::new_v4()
                ));
                fs::copy(old, &pending).map_err(|e| e.to_string())?;
                fs::OpenOptions::new()
                    .write(true)
                    .open(&pending)
                    .and_then(|f| f.sync_all())
                    .map_err(|e| e.to_string())?;
                fs::rename(pending, target).map_err(|e| e.to_string())?;
            } else {
                fs::remove_file(target).map_err(|e| e.to_string())?;
            }
        }
        self.status = "rolled-back".into();
        self.save()?;
        fs::remove_file(path(&self.destination)).map_err(|e| e.to_string())
    }
}

pub fn recover(destination: &Path) -> Result<bool, String> {
    if !path(destination).exists() {
        return Ok(false);
    }
    let mut journal: Journal =
        serde_json::from_slice(&fs::read(path(destination)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let destination = destination.canonicalize().map_err(|e| e.to_string())?;
    if journal
        .destination
        .canonicalize()
        .map_err(|e| e.to_string())?
        != destination
    {
        return Err("迁移恢复目录与当前数据目录不符".into());
    }
    let staging = crate::staging::root()
        .canonicalize()
        .map_err(|e| e.to_string())?;
    for p in [&journal.stage, &journal.backup] {
        let p = p.canonicalize().map_err(|e| e.to_string())?;
        if !p.starts_with(&staging)
            || p == staging
            || destination.starts_with(&p)
            || p.starts_with(&destination)
        {
            return Err("迁移恢复暂存边界无效".into());
        }
    }
    if journal.status == "complete" || journal.status == "rolled-back" {
        fs::remove_file(path(&destination)).map_err(|e| e.to_string())?;
    } else {
        journal.rollback()?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restart_restores_old_library_and_removes_only_committed_new_files() {
        let root = crate::staging::create("migration-crash-test").unwrap();
        let dest = root.join("app-data");
        let stage = crate::staging::create("migration-crash-stage").unwrap();
        let backup = crate::staging::create("migration-crash-backup").unwrap();
        ensure_dir(&dest).unwrap();
        ensure_dir(&stage.join("new")).unwrap();
        fs::write(dest.join("library.json"), b"old").unwrap();
        fs::write(stage.join("new/library.json"), b"new").unwrap();
        fs::write(stage.join("new/new.png"), b"png").unwrap();
        let mut j = Journal::begin(&dest, &stage, &backup).unwrap();
        j.copy(Path::new("new.png")).unwrap();
        j.copy(Path::new("library.json")).unwrap();
        drop(j);
        assert!(recover(&dest).unwrap());
        assert_eq!(fs::read(dest.join("library.json")).unwrap(), b"old");
        assert!(!dest.join("new.png").exists());
        assert!(!recover(&dest).unwrap());
        for p in [stage, backup, root] {
            crate::staging::remove(p).unwrap();
        }
    }
}
