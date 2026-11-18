//! 图片先独立暂存、校验再入库；拖入模式允许首个新候选自动应用。
use crate::*;
use serde_json::{json, Value};

/// 完整解码 PNG，拒绝伪装文件和只有文件头的损坏图片；内存受解码器默认限制约束。
pub(crate) fn validate_png(path: &Path) -> Result<(), String> {
    image::ImageReader::with_format(
        std::io::BufReader::new(fs::File::open(path).map_err(|e| e.to_string())?),
        image::ImageFormat::Png,
    )
    .decode()
    .map(|_| ())
    .map_err(|e| format!("PNG 内容损坏：{e}"))
}

pub(crate) fn parsed_target(
    name: &str,
    hash: &str,
    target: Option<&Value>,
) -> Result<Parsed, String> {
    if let Some(target) = target {
        if target["temporary"].as_bool() == Some(true) {
            return Ok(Parsed {
                hero_id: format!("temporary:{hash}"),
                section_key: "temporary".into(),
                canonical: format!("unmatched:{hash}"),
                source: name.into(),
            });
        }
        let cn = target["canonical"].as_str().ok_or("缺少目标资源名")?;
        let p = parse_filename(cn).ok_or("目标槽位无效")?;
        let id = target["heroId"]
            .as_str()
            .or_else(|| target["entityId"].as_str());
        if id != Some(p.hero_id.as_str())
            || target["sectionKey"].as_str() != Some(p.section_key.as_str())
        {
            return Err("目标身份不一致".into());
        }
        Ok(p)
    } else {
        Ok(parse_filename(name).unwrap_or_else(|| Parsed {
            hero_id: format!("temporary:{hash}"),
            section_key: "temporary".into(),
            canonical: format!("unmatched:{hash}"),
            source: name.into(),
        }))
    }
}

/// 每项导入先复制到独立暂存区并校验；库提交失败时删除本批新文件，保留已有候选。
pub(crate) fn import_selected(
    input: Value,
    state: tauri::State<AppState>,
) -> Result<Vec<ImportResult>, String> {
    let _guard = state.mutation.lock().map_err(|_| "库写入锁已损坏")?;
    import_into(input, load_library(), &vault_dir(), save_library)
}

/// 库和输出目录由调用方提供，隔离测试与正式导入共用完整提交链。
pub(super) fn import_into(
    input: Value,
    mut lib: Library,
    vault: &Path,
    commit: impl Fn(&Library) -> Result<(), String>,
) -> Result<Vec<ImportResult>, String> {
    let drop_mode = input["dropMode"].as_bool().unwrap_or(false);
    let paths: Vec<String> =
        serde_json::from_value(input["paths"].clone()).map_err(|e| e.to_string())?;
    let stage = staging::create("image-import")?;
    let mut created = Vec::new();
    let mut out = Vec::new();
    let mut first_indices = Vec::new();
    for (i, source) in paths.iter().enumerate() {
        let path = Path::new(source);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let result = (|| {
            if !name.to_ascii_lowercase().ends_with(".png") {
                return Err("仅支持 PNG 图片".to_string());
            }
            let staged = stage.join(format!("{i}.png"));
            fs::copy(path, &staged).map_err(|e| e.to_string())?;
            validate_png(&staged)?;
            let hash = sha256_file(&staged)?;
            if let Some(expected) = input["expectedHashes"].get(source).and_then(Value::as_str) {
                if expected != hash {
                    return Err("图片在预览后发生变化，请重新识别".into());
                }
            }
            let target = input["targets"]
                .get(source)
                .filter(|v| v.is_object())
                .or_else(|| input.get("target").filter(|v| v.is_object()));
            let (parsed, manual) = crate::image_drop::resolve(&name, &hash, target, drop_mode)?;
            let stored_name = if drop_mode && manual && parsed.section_key != "temporary" {
                format!("{}.png", parsed.canonical)
            } else {
                name.clone()
            };
            let category = category_of(&parsed.section_key);
            let resource = get_resource(
                &mut lib,
                &category,
                &parsed.hero_id,
                &parsed.section_key,
                &parsed.canonical,
            );
            let duplicate = resource.files.iter().any(|f| f.hash == hash);
            let first = drop_mode
                && !duplicate
                && resource.files.is_empty()
                && parsed.section_key != "temporary";
            let rel = if duplicate {
                resource
                    .files
                    .iter()
                    .find(|f| f.hash == hash)
                    .unwrap()
                    .rel_path
                    .clone()
            } else {
                let (dest, rel) = place_in_vault(
                    vault,
                    &category,
                    &parsed.hero_id,
                    &parsed.section_key,
                    &stored_name,
                    &hash,
                    ".png",
                )?;
                if !dest.exists() {
                    fs::copy(&staged, &dest).map_err(|e| e.to_string())?;
                    created.push(dest.clone());
                } else if sha256_file(&dest)? != hash {
                    return Err("资源库目标文件冲突".into());
                }
                resource.files.push(FileEntry {
                    fs_identity: crate::directory_files::file_identity(&dest).unwrap_or_default(),
                    hash: hash.clone(),
                    ext: ".png".into(),
                    source_name: stored_name,
                    rel_path: rel.clone(),
                    added_at: unix_now(),
                    classification: crate::management_types::ClassificationRecord {
                        manual,
                        original_name: name.clone(),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                rel
            };
            if first {
                first_indices.push(out.len());
            }
            Ok(ImportResult {
                file: name.clone(),
                ok: true,
                reason: None,
                duplicate,
                applied: false,
                apply_error: None,
                hero_id: Some(parsed.hero_id),
                section_key: Some(parsed.section_key),
                canonical: Some(parsed.canonical),
                hash: Some(hash),
                rel_path: Some(rel),
            })
        })();
        out.push(result.unwrap_or_else(|e| import_error(name, e)));
    }
    let saved = commit(&lib);
    if saved.is_err() {
        for path in created {
            let _ = fs::remove_file(path);
        }
    }
    let _ = staging::remove(stage);
    saved?;
    for index in first_indices {
        crate::image_drop::apply_first(&mut lib, vault, &mut out[index], &commit);
    }
    Ok(out)
}

#[tauri::command]
pub fn images_inspect(paths: Vec<String>, target: Option<Value>) -> Result<Vec<Value>, String> {
    let lib = load_library();
    paths.into_iter().map(|path|{
        let p=PathBuf::from(&path);let name=p.file_name().and_then(|s|s.to_str()).unwrap_or("");
        let result=(||{
            if !name.to_lowercase().ends_with(".png") {return Err("仅支持 PNG 图片".to_string())}
            let mut file=fs::File::open(&p).map_err(|e|e.to_string())?;
            let mut header=[0u8;24];file.read_exact(&mut header).map_err(|_|"PNG 文件头不完整")?;
            if &header[..8]!=b"\x89PNG\r\n\x1a\n" {return Err("文件内容不是 PNG".into())}
            validate_png(&p)?;
            let hash=sha256_file(&p)?;
            let parsed=parsed_target(name, &hash, target.as_ref())?;
            let res=lib_map(&lib,&category_of(&parsed.section_key)).get(&parsed.hero_id).and_then(|h|h.sections.get(&parsed.section_key)).and_then(|s|s.resources.get(&parsed.canonical));
            let duplicate=res.is_some_and(|r|r.files.iter().any(|f|f.hash==hash));
            let preview = if fs::metadata(&p).map_err(|e| e.to_string())?.len() <= 8 * 1024 * 1024 {
                Some(format!("data:image/png;base64,{}", base64_encode(&fs::read(&p).map_err(|e| e.to_string())?)))
            } else { None };
            Ok(json!({"id":format!("image:{}",sha256_bytes(path.as_bytes())),"path":path,"previewUrl":preview,"hash":hash,"status":if duplicate{"duplicate"}else{"new"},"conflict":res.is_some(),"meta":{"sourceName":name,"entityId":parsed.hero_id,"sectionKey":parsed.section_key,"canonical":parsed.canonical}}))
        })();
        Ok(result.unwrap_or_else(|error:String|json!({"id":path,"path":path,"status":"error","error":error,"meta":{"sourceName":name}})))
    }).collect()
}
