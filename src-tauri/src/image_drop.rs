//! 拖入只读名称匹配与首个候选应用；调用方在整个导入和应用期间持有库写入锁。
use crate::management_types::ResourceSlot;
use crate::*;
use serde_json::Value;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct ImageDropMatch {
    pub path: String,
    pub target: Option<ResourceSlot>,
}

/// 悬停只解析文件名，不打开、解码或复制图片。
#[tauri::command]
pub fn images_match_drop(paths: Vec<String>) -> Vec<ImageDropMatch> {
    paths
        .into_iter()
        .map(|path| {
            let name = Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let target = name
                .to_ascii_lowercase()
                .ends_with(".png")
                .then(|| parse_filename(name))
                .flatten()
                .map(|p| ResourceSlot::from_parsed(&p));
            ImageDropMatch { path, target }
        })
        .collect()
}

/// 名称规则优先于落点；仅拖入允许定向副本使用 canonical 名称。
pub(super) fn resolve(
    name: &str,
    hash: &str,
    target: Option<&Value>,
    drop_mode: bool,
) -> Result<(Parsed, bool), String> {
    if drop_mode {
        if let Some(parsed) = parse_filename(name) {
            return Ok((parsed, false));
        }
        if target.is_none() {
            return Err("无法识别部位，请选择归属或临时资源".into());
        }
    }
    let parsed = crate::image_import::parsed_target(name, hash, target)?;
    Ok((parsed, target.is_some()))
}

/// 入库提交之后才应用，失败恢复游戏和应用状态，已经入库的候选仍然保留。
pub(super) fn apply_first(
    lib: &mut Library,
    vault: &Path,
    result: &mut ImportResult,
    commit: &impl Fn(&Library) -> Result<(), String>,
) {
    let applied = (|| {
        if lib.settings.modimg_path.is_empty() {
            return Err("请先在设置中选择游戏目录".to_string());
        }
        let canonical = result.canonical.as_deref().ok_or("缺少应用槽位")?;
        let parsed = parse_filename(canonical).ok_or("不支持的应用槽位")?;
        let category = category_of(&parsed.section_key);
        let resource = lib_map(lib, &category)
            .get(&parsed.hero_id)
            .and_then(|h| h.sections.get(&parsed.section_key))
            .and_then(|s| s.resources.get(canonical))
            .ok_or("资源不存在")?;
        if resource.disabled {
            return Err("请先启用被禁用的资源".into());
        }
        let entry = resource
            .files
            .iter()
            .find(|f| Some(&f.hash) == result.hash.as_ref())
            .ok_or("候选不存在")?;
        let root = vault.canonicalize().map_err(|e| e.to_string())?;
        let source = vault_path(&root, entry)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !source.starts_with(&root) || sha256_file(&source)? != entry.hash {
            return Err("Mod 文件路径或内容校验失败".into());
        }
        let mut next = lib.clone();
        let resource = get_resource(
            &mut next,
            &category,
            &parsed.hero_id,
            &parsed.section_key,
            canonical,
        );
        resource.active_hash = result.hash.clone();
        resource.disabled = false;
        crate::resource_transaction::apply_batch(
            Path::new(&lib.settings.modimg_path),
            &[(canonical.into(), Some(source))],
            || commit(&next),
        )?;
        *lib = next;
        Ok::<(), String>(())
    })();
    match applied {
        Ok(()) => result.applied = true,
        Err(error) => result.apply_error = Some(error),
    }
}

#[cfg(test)]
#[path = "image_drop_tests.rs"]
mod tests;
