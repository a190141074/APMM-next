//! 封面替换先生成有界预览副本，再按草稿修订原子提交，附图顺序保持不变。
use super::*;

/// 读取并校验预览源图，生成独立 PNG 副本，不改写源文件。
pub(super) fn prepare(path: &Path) -> Result<AuthorFile, String> {
    if fs::metadata(path).map_err(|_| "图片不存在")?.len() > 32 * 1024 * 1024 {
        return Err("预览源图过大".into());
    }
    let bytes = fs::read(path).map_err(|_| "图片读取失败")?;
    crate::workshop::validate_png(&bytes)?;
    let img = image::load_from_memory(&bytes)
        .map_err(|_| "图片解码失败")?
        .thumbnail(1280, 1280);
    let mut output = Cursor::new(Vec::new());
    img.write_to(&mut output, image::ImageFormat::Png)
        .map_err(|_| "预览生成失败")?;
    if output.get_ref().len() > 2 * 1024 * 1024 {
        return Err("预览图超过 2 MiB，请选用更小图片".into());
    }
    sources::file("preview.png".into(), String::new(), output.get_ref())
}

/// 图片有效且修订未变化才替换首张封面；失败时保留已保存的完整图片清单。
#[tauri::command]
pub(crate) async fn author_set_cover(
    id: String,
    revision: u64,
    path: String,
) -> Result<AuthorDraft, String> {
    checked_id(&id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let image = prepare(Path::new(&path))?;
        let _guard = lock()?;
        let mut state = load()?;
        let draft = draft_mut(&mut state, &id)?;
        if draft.revision != revision {
            return Err("草稿已更新，请重新载入后更换封面".into());
        }
        if draft.images.is_empty() {
            draft.images.push(image);
        } else {
            draft.images[0] = image;
        }
        touch(draft);
        let result = draft.clone();
        save(&state)?;
        Ok(result)
    })
    .await
    .map_err(|_| "封面处理任务中断")?
}
