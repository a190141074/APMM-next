//! 从旧版迁入的领域实现；保留协议与行为，在明确边界内逐步替换。
use crate::*;

/* ============ 游戏目录探测 ============ */

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DetectResult {
    pub(crate) status: String,
    pub(crate) game_path: String,
    pub(crate) modimg_path: String,
}

/// 收集常见 Steam 安装与 libraryfolders.vdf 声明的库，供游戏及平台目录匹配复用。
pub(crate) fn steam_libraries() -> Vec<PathBuf> {
    let mut seen: Vec<PathBuf> = Vec::new();
    for code in b'C'..=b'Z' {
        let root = format!("{}:\\", code as char);
        if !Path::new(&root).exists() {
            continue;
        }
        for sub in [
            "Steam\\steamapps",
            "SteamLibrary\\steamapps",
            "Program Files (x86)\\Steam\\steamapps",
            "Program Files\\Steam\\steamapps",
        ] {
            let p = Path::new(&root).join(sub);
            if p.exists() && !seen.contains(&p) {
                seen.push(p);
            }
        }
    }
    let extra: Vec<PathBuf> = seen
        .iter()
        .filter_map(|p| {
            let txt = fs::read_to_string(p.join("libraryfolders.vdf"))
                .or_else(|_| {
                    fs::read_to_string(
                        p.parent()
                            .unwrap_or(p)
                            .join("config")
                            .join("libraryfolders.vdf"),
                    )
                })
                .ok()?;
            let mut out = Vec::new();
            for cap in Regex::new(r#""path"\s*"([^"]+)""#)
                .unwrap()
                .captures_iter(&txt)
            {
                out.push(Path::new(&cap[1].replace("\\\\", "\\")).join("steamapps"));
            }
            Some(out)
        })
        .flatten()
        .collect();
    for p in extra {
        if !seen.contains(&p) {
            seen.push(p);
        }
    }

    seen
}

pub(crate) fn find_game() -> Option<(PathBuf, Vec<String>)> {
    for sa in steam_libraries() {
        let game_root = sa.join("common").join("Astral Party");
        if !game_root.exists() {
            continue;
        }
        let mut subs: Vec<String> = fs::read_dir(&game_root)
            .ok()?
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        if subs.is_empty() {
            continue;
        }
        subs.sort();
        let pick = subs
            .iter()
            .find(|n| n.eq_ignore_ascii_case("8vJXn6CN"))
            .or_else(|| subs.iter().find(|n| n.to_uppercase().ends_with("CN")))
            .cloned()
            .unwrap_or_else(|| subs[0].clone());
        return Some((game_root.join(pick), subs));
    }
    None
}

pub(crate) fn ensure_modimg(game_path: &Path) -> Result<(String, PathBuf), String> {
    let m = game_path.join("modimg");
    let mut status = "ok";
    if !m.exists() {
        fs::create_dir_all(&m).map_err(|e| e.to_string())?;
        status = "created";
    }
    Ok((status.to_string(), m))
}
