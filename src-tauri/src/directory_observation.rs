//! 扫描观察与库提交分离；错误保留为观察结果，不能把无法读取误判成删除。
use crate::file_verification::{Stamp, VerificationCache, VerificationStats, VerifiedFile};
use crate::*;
use std::collections::BTreeMap;

#[derive(Default, Clone)]
pub(crate) struct RootObservation {
    pub files: BTreeMap<PathBuf, Result<VerifiedFile, String>>,
    directories: BTreeMap<PathBuf, SystemTime>,
}

#[derive(Default, Clone)]
pub(crate) struct Observation {
    pub roots: BTreeMap<PathBuf, RootObservation>,
    pub stats: VerificationStats,
}

impl Observation {
    pub fn root(&self, path: &Path) -> Result<&RootObservation, String> {
        self.roots
            .get(path)
            .ok_or_else(|| format!("目录没有完整观察结果：{}", path.display()))
    }

    /// 完整列举只在启动、强制检查或通知失效时进行；普通通知刷新涉及的子树。
    pub fn collect(
        roots: &[PathBuf],
        cache: &mut VerificationCache,
        prior: Option<&Self>,
        dirty: &[PathBuf],
        force: bool,
        checkpoint: impl Fn() -> Result<(), String>,
        progress: impl Fn(usize, usize),
    ) -> Result<Self, String> {
        let mut result = prior.cloned().unwrap_or_default();
        result.stats = VerificationStats::default();
        result.roots.retain(|p, _| roots.contains(p));
        let mut pending = Vec::new();
        for root in roots {
            // 根消失或无权访问时整轮失败，保留资源库，不将旧候选清空。
            fs::read_dir(root).map_err(|e| format!("无法读取目录 {}：{e}", root.display()))?;
            let all = force || prior.is_none() || !result.roots.contains_key(root);
            let scopes = if all {
                vec![root.clone()]
            } else {
                dirty
                    .iter()
                    .filter(|p| p.starts_with(root))
                    .cloned()
                    .collect()
            };
            let entry = result.roots.entry(root.clone()).or_default();
            for scope in scopes {
                checkpoint()?;
                entry.files.retain(|p, _| !p.starts_with(&scope));
                entry.directories.retain(|p, _| !p.starts_with(&scope));
                // 新增或删除会改变父目录时间；提交复核也能发现枚举后才出现的文件。
                for ancestor in scope
                    .ancestors()
                    .skip(1)
                    .take_while(|p| p.starts_with(root))
                {
                    remember_directory(ancestor, &mut entry.directories)?;
                }
                match fs::symlink_metadata(&scope) {
                    Ok(meta) if meta.file_type().is_symlink() => {}
                    Ok(meta) if meta.is_dir() => {
                        let mut folders = vec![scope];
                        while let Some(folder) = folders.pop() {
                            checkpoint()?;
                            remember_directory(&folder, &mut entry.directories)?;
                            for item in fs::read_dir(&folder).map_err(|e| e.to_string())? {
                                let item = item.map_err(|e| e.to_string())?;
                                let kind = item.file_type().map_err(|e| e.to_string())?;
                                if kind.is_symlink() {
                                    continue;
                                }
                                if kind.is_dir() {
                                    folders.push(item.path());
                                } else if kind.is_file()
                                    && crate::directory_files::image_name(&item.path()).is_some()
                                {
                                    pending.push((root.clone(), item.path()));
                                }
                            }
                        }
                    }
                    Ok(meta)
                        if meta.is_file()
                            && crate::directory_files::image_name(&scope).is_some() =>
                    {
                        pending.push((root.clone(), scope));
                    }
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(format!("{}：{e}", scope.display())),
                }
            }
        }
        pending.sort();
        pending.dedup();
        let total = pending.len();
        for (i, (root, path)) in pending.into_iter().enumerate() {
            checkpoint()?;
            // 内容变化通知必须重读，即使外部工具恢复了时间戳；启动可复用元数据。
            let must_read = force || dirty.iter().any(|p| path.starts_with(p));
            let value = cache.verify(&path, true, must_read, &mut result.stats);
            result
                .roots
                .get_mut(&root)
                .unwrap()
                .files
                .insert(path, value);
            if i % 32 == 0 || i + 1 == total {
                progress(i + 1, total);
            }
        }
        Ok(result)
    }

    pub fn stable(&self) -> bool {
        self.roots.iter().all(|(root, files)| {
            root.is_dir()
                && files.directories.iter().all(|(path, time)| {
                    fs::symlink_metadata(path)
                        .ok()
                        .filter(|m| m.is_dir() && !m.file_type().is_symlink())
                        .and_then(|m| m.modified().ok())
                        .as_ref()
                        == Some(time)
                })
                && files.files.iter().all(|(path, result)| match result {
                    Ok(v) => Stamp::read(path).ok().as_ref() == Some(&v.stamp),
                    Err(_) => path.exists(),
                })
        })
    }

    pub fn trim_cache(&self, cache: &mut VerificationCache) {
        cache
            .entries
            .retain(|p, _| self.roots.values().any(|root| root.files.contains_key(p)));
    }
}

fn remember_directory(
    path: &Path,
    directories: &mut BTreeMap<PathBuf, SystemTime>,
) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{}：{e}", path.display()))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(format!("目录类型发生变化：{}", path.display()));
    }
    directories.insert(
        path.to_path_buf(),
        meta.modified().map_err(|e| e.to_string())?,
    );
    Ok(())
}

pub(crate) fn full(
    vault: &Path,
    game: &Path,
    checkpoint: impl Fn() -> Result<(), String>,
) -> Result<Observation, String> {
    let mut roots = vec![vault.to_path_buf()];
    if game.is_dir() {
        roots.push(game.to_path_buf());
    }
    let mut cache = VerificationCache::default();
    Observation::collect(&roots, &mut cache, None, &[], true, checkpoint, |_, _| {})
}
