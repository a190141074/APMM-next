//! 成组资源的游戏文件事务；提交库失败时恢复整批，回滚失败则保留备份位置。
use crate::*;
pub fn apply_batch(
    modimg: &Path,
    items: &[(String, Option<PathBuf>)],
    commit: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let stage = crate::staging::create("resource-transaction")?;
    let names = items
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<HashSet<_>>();
    if names.len() != items.len() {
        crate::staging::remove(&stage)?;
        return Err("同一批次包含重复应用槽位".into());
    }
    let backups = backup_modimg_for_queue(modimg, &names, &stage)?;
    let result = (|| {
        for (name, source) in items {
            match source {
                Some(path) => {
                    write_entry_to_modimg(modimg, name, path, ".png")?;
                }
                None => purge_modimg(modimg, name)?,
            }
        }
        commit()
    })();
    if let Err(error) = result {
        if let Err(rollback) = restore_modimg_queue_backup(modimg, &names, &backups) {
            return Err(format!(
                "{error}；回滚未完成：{rollback}；备份保留在 {}",
                stage.display()
            ));
        }
        crate::staging::remove(stage)?;
        return Err(error);
    }
    crate::staging::remove(stage)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn group_rolls_back_when_second_file_or_library_commit_fails() {
        let root = crate::staging::create("group-test").unwrap();
        let game = root.join("modimg");
        ensure_dir(&game).unwrap();
        fs::write(game.join("cursor_up.png"), b"original-up").unwrap();
        fs::write(game.join("cursor_down.png.disabled"), b"original-down").unwrap();
        let source = root.join("new.png");
        fs::write(&source, b"replacement").unwrap();
        let bad = vec![
            ("cursor_up".into(), Some(source.clone())),
            ("cursor_down".into(), Some(root.join("missing"))),
        ];
        assert!(apply_batch(&game, &bad, || Ok(())).is_err());
        assert_eq!(
            fs::read(game.join("cursor_up.png")).unwrap(),
            b"original-up"
        );
        let items = vec![
            ("cursor_up".into(), Some(source)),
            ("cursor_down".into(), None),
        ];
        assert!(apply_batch(&game, &items, || Err("模拟库写入失败".into())).is_err());
        assert_eq!(
            fs::read(game.join("cursor_down.png.disabled")).unwrap(),
            b"original-down"
        );
        crate::staging::remove(root).unwrap();
    }
}
