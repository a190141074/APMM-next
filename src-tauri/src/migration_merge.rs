//! 非空库只补齐旧版候选，保留现有应用、设置和预设；提交复用迁移备份日志。
use crate::*;
use serde_json::{json, Value};

struct Candidate {
    monster: bool,
    id: String,
    section: String,
    canonical: String,
    file: FileEntry,
}

pub(super) fn has_content(lib: &Library) -> bool {
    !lib.heroes.is_empty()
        || !lib.monsters.is_empty()
        || !lib.workshop.is_empty()
        || !lib.settings.presets.is_empty()
}

fn missing(source: &Library, target: &Library) -> Vec<Candidate> {
    let mut result = Vec::new();
    for (monster, old, current) in [
        (false, &source.heroes, &target.heroes),
        (true, &source.monsters, &target.monsters),
    ] {
        for (id, entity) in old {
            for (section, entry) in &entity.sections {
                for (canonical, resource) in &entry.resources {
                    let existing = current
                        .get(id)
                        .and_then(|h| h.sections.get(section))
                        .and_then(|s| s.resources.get(canonical));
                    let mut seen = existing
                        .into_iter()
                        .flat_map(|r| r.files.iter().map(|f| f.hash.clone()))
                        .collect::<HashSet<_>>();
                    for file in &resource.files {
                        if seen.insert(file.hash.clone()) {
                            result.push(Candidate {
                                monster,
                                id: id.clone(),
                                section: section.clone(),
                                canonical: canonical.clone(),
                                file: file.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    result.sort_by(|a, b| {
        (&a.monster, &a.id, &a.section, &a.canonical, &a.file.hash).cmp(&(
            &b.monster,
            &b.id,
            &b.section,
            &b.canonical,
            &b.file.hash,
        ))
    });
    result
}

pub(super) fn missing_count(source: &Library, target: &Library) -> usize {
    missing(source, target).len()
}

/// 调用者持有库写入锁。逐槽位去重，先核对全部载荷，再按载荷、库、标记顺序提交。
pub(super) fn copy_to(
    root: &Path,
    destination: &Path,
    source_raw: &[u8],
    target_raw: &[u8],
    cancel: &AtomicBool,
    progress: impl Fn(usize, usize, &str),
) -> Result<Value, String> {
    let checkpoint = || {
        if cancel.load(Ordering::Acquire) {
            Err("迁移已取消".to_string())
        } else {
            Ok(())
        }
    };
    checkpoint()?;
    let source: Library = serde_json::from_slice(source_raw).map_err(|e| e.to_string())?;
    let mut target: Library = serde_json::from_slice(target_raw).map_err(|e| e.to_string())?;
    let candidates = missing(&source, &target);
    let count = candidates.len();
    if count == 0 {
        return Ok(
            json!({"source":root,"mode":"merge","modsAdded":0,"files":0,"gameModsApplied":false}),
        );
    }
    // 摘要来自实际解析的同一份字节，不能在解析后另读文件作为基准。
    let source_hash = format!("{:x}", Sha256::digest(source_raw));
    let target_hash = format!("{:x}", Sha256::digest(target_raw));
    let source_vault = root
        .join("vault")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !source_vault.starts_with(root) {
        return Err("旧版 Vault 不在所选数据目录内".into());
    }
    let stage = crate::staging::create("migration-merge")?;
    let backup = crate::staging::create("migration-backup")?;
    let mut journal = crate::migration_journal::Journal::begin(destination, &stage, &backup)?;
    let result = (|| {
        let mut payloads = Vec::new();
        for (index, candidate) in candidates.into_iter().enumerate() {
            checkpoint()?;
            let mut file = candidate.file;
            if !is_png_mod_ext(&file.ext) {
                return Err(format!("旧版候选不是受支持的 PNG：{}", file.source_name));
            }
            let old = vault_path(&source_vault, &file)
                .canonicalize()
                .map_err(|e| format!("旧版候选缺失：{}：{e}", file.source_name))?;
            if !old.starts_with(&source_vault) || sha256_file(&old)? != file.hash {
                return Err(format!("旧版候选路径或摘要异常：{}", file.source_name));
            }
            // 为每个槽位分配独立文件，原文件名只用于显示，不能覆盖已有候选。
            let category = if candidate.monster {
                "monsters".into()
            } else {
                category_of(&candidate.section)
            };
            let name = format!(
                "{}_{}{}",
                sanitize_name(&candidate.canonical),
                uuid::Uuid::new_v4(),
                file.ext
            );
            let (staged, relative) = place_in_vault(
                &stage.join("new/vault"),
                &category,
                &candidate.id,
                &candidate.section,
                &name,
                &file.hash,
                &file.ext,
            )?;
            let relative_path = Path::new("vault").join(&relative);
            if destination.join(&relative_path).exists() {
                return Err("候选目标已存在，请重试".into());
            }
            fs::copy(&old, &staged).map_err(|e| e.to_string())?;
            if sha256_file(&staged)? != file.hash || sha256_file(&old)? != file.hash {
                return Err("复制期间旧版候选发生变化，请关闭旧版后重试".into());
            }
            payloads.push((old, file.hash.clone(), relative_path));
            file.rel_path = relative;
            file.fs_identity.clear();
            let map = if candidate.monster {
                &mut target.monsters
            } else {
                &mut target.heroes
            };
            map.entry(candidate.id)
                .or_default()
                .sections
                .entry(candidate.section)
                .or_default()
                .resources
                .entry(candidate.canonical)
                .or_default()
                .files
                .push(file);
            progress(index + 1, count, &staged.to_string_lossy());
        }
        checkpoint()?;
        if sha256_file(&root.join("library.json"))? != source_hash
            || sha256_file(&destination.join("library.json"))? != target_hash
        {
            return Err("迁移期间资源库发生变化，请重试".into());
        }
        for (path, hash, _) in &payloads {
            if sha256_file(path)? != *hash {
                return Err("迁移期间旧版候选发生变化，请重试".into());
            }
        }
        let report = json!({"source":root,"mode":"merge","modsAdded":count,"files":count,
            "completedAt":unix_now(),"gameModsApplied":false});
        ensure_dir(&stage.join("new")).map_err(|e| e.to_string())?;
        fs::write(
            stage.join("new/library.json"),
            serde_json::to_vec_pretty(&target).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::write(stage.join("new/migration.json"), report.to_string())
            .map_err(|e| e.to_string())?;
        for (_, _, relative) in &payloads {
            checkpoint()?;
            journal.copy(relative)?;
        }
        checkpoint()?;
        journal.copy(Path::new("library.json"))?;
        journal.copy(Path::new("migration.json"))?;
        checkpoint()?;
        journal.complete()?;
        Ok(report)
    })();
    if result.is_err() {
        crate::migration_journal::recover(destination)
            .map_err(|e| format!("候选补齐失败且恢复未完成：{e}；备份：{}", backup.display()))?;
    }
    crate::staging::remove(stage)?;
    crate::staging::remove(backup)?;
    result
}
