//! 从旧版迁入的领域实现；保留协议与行为，在明确边界内逐步替换。
use crate::*;

/* ============ 插件库（APX 补丁） ============ */

/// 插件发布仓库（CNB），按平台目录存放 manifest.json 与补丁文件。
pub(crate) const PLUGIN_REPO_RAW_PREFIX: &str =
    "https://cnb.cool/erinilis/apx-plugin-releases/-/git/raw/main/";
pub(crate) const PLUGIN_MANIFEST_MAX_BYTES: u64 = 8 * 1024 * 1024;
pub(crate) const PLUGIN_FILE_MAX_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const PLUGIN_MAX_FILES: usize = 512;
pub(crate) const PLUGIN_DEFAULT_PLATFORM: &str = "CN_STEAM";

/// (平台键, 显示名, LocalLow 数据目录名, Unity Data 目录名)
/// LocalLow 基址固定为 %USERPROFILE%\AppData\LocalLow\feimo。
/// 仅 Steam 客户端使用 AstralParty_CN 命名，其余渠道统一为「吉星派对」。
pub(crate) const PLUGIN_PLATFORMS: [(&str, &str, &str, &str); 6] = [
    (
        "CN_STEAM",
        "Steam 国服",
        "AstralParty_CN",
        "AstralParty_CN_Data",
    ),
    (
        "INT_STEAM",
        "Steam 国际服",
        "AstralParty_INT",
        "AstralParty_INT_Data",
    ),
    ("CN_TAPTAP", "TapTap", "吉星派对", "吉星派对_Data"),
    ("CN_BILIBILI", "Bilibili", "吉星派对", "吉星派对_Data"),
    ("CN_WEGAME", "WeGame", "吉星派对", "吉星派对_Data"),
    ("CN_ANDROID", "Android", "", ""),
];

pub(crate) fn plugin_is_android(platform: &str) -> bool {
    platform.eq_ignore_ascii_case("CN_ANDROID")
}

pub(crate) fn plugin_platform_info(
    key: &str,
) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    PLUGIN_PLATFORMS.iter().copied().find(|item| item.0 == key)
}

/// Steam 国服与国际服使用同一套 APX 文件；仅游戏目录名称不同。
pub(crate) fn plugin_repository_platform(platform: &str) -> &str {
    if platform.eq_ignore_ascii_case("INT_STEAM") {
        "CN_STEAM"
    } else {
        platform
    }
}

pub(crate) fn plugin_platform_options() -> Vec<PluginPlatformOption> {
    PLUGIN_PLATFORMS
        .iter()
        .map(|(key, label, locallow, data)| PluginPlatformOption {
            key: (*key).to_string(),
            label: (*label).to_string(),
            locallow_dir: (*locallow).to_string(),
            data_dir: (*data).to_string(),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginPlatformOption {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) locallow_dir: String,
    pub(crate) data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PluginFileEntry {
    #[serde(default)]
    pub(crate) relative_path: String,
    #[serde(default)]
    pub(crate) target_type: String,
    #[serde(default)]
    pub(crate) target_path: String,
    #[serde(default)]
    pub(crate) sha256: String,
    #[serde(default)]
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct PluginManifest {
    #[serde(default)]
    pub(crate) format_version: String,
    #[serde(default)]
    pub(crate) platform: String,
    #[serde(default)]
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) game_version: String,
    #[serde(default)]
    pub(crate) generated_at: String,
    #[serde(default)]
    pub(crate) total_patch_files: usize,
    #[serde(default)]
    pub(crate) total_patch_size: u64,
    #[serde(default)]
    pub(crate) total_restore_files: usize,
    #[serde(default)]
    pub(crate) total_restore_size: u64,
    #[serde(default)]
    pub(crate) files: Vec<PluginFileEntry>,
    #[serde(default)]
    pub(crate) restore_files: Vec<PluginFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginFileStatus {
    pub(crate) relative_path: String,
    pub(crate) target_type: String,
    pub(crate) description: String,
    pub(crate) size: u64,
    pub(crate) target: String,
    pub(crate) present: bool,
    pub(crate) installed: bool,
    pub(crate) expected_size: u64,
    pub(crate) actual_size: Option<u64>,
    pub(crate) sha256: String,
    pub(crate) validation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginStatus {
    pub(crate) platform: String,
    pub(crate) platform_label: String,
    pub(crate) platforms: Vec<PluginPlatformOption>,
    pub(crate) local_game_version: Option<String>,
    pub(crate) local_version_source: String,
    pub(crate) local_plugin_version: Option<String>,
    pub(crate) remote_version: Option<String>,
    pub(crate) remote_game_version: Option<String>,
    pub(crate) remote_generated_at: Option<String>,
    pub(crate) version_match: Option<bool>,
    pub(crate) warning: Option<String>,
    pub(crate) directory_error: Option<String>,
    pub(crate) directory_key: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) installed_files: usize,
    pub(crate) total_files: usize,
    pub(crate) total_bytes: u64,
    pub(crate) files: Vec<PluginFileStatus>,
    pub(crate) locallow_dir: String,
    pub(crate) game_data_dir: String,
    pub(crate) game_root_dir: String,
    pub(crate) cache_version: Option<String>,
    pub(crate) cache_complete: bool,
    pub(crate) cache_up_to_date: bool,
    pub(crate) cache_files: usize,
    pub(crate) cache_bytes: u64,
}

pub(crate) fn plugin_cache_dir(platform: &str) -> PathBuf {
    data_dir().join("plugins").join(platform)
}

pub(crate) fn apx_cache_root() -> PathBuf {
    data_dir().join("cache").join("apx")
}

pub(crate) fn apx_cache_version_dir(platform: &str, version: &str) -> PathBuf {
    apx_cache_root().join(platform).join(sanitize_name(version))
}

pub(crate) fn apx_cache_manifest_path(platform: &str, version: &str) -> PathBuf {
    apx_cache_version_dir(platform, version).join("manifest.json")
}

pub(crate) fn apx_cache_file_path(
    platform: &str,
    version: &str,
    kind: &str,
    entry: &PluginFileEntry,
) -> Result<PathBuf, String> {
    let relative = safe_relative_path(&entry.relative_path)
        .ok_or_else(|| format!("清单包含不安全的路径：{}", entry.relative_path))?;
    Ok(apx_cache_version_dir(platform, version)
        .join(kind)
        .join(relative))
}

pub(crate) fn plugin_manifest_cache_path(platform: &str) -> PathBuf {
    plugin_cache_dir(platform).join("manifest.json")
}

pub(crate) fn plugin_staging_dir(platform: &str) -> PathBuf {
    plugin_cache_dir(platform).join("staging")
}

pub(crate) fn plugin_backup_dir(platform: &str) -> PathBuf {
    plugin_cache_dir(platform).join("backup")
}

pub(crate) fn plugin_manifest_url(platform: &str) -> String {
    format!(
        "{}{}/manifest.json",
        PLUGIN_REPO_RAW_PREFIX,
        plugin_repository_platform(platform)
    )
}

pub(crate) fn plugin_file_url(platform: &str, relative_path: &str) -> String {
    format!(
        "{}{}/{}",
        PLUGIN_REPO_RAW_PREFIX,
        plugin_repository_platform(platform),
        encode_uri_path(relative_path)
    )
}

/// 仅保留 URL 路径安全字符，其余逐字节百分号编码。
pub(crate) fn encode_uri_path(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.replace('\\', "/").bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{:02X}", byte));
        }
    }
    out
}

/// 拒绝绝对路径、盘符与 `..`，只接受仓库内的相对路径。
pub(crate) fn safe_relative_path(raw: &str) -> Option<PathBuf> {
    // Some launchers/config exports leave a trailing HTML space entity or newline.
    // These are never meaningful in a Windows path and make an otherwise valid
    // game directory fail to resolve.
    let normalized = raw
        .replace("&#x20;", " ")
        .replace('\u{00a0}', " ")
        .trim()
        .replace('\\', "/");
    if normalized.trim().is_empty() || normalized.starts_with('/') || normalized.contains(':') {
        return None;
    }
    let mut out = PathBuf::new();
    for part in normalized.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return None;
        }
        out.push(part);
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

pub(crate) fn normalize_windows_path(raw: &str) -> PathBuf {
    PathBuf::from(raw.replace("&#x20;", " ").replace('\u{00a0}', " ").trim())
}

pub(crate) fn configured_plugin_dir(value: Option<String>) -> Option<PathBuf> {
    value
        .map(|path| normalize_windows_path(&path))
        .filter(|path| !path.as_os_str().is_empty())
}

pub(crate) fn locallow_root() -> Option<PathBuf> {
    let profile = std::env::var_os("USERPROFILE").map(PathBuf::from)?;
    let root = profile.join("AppData").join("LocalLow");
    if root.is_dir() {
        Some(root)
    } else {
        None
    }
}

pub(crate) fn locallow_game_dir(platform: &str) -> Option<PathBuf> {
    if plugin_is_android(platform) { return None; }
    if let Some(path) = crate::apx_directories::configured_directory(&load_library().settings, platform, "locallow") {
        return Some(path);
    }
    let (_, _, locallow, _) = plugin_platform_info(platform)?;
    Some(locallow_root()?.join("feimo").join(locallow))
}

/// APX 游戏根目录属于当前平台，不复用其他渠道的安装路径。
pub(crate) fn game_root_dir(platform: &str) -> Option<PathBuf> {
    crate::apx_directories::platform_game_root(&load_library().settings, platform)
}

pub(crate) fn game_data_dir(platform: &str) -> Option<PathBuf> {
    if plugin_is_android(platform) { return None; }
    if let Some(path) = crate::apx_directories::configured_directory(&load_library().settings, platform, "data") {
        return Some(path);
    }
    let (_, _, _, data) = plugin_platform_info(platform)?;
    Some(game_root_dir(platform)?.join(data))
}

pub(crate) fn plugin_directory_issue(platform: &str) -> Option<(String, String)> {
    if plugin_is_android(platform) {
        return None;
    }
    let checks = [
        ("locallow", "LocalLow 缓存目录", locallow_game_dir(platform)),
        ("data", "游戏 Data 目录", game_data_dir(platform)),
        ("root", "游戏根目录", game_root_dir(platform)),
    ];
    checks
        .into_iter()
        .find_map(|(key, label, path)| match path {
            Some(path) if path.is_dir() => None,
            Some(path) => Some((
                key.to_string(),
                format!("{}不存在：{}。请手动设置。", label, path.display()),
            )),
            None => Some((key.to_string(), format!("{}未设置，请手动设置。", label))),
        })
}

pub(crate) fn ensure_plugin_directories(platform: &str) -> Result<(), String> {
    match plugin_directory_issue(platform) {
        Some((_, message)) => Err(message),
        None => Ok(()),
    }
}

pub(crate) fn friendly_plugin_write_error(target: &Path, error: &std::io::Error) -> String {
    let detail = error.to_string();
    let denied = error.kind() == std::io::ErrorKind::PermissionDenied
        || detail.contains("os error 5")
        || detail.contains("WinError 5");
    if denied {
        format!(
            "无法写入 {}。请先关闭游戏后重试；如果仍失败，请确认当前账户对游戏目录有写入权限。（技术详情：{}）",
            target.display(), detail
        )
    } else {
        format!("写入 {} 失败：{}", target.display(), detail)
    }
}

/// `target_type == "game_root"` 中带 `Plugins/x86_64/` 或
/// `com.unity.addressables/` 的条目必须放进 Unity 的 `*_Data` 目录，
/// 其余依赖直接落在游戏根目录。
pub(crate) fn plugin_uses_unity_data_dir(target_path: &str) -> bool {
    let flat = target_path.replace('\\', "/");
    flat.starts_with("Plugins/")
        || flat.contains("/Plugins/")
        || flat == "com.unity.addressables"
        || flat.starts_with("com.unity.addressables/")
}

pub(crate) fn plugin_is_addressables_path(target_path: &str) -> bool {
    let flat = target_path.replace('\\', "/");
    flat == "com.unity.addressables" || flat.starts_with("com.unity.addressables/")
}

/// 新版游戏把 Addressables bundle 缓存放回 `*_Data`，但旧版清单仍可能
/// 把这些条目标记为 `locallow`。当本机已经出现新版目录时，兼容旧清单
/// 的落点；版本号仍由 LocalLow 下的 catalog 哈希读取。
pub(crate) fn plugin_addressables_relocated_to_data(
    platform: &str,
    target_type: &str,
    target_path: &str,
) -> bool {
    target_type.eq_ignore_ascii_case("locallow")
        && plugin_is_addressables_path(target_path)
        && game_data_dir(platform)
            .map(|base| base.join("com.unity.addressables").is_dir())
            .unwrap_or(false)
}

/// 解析一条清单记录的实际落点，返回 (绝对目标路径, 根标签, 根目录)。
pub(crate) fn plugin_target(
    platform: &str,
    entry: &PluginFileEntry,
) -> Option<(PathBuf, String, PathBuf)> {
    if plugin_is_android(platform) {
        return None;
    }
    let raw = if entry.target_path.trim().is_empty() {
        entry.relative_path.as_str()
    } else {
        entry.target_path.as_str()
    };
    let relative = safe_relative_path(raw)?;
    if plugin_addressables_relocated_to_data(platform, &entry.target_type, raw) {
        let base = game_data_dir(platform)?;
        return Some((base.join(relative), "data".into(), base));
    }
    match entry.target_type.as_str() {
        "locallow" => {
            let base = locallow_game_dir(platform)?;
            Some((base.join(relative), "locallow".into(), base))
        }
        "game_root" => {
            if plugin_uses_unity_data_dir(raw) {
                let base = game_data_dir(platform)?;
                Some((base.join(relative), "data".into(), base))
            } else {
                let base = game_root_dir(platform)?;
                Some((base.join(relative), "root".into(), base))
            }
        }
        _ => None,
    }
}

pub(crate) fn extract_version(text: &str) -> Option<String> {
    static CACHE: std::sync::OnceLock<Option<Regex>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| Regex::new(r"\d+\.\d+\.\d+").ok())
        .as_ref()?
        .find(text)
        .map(|m| m.as_str().to_string())
}

/// 本地游戏版本：取 com.unity.addressables 下最新的 catalog_*.hash，读取其内容。
pub(crate) fn local_game_version_for(platform: &str) -> (Option<String>, String) {
    let Some(base) = locallow_game_dir(platform) else {
        return (None, "无法定位 LocalLow 目录".into());
    };
    let dir = base.join("com.unity.addressables");
    let Ok(entries) = fs::read_dir(&dir) else {
        return (None, format!("目录不存在：{}", dir.display()));
    };
    let mut candidates: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        if !lower.starts_with("catalog_") || !lower.ends_with(".hash") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH);
        candidates.push((modified, entry.path()));
    }
    if candidates.is_empty() {
        return (None, format!("未找到 catalog_*.hash：{}", dir.display()));
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    let newest = &candidates[0].1;
    let name = newest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let content = fs::read_to_string(newest).unwrap_or_default();
    let head = content.lines().next().unwrap_or("").trim().to_string();
    let version = extract_version(&head).or_else(|| extract_version(&name));
    let source = match head.is_empty() {
        true => format!("{}（共 {} 个 catalog 哈希）", name, candidates.len()),
        false => format!("{} → {}（共 {} 个）", name, head, candidates.len()),
    };
    (version, source)
}

pub(crate) fn plugin_http_get(url: &str, max_bytes: u64, label: &str) -> Result<Vec<u8>, String> {
    let client = Client::builder()
        .redirect(Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(PREVIEW_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(PREVIEW_REQUEST_TIMEOUT_SECS))
        .user_agent("AstralPartyModManager/0.1")
        .build()
        .map_err(|e| format!("初始化网络组件失败：{}", e))?;
    let mut last_error = String::new();
    for attempt in 0..PREVIEW_SOURCE_ATTEMPTS {
        match client.get(url).send() {
            Ok(mut response) if response.status().is_success() => {
                let mut buffer = [0u8; 64 * 1024];
                let mut collected: Vec<u8> = Vec::new();
                loop {
                    let count = response
                        .read(&mut buffer)
                        .map_err(|e| format!("读取{}失败：{}", label, e))?;
                    if count == 0 {
                        break;
                    }
                    collected.extend_from_slice(&buffer[..count]);
                    if collected.len() as u64 > max_bytes {
                        return Err(format!("{}超过大小上限", label));
                    }
                }
                return Ok(collected);
            }
            Ok(response) => {
                last_error = format!("服务器返回 HTTP {}", response.status());
            }
            Err(error) => {
                last_error = format!("网络请求失败：{}", error);
            }
        }
        if attempt + 1 < PREVIEW_SOURCE_ATTEMPTS {
            std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
        }
    }
    Err(format!(
        "获取{}失败：{}",
        label,
        if last_error.is_empty() {
            "未知网络错误".to_string()
        } else {
            last_error
        }
    ))
}

pub(crate) fn read_cached_plugin_manifest(platform: &str) -> Option<PluginManifest> {
    let text = fs::read_to_string(plugin_manifest_cache_path(platform)).ok()?;
    serde_json::from_str(&text).ok()
}

pub(crate) fn fetch_plugin_manifest(platform: &str) -> Result<PluginManifest, String> {
    if plugin_platform_info(platform).is_none() {
        return Err(format!("不支持的客户端：{}", platform));
    }
    if platform.eq_ignore_ascii_case("INT_STEAM") {
        write_log(
            "info",
            "plugin.repository",
            "Steam 国际服使用国服插件清单仓库，客户端安装目录保持独立",
            None,
        );
    }
    let bytes = plugin_http_get(
        &plugin_manifest_url(platform),
        PLUGIN_MANIFEST_MAX_BYTES,
        "插件清单",
    )?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    let manifest: PluginManifest =
        serde_json::from_str(&text).map_err(|e| format!("插件清单格式无效：{}", e))?;
    if manifest.files.is_empty() {
        return Err("插件清单没有可部署文件".into());
    }
    if manifest.files.len() > PLUGIN_MAX_FILES || manifest.restore_files.len() > PLUGIN_MAX_FILES {
        return Err("插件清单文件数量超过上限".into());
    }
    // INT_STEAM intentionally reuses the CN_STEAM repository manifest; only
    // the game root/Data directory differs between the two clients.
    if manifest.format_version.is_empty() {
        return Err("插件清单缺少 format_version".into());
    }
    let declared_platform_ok = manifest.platform.is_empty()
        || manifest.platform.eq_ignore_ascii_case(platform)
        || (platform.eq_ignore_ascii_case("INT_STEAM")
            && manifest.platform.eq_ignore_ascii_case("CN_STEAM"));
    if !declared_platform_ok {
        return Err(format!(
            "插件清单平台不匹配：期望 {}，实际 {}",
            platform, manifest.platform
        ));
    }
    if manifest.total_patch_files > 0 && manifest.total_patch_files != manifest.files.len() {
        return Err(format!(
            "插件清单文件数量不一致：声明 {}，实际 {}",
            manifest.total_patch_files,
            manifest.files.len()
        ));
    }
    if manifest.total_restore_files > 0
        && manifest.total_restore_files != manifest.restore_files.len()
    {
        return Err(format!(
            "插件清单还原数量不一致：声明 {}，实际 {}",
            manifest.total_restore_files,
            manifest.restore_files.len()
        ));
    }
    let declared_bytes: u64 = manifest.files.iter().map(|item| item.size).sum();
    if manifest.total_patch_size > 0 && declared_bytes != manifest.total_patch_size {
        write_log(
            "warn",
            "plugin.manifest",
            &format!(
                "{} 清单体积与实际文件不一致：声明 {}，合计 {}",
                platform, manifest.total_patch_size, declared_bytes
            ),
            None,
        );
    }
    let declared_restore: u64 = manifest.restore_files.iter().map(|item| item.size).sum();
    if manifest.total_restore_size > 0 && declared_restore != manifest.total_restore_size {
        write_log(
            "warn",
            "plugin.manifest",
            &format!(
                "{} 清单还原体积与实际文件不一致：声明 {}，合计 {}",
                platform, manifest.total_restore_size, declared_restore
            ),
            None,
        );
    }
    let dir = plugin_cache_dir(platform);
    ensure_dir(&dir).map_err(|e| e.to_string())?;
    let _ = fs::write(plugin_manifest_cache_path(platform), text.as_bytes());
    Ok(manifest)
}

/// 本地已有缓存就不再访问网络，保证离线也能看到上次的清单状态。
pub(crate) fn load_plugin_manifest(
    platform: &str,
    refresh: bool,
) -> Result<PluginManifest, String> {
    if !refresh {
        if let Some(manifest) = read_cached_plugin_manifest(platform) {
            return Ok(manifest);
        }
    }
    fetch_plugin_manifest(platform)
}

pub(crate) fn validate_runtime_dependencies_json(
    bytes: &[u8],
    platform: &str,
) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| format!("runtime_dependencies.json JSON 无效：{}", e))?;
    let object = value
        .as_object()
        .ok_or_else(|| "runtime_dependencies.json 顶层必须是对象".to_string())?;
    for key in ["format_version", "main_assembly"] {
        if object
            .get(key)
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
            .is_none()
        {
            return Err(format!("runtime_dependencies.json 缺少 {}", key));
        }
    }
    if let Some(file_platform) = object.get("platform").and_then(|v| v.as_str()) {
        let platform_matches = file_platform.eq_ignore_ascii_case(platform)
            || (platform.eq_ignore_ascii_case("INT_STEAM")
                && file_platform.eq_ignore_ascii_case("CN_STEAM"));
        if !platform_matches {
            return Err(format!(
                "runtime_dependencies.json 平台不匹配：期望 {}，实际 {}",
                platform, file_platform
            ));
        }
    }
    for key in [
        "managed_assemblies_embedded_in_main_assembly",
        "managed_assemblies_embedded_in_assembly_bundle",
        "native_plugins_require_manual_deployment",
        "native_plugin_directories",
    ] {
        if let Some(items) = object.get(key) {
            let array = items
                .as_array()
                .ok_or_else(|| format!("runtime_dependencies.json 的 {} 必须是数组", key))?;
            for item in array {
                let path = item.as_str().ok_or_else(|| {
                    format!("runtime_dependencies.json 的 {} 含有非字符串条目", key)
                })?;
                if safe_relative_path(path).is_none() {
                    return Err(format!(
                        "runtime_dependencies.json 含有不安全路径：{}",
                        path
                    ));
                }
            }
        }
    }
    for key in ["native_plugin_directory", "main_assembly"] {
        if let Some(path) = object.get(key).and_then(|v| v.as_str()) {
            if safe_relative_path(path).is_none() {
                return Err(format!(
                    "runtime_dependencies.json 含有不安全路径：{}",
                    path
                ));
            }
        }
    }
    if let Some(driver) = object.get("dynamic_driver") {
        let driver = driver
            .as_object()
            .ok_or_else(|| "runtime_dependencies.json 的 dynamic_driver 必须是对象".to_string())?;
        for key in ["file", "source_sha256", "sha256"] {
            if driver
                .get(key)
                .and_then(|v| v.as_str())
                .filter(|v| !v.trim().is_empty())
                .is_none()
            {
                return Err(format!(
                    "runtime_dependencies.json 的 dynamic_driver 缺少 {}",
                    key
                ));
            }
        }
        if let Some(file) = driver.get("file").and_then(|v| v.as_str()) {
            if safe_relative_path(file).is_none() {
                return Err(format!(
                    "runtime_dependencies.json 含有不安全路径：{}",
                    file
                ));
            }
        }
        if driver.get("size").and_then(|v| v.as_u64()).is_none() {
            return Err("runtime_dependencies.json 的 dynamic_driver 缺少有效 size".into());
        }
    }
    Ok(())
}

pub(crate) fn runtime_dependencies_entry(entry: &PluginFileEntry) -> bool {
    entry
        .relative_path
        .replace('\\', "/")
        .eq_ignore_ascii_case("runtime_dependencies.json")
}

/// Runtime dependency metadata is useful for diagnostics, but is not a game
/// runtime artifact. Never make installation/export depend on its stale hash.
pub(crate) fn is_optional_plugin_metadata(entry: &PluginFileEntry) -> bool {
    runtime_dependencies_entry(entry)
}

pub(crate) fn cache_plugin_entry(
    platform: &str,
    version: &str,
    kind: &str,
    entry: &PluginFileEntry,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
) -> Result<PathBuf, String> {
    let target = apx_cache_file_path(platform, version, kind, entry)?;
    if crate::downloads::checked(&target, entry.size, &entry.sha256) {
        return Ok(target);
    }
    let url = plugin_file_url(platform, &entry.relative_path);
    let cached = crate::downloads::fetch(
        crate::downloads::Download {
            identity: &format!("apx:{platform}:{version}:{kind}:{}", entry.relative_path),
            urls: vec![url],
            bytes: entry.size,
            sha256: &entry.sha256,
        },
        cancel,
        |done, total, speed| {
            use tauri::Manager;
            app.state::<crate::tasks::TaskManager>()
                .download_progress(app, done, total, speed);
        },
    )?;
    ensure_dir(target.parent().ok_or("缓存目录无效")?).map_err(|e| e.to_string())?;
    let part = target.with_extension("part");
    fs::copy(cached, &part).map_err(|e| e.to_string())?;
    fs::rename(part, &target).map_err(|e| e.to_string())?;
    Ok(target)
}

pub(crate) fn prepare_plugin_cache(
    platform: &str,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
) -> Result<PluginManifest, String> {
    let mut manifest = load_plugin_manifest(platform, true)?;
    let version = manifest.version.trim();
    if version.is_empty() {
        return Err(format!("{} 清单缺少版本号", platform));
    }
    let total = manifest
        .files
        .iter()
        .filter(|e| !is_optional_plugin_metadata(e))
        .count()
        + manifest
            .restore_files
            .iter()
            .filter(|e| !is_optional_plugin_metadata(e))
            .count();
    let required_entries = manifest
        .files
        .iter()
        .filter(|e| !is_optional_plugin_metadata(e))
        .map(|e| ("files", e))
        .chain(
            manifest
                .restore_files
                .iter()
                .filter(|e| !is_optional_plugin_metadata(e))
                .map(|e| ("restore", e)),
        );
    for (index, (kind, entry)) in required_entries.enumerate() {
        emit_plugin_progress(
            app,
            "cache",
            Some((((index as u64) * 100) / (total.max(1) as u64)) as u8),
            &format!("正在缓存 {}：{}", platform, entry.relative_path),
            None,
            None,
        );
        cache_plugin_entry(platform, version, kind, entry, app, cancel)?;
    }
    manifest.total_patch_files = manifest.files.len();
    manifest.total_patch_size = manifest.files.iter().map(|item| item.size).sum();
    manifest.total_restore_files = manifest.restore_files.len();
    manifest.total_restore_size = manifest.restore_files.iter().map(|item| item.size).sum();
    let destination = apx_cache_manifest_path(platform, version);
    ensure_dir(
        destination
            .parent()
            .ok_or_else(|| "缓存路径无效".to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let serialized = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(destination, serialized).map_err(|e| format!("写入缓存清单失败：{}", e))?;
    Ok(manifest)
}

pub(crate) fn cache_manifest_is_complete(
    platform: &str,
    manifest: &PluginManifest,
) -> (bool, usize, u64) {
    let mut files = 0usize;
    let mut bytes = 0u64;
    for (kind, entries) in [
        ("files", &manifest.files),
        ("restore", &manifest.restore_files),
    ] {
        for entry in entries
            .iter()
            .filter(|entry| !is_optional_plugin_metadata(entry))
        {
            let path = match apx_cache_file_path(platform, &manifest.version, kind, entry) {
                Ok(path) => path,
                Err(_) => return (false, files, bytes),
            };
            let metadata = match fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => return (false, files, bytes),
            };
            if metadata.len() != entry.size
                || entry.sha256.is_empty()
                || !sha256_file(&path)
                    .map(|hash| hash.eq_ignore_ascii_case(&entry.sha256))
                    .unwrap_or(false)
            {
                return (false, files, bytes);
            }
            files += 1;
            bytes = bytes.saturating_add(metadata.len());
        }
    }
    (true, files, bytes)
}

pub(crate) fn latest_complete_cached_manifest(
    platform: &str,
) -> Option<(PluginManifest, usize, u64)> {
    let root = apx_cache_root().join(platform);
    let mut candidates = Vec::new();
    for entry in fs::read_dir(root).ok()?.flatten() {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        let manifest: PluginManifest = match fs::read(manifest_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        {
            Some(manifest) => manifest,
            None => continue,
        };
        let declared_platform_ok = manifest.platform.is_empty()
            || manifest.platform.eq_ignore_ascii_case(platform)
            || (platform.eq_ignore_ascii_case("INT_STEAM")
                && manifest.platform.eq_ignore_ascii_case("CN_STEAM"));
        if !declared_platform_ok {
            continue;
        }
        let (complete, files, bytes) = cache_manifest_is_complete(platform, &manifest);
        if complete {
            candidates.push((manifest, files, bytes));
        }
    }
    candidates.sort_by(|a, b| a.0.version.cmp(&b.0.version));
    candidates.pop()
}

pub(crate) fn prepare_all_plugin_caches(
    current: &str,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
) -> Result<PluginManifest, String> {
    let mut current_manifest = None;
    for (platform, _, _, _) in PLUGIN_PLATFORMS {
        match prepare_plugin_cache(platform, app, cancel) {
            Ok(manifest) if platform == current => current_manifest = Some(manifest),
            Ok(_) => {}
            Err(error) if platform == current => return Err(error),
            Err(error) => write_log(
                "warn",
                "apx.cache",
                &format!("{} 缓存未完成：{}", platform, error),
                None,
            ),
        }
    }
    current_manifest.ok_or_else(|| format!("{} 缓存准备失败", current))
}

pub(crate) fn plugin_status_inner(platform: &str) -> PluginStatus {
    let info = plugin_platform_info(platform).unwrap_or(PLUGIN_PLATFORMS[0]);
    let directory_issue = plugin_directory_issue(info.0);
    let (local_game_version, local_version_source) = if plugin_is_android(info.0) {
        (None, "Android 通过导出使用，无需本机游戏目录".into())
    } else {
        local_game_version_for(info.0)
    };
    let manifest = read_cached_plugin_manifest(info.0);
    let (cache_version, cache_complete, cache_files, cache_bytes) =
        latest_complete_cached_manifest(info.0)
            .map(|(manifest, files, bytes)| (Some(manifest.version), true, files, bytes))
            .unwrap_or((None, false, 0, 0));

    let mut files = Vec::new();
    let mut installed_files = 0usize;
    let mut total_bytes = 0u64;
    let mut remote_version = None;
    let mut remote_game_version = None;
    let mut remote_generated_at = None;

    if let Some(manifest) = &manifest {
        remote_version = Some(manifest.version.clone()).filter(|v| !v.is_empty());
        remote_game_version = Some(manifest.game_version.clone()).filter(|v| !v.is_empty());
        remote_generated_at = Some(manifest.generated_at.clone()).filter(|v| !v.is_empty());
        for entry in manifest
            .files
            .iter()
            .filter(|entry| !is_optional_plugin_metadata(entry))
        {
            total_bytes = total_bytes.saturating_add(entry.size);
            let (target_text, target_type, present, matches) = match plugin_target(info.0, entry) {
                Some((path, target_type, _)) => {
                    let present = path.is_file();
                    let matches = present
                        && path.metadata().is_ok_and(|meta| meta.len() == entry.size)
                        && !entry.sha256.is_empty()
                        && sha256_file(&path)
                            .map(|hash| hash.eq_ignore_ascii_case(&entry.sha256))
                            .unwrap_or(false);
                    (
                        path.to_string_lossy().to_string(),
                        target_type,
                        present,
                        matches,
                    )
                }
                None => (String::new(), entry.target_type.clone(), false, false),
            };
            if matches {
                installed_files += 1;
            }
            let actual_size = if target_text.is_empty() {
                None
            } else {
                Path::new(&target_text).metadata().ok().map(|m| m.len())
            };
            files.push(PluginFileStatus {
                relative_path: entry.relative_path.clone(),
                target_type,
                description: entry.description.clone(),
                size: entry.size,
                target: target_text,
                present,
                installed: matches,
                expected_size: entry.size,
                actual_size,
                sha256: entry.sha256.clone(),
                validation_error: if present && !matches {
                    Some("本地文件校验不一致".to_string())
                } else {
                    None
                },
            });
        }
    }

    let version_match = match (&local_game_version, &remote_game_version) {
        (Some(local), Some(remote)) if !remote.is_empty() => Some(local == remote),
        _ => None,
    };
    let warning = if let Some((_, message)) = &directory_issue {
        Some(message.clone())
    } else if plugin_is_android(info.0) {
        Some("Android 当前仅支持缓存与导出".into())
    } else if version_match == Some(false) {
        Some(format!(
            "服务器尚未更新：本地游戏为 {}，清单标注的游戏版本是 {}（补丁 {}）。继续安装可能导致游戏异常，请等待对应补丁。",
            local_game_version.clone().unwrap_or_default(),
            remote_game_version.clone().unwrap_or_default(),
            remote_version.clone().unwrap_or_default()
        ))
    } else {
        None
    };

    let local_plugin_version = if !files.is_empty() && installed_files == files.len() {
        remote_version.clone()
    } else {
        None
    };
    PluginStatus {
        platform: info.0.to_string(),
        platform_label: info.1.to_string(),
        platforms: plugin_platform_options(),
        local_game_version,
        local_version_source,
        local_plugin_version,
        remote_version: remote_version.clone(),
        remote_game_version,
        remote_generated_at,
        version_match,
        warning,
        directory_error: directory_issue.as_ref().map(|(_, message)| message.clone()),
        directory_key: directory_issue.map(|(key, _)| key),
        error: None,
        installed_files,
        total_files: files.len(),
        total_bytes,
        files,
        locallow_dir: locallow_game_dir(info.0)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        game_data_dir: game_data_dir(info.0)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        game_root_dir: game_root_dir(info.0)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        cache_up_to_date: cache_version
            .as_deref()
            .zip(remote_version.as_deref())
            .map(|(cached, remote)| cached == remote)
            .unwrap_or(false),
        cache_version,
        cache_complete,
        cache_files,
        cache_bytes,
    }
}

pub(crate) fn emit_plugin_progress(
    app: &tauri::AppHandle,
    phase: &str,
    percent: Option<u8>,
    message: &str,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
) {
    let _ = app.emit(
        "plugin-progress",
        serde_json::json!({
            "phase": phase,
            "percent": percent,
            "downloadedBytes": downloaded_bytes,
            "totalBytes": total_bytes,
            "message": message
        }),
    );
}

/// 事务式写入：先全部下载到 staging 并校验，再逐文件备份原文件后落盘；
/// 任一步失败即从备份回滚，避免留下半套补丁。
pub(crate) fn plugin_apply_entries(
    platform: &str,
    entries: &[PluginFileEntry],
    cache_version: &str,
    cache_kind: &str,
    force: bool,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
    action: &str,
) -> Result<usize, String> {
    struct Plan {
        entry: PluginFileEntry,
        target: PathBuf,
        backup_tag: String,
        relative: PathBuf,
    }

    let mut plan: Vec<Plan> = Vec::new();
    for entry in entries {
        let (target, backup_tag, _) = plugin_target(platform, entry)
            .ok_or_else(|| format!("无法解析目标路径：{}", entry.relative_path))?;
        let raw = if entry.target_path.trim().is_empty() {
            entry.relative_path.as_str()
        } else {
            entry.target_path.as_str()
        };
        let relative =
            safe_relative_path(raw).ok_or_else(|| format!("清单包含不安全的路径：{}", raw))?;
        if !force
            && target.is_file()
            && !entry.sha256.is_empty()
            && sha256_file(&target)
                .map(|hash| hash.eq_ignore_ascii_case(&entry.sha256))
                .unwrap_or(false)
        {
            continue;
        }
        plan.push(Plan {
            entry: entry.clone(),
            target,
            backup_tag,
            relative,
        });
    }

    let total = plan.len();
    if total == 0 {
        return Ok(0);
    }

    let stage_root = crate::staging::create("apx-apply")?;
    if stage_root.exists() {
        crate::staging::remove(&stage_root).map_err(|e| e.to_string())?;
    }
    ensure_dir(&stage_root).map_err(|e| e.to_string())?;

    let grand_total: u64 = plan.iter().map(|item| item.entry.size.max(1)).sum();
    let mut downloaded_total = 0u64;
    let mut staged: Vec<PathBuf> = Vec::with_capacity(total);

    for (index, item) in plan.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            let _ = crate::staging::remove(&stage_root);
            return Err("用户取消操作".into());
        }
        let percent =
            Some((((downloaded_total.min(grand_total)) * 100) / grand_total.max(1)) as u8);
        emit_plugin_progress(
            app,
            "download",
            percent,
            &format!(
                "正在下载并校验 {}（{}/{}）",
                item.entry.relative_path,
                index + 1,
                total
            ),
            Some(downloaded_total),
            Some(grand_total),
        );
        let cached = apx_cache_file_path(platform, cache_version, cache_kind, &item.entry)?;
        let cached_ok = cached.is_file()
            && item.entry.size == cached.metadata().map(|m| m.len()).unwrap_or_default()
            && !item.entry.sha256.is_empty()
            && sha256_file(&cached)
                .map(|hash| hash.eq_ignore_ascii_case(&item.entry.sha256))
                .unwrap_or(false);
        if cached_ok {
            let file = stage_root.join(format!("{:04}.bin", index));
            fs::copy(&cached, &file).map_err(|e| format!("读取 APX 缓存失败：{}", e))?;
            downloaded_total = downloaded_total.saturating_add(item.entry.size);
            staged.push(file);
            continue;
        }
        let _ = fs::remove_file(&cached);
        let _ = crate::staging::remove(&stage_root);
        return Err(format!(
            "APX 缓存缺失或校验失败：{}，请重新安装",
            item.entry.relative_path
        ));
    }

    emit_plugin_progress(
        app,
        "install",
        Some(100),
        &format!("正在{} {} 个文件", action, total),
        Some(downloaded_total),
        Some(grand_total),
    );

    crate::tasks::commit_phase(app)?;
    let backup_root = plugin_backup_dir(platform).join(uuid::Uuid::new_v4().to_string());
    let mut backups: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let commit = (|| -> Result<(), String> {
        for (index, item) in plan.iter().enumerate() {
            if let Some(parent) = item.target.parent() {
                ensure_dir(parent).map_err(|e| format!("创建目标目录失败：{}", e))?;
            }
            let backup = if item.target.is_file() {
                let path = backup_root.join(&item.backup_tag).join(&item.relative);
                if let Some(parent) = path.parent() {
                    ensure_dir(parent).map_err(|e| e.to_string())?;
                }
                fs::copy(&item.target, &path).map_err(|e| format!("备份原文件失败：{}", e))?;
                Some(path)
            } else {
                None
            };
            backups.push((item.target.clone(), backup));
            fs::copy(&staged[index], &item.target)
                .map_err(|e| friendly_plugin_write_error(&item.target, &e))?;
        }
        Ok(())
    })();

    if let Err(error) = commit {
        let mut rollback_errors = Vec::new();
        for (target, backup) in backups.iter().rev() {
            let restored = match backup {
                Some(path) => fs::copy(path, target).map(|_| ()),
                None => fs::remove_file(target),
            };
            if let Err(e) = restored {
                rollback_errors.push(format!("{}：{e}", target.display()));
            }
        }
        if !rollback_errors.is_empty() {
            return Err(format!(
                "{action}失败：{error}；回滚未完成：{}；备份位于 {}",
                rollback_errors.join("；"),
                backup_root.display()
            ));
        }
        let _ = crate::staging::remove(&stage_root);
        return Err(format!("{action}已回滚：{error}"));
    }
    let _ = crate::staging::remove(&stage_root);
    write_log(
        "info",
        "plugin.apply",
        &format!("{} {} 个文件（{}）", action, total, platform),
        None,
    );
    Ok(total)
}

pub(crate) fn plugin_install_blocking(
    platform: &str,
    force: bool,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
) -> Result<usize, String> {
    if plugin_is_android(platform) {
        return Err("Android 当前仅支持导出".into());
    }
    ensure_plugin_directories(platform)?;
    let manifest = prepare_all_plugin_caches(platform, app, cancel)?;
    let files: Vec<PluginFileEntry> = manifest
        .files
        .iter()
        .filter(|entry| !is_optional_plugin_metadata(entry))
        .cloned()
        .collect();
    plugin_apply_entries(
        platform,
        &files,
        &manifest.version,
        "files",
        force,
        app,
        cancel,
        "安装",
    )
}

pub(crate) fn plugin_restore_blocking(
    platform: &str,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
) -> Result<usize, String> {
    if plugin_is_android(platform) {
        return Err("Android 当前仅支持导出".into());
    }
    let manifest = prepare_plugin_cache(platform, app, cancel)?;
    if manifest.restore_files.is_empty() {
        return Err("该客户端清单没有提供还原文件".into());
    }
    let files: Vec<PluginFileEntry> = manifest
        .restore_files
        .iter()
        .filter(|entry| !is_optional_plugin_metadata(entry))
        .cloned()
        .collect();
    plugin_apply_entries(
        platform,
        &files,
        &manifest.version,
        "restore",
        true,
        app,
        cancel,
        "还原",
    )
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn plugin_export(
    platform: Option<String>,
    destination: String,
    use_cached: Option<bool>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let key = resolve_plugin_platform(platform.as_deref());
    let destination = PathBuf::from(destination);
    let cache_lock = state.apx_cache_busy.clone();
    let cancel = state.plugin_cancel.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = cache_lock.lock().map_err(|_| "APX 缓存锁已损坏".to_string())?;
        cancel.store(false, Ordering::Relaxed);
        let manifest = if use_cached.unwrap_or(false) {
            let manifest = latest_complete_cached_manifest(&key)
                .map(|(manifest, _, _)| manifest)
                .ok_or_else(|| "本地没有完整可导出的 APX 缓存".to_string())?;
            for (kind, entries) in [("files", &manifest.files), ("restore", &manifest.restore_files)] {
                for entry in entries {
                    if is_optional_plugin_metadata(entry) { continue; }
                    let source = apx_cache_file_path(&key, &manifest.version, kind, entry)?;
                    let bytes = fs::read(&source).map_err(|_| format!("本地缓存不完整：{}", entry.relative_path))?;
                    let strict = entry.size == bytes.len() as u64 && !entry.sha256.is_empty() && sha256_bytes(&bytes).eq_ignore_ascii_case(&entry.sha256);
                    let semantic = runtime_dependencies_entry(entry) && validate_runtime_dependencies_json(&bytes, &key).is_ok();
                    if !strict && !semantic { return Err(format!("本地缓存校验失败：{}", entry.relative_path)); }
                }
            }
            manifest
        } else {
            prepare_plugin_cache(&key, &app, &cancel)?
        };
        let root = destination.parent().ok_or_else(|| "导出路径无效".to_string())?;
        ensure_dir(root).map_err(|e| format!("创建导出目录失败：{}", e))?;
        if destination.exists() { return Err("目标导出文件已存在，请选择其他路径".into()); }
        let temp = destination.with_extension(format!("part-{}", std::process::id()));
        // Every error after this point must remove the partial archive.  Keep
        // the target untouched until the finished ZIP can be atomically moved.
        struct RemovePartial(PathBuf);
        impl Drop for RemovePartial {
            fn drop(&mut self) { let _ = fs::remove_file(&self.0); }
        }
        let cleanup = RemovePartial(temp.clone());
        let file = fs::File::create(&temp).map_err(|e| format!("创建导出文件失败：{}", e))?;
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let base = format!("APX/{}/{}/", key, sanitize_name(&manifest.version));
        archive.start_file(format!("{}manifest.json", base), options).map_err(|e| e.to_string())?;
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
        archive.write_all(&manifest_bytes).map_err(|e| e.to_string())?;
        let mut count = 0usize;
        for (kind, entries) in [("files", &manifest.files), ("restore", &manifest.restore_files)] {
            for entry in entries {
                if is_optional_plugin_metadata(entry) { continue; }
                let source = apx_cache_file_path(&key, &manifest.version, kind, entry)?;
                if !source.is_file() {
                    return Err(format!("缓存文件不存在：{}", entry.relative_path));
                }
                let cached_bytes = fs::read(&source).map_err(|e| format!("读取导出缓存失败：{}", e))?;
                let strict_ok = entry.size == cached_bytes.len() as u64
                    && !entry.sha256.is_empty()
                    && sha256_bytes(&cached_bytes).eq_ignore_ascii_case(&entry.sha256);
                let semantic_ok = runtime_dependencies_entry(entry)
                    && validate_runtime_dependencies_json(&cached_bytes, &key).is_ok();
                if !strict_ok && !semantic_ok {
                    return Err(format!("缓存文件校验失败：{}", entry.relative_path));
                }
                let relative = safe_relative_path(&entry.relative_path).ok_or_else(|| format!("清单包含不安全路径：{}", entry.relative_path))?;
                archive.start_file(format!("{}{}/{}", base, kind, relative.to_string_lossy().replace('\\', "/")), options).map_err(|e| e.to_string())?;
                let mut input = fs::File::open(source).map_err(|e| e.to_string())?;
                std::io::copy(&mut input, &mut archive).map_err(|e| e.to_string())?;
                count += 1;
            }
        }
        archive.finish().map_err(|e| e.to_string())?;
        fs::rename(&temp, &destination).map_err(|e| format!("提交导出文件失败：{}", e))?;
        std::mem::forget(cleanup);
        write_log("info", "plugin.export", &format!("导出 {} {} 个文件", key, count), None);
        Ok(serde_json::json!({ "path": destination.to_string_lossy(), "platform": key, "version": manifest.version, "files": count }))
    }).await.map_err(|e| format!("导出 APX 插件失败：{}", e))?
}

pub(crate) fn resolve_plugin_platform(requested: Option<&str>) -> String {
    let stored = load_library().settings.plugin_platform.unwrap_or_default();
    let candidate = requested
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| Some(stored).filter(|v| !v.is_empty()))
        .unwrap_or_else(|| PLUGIN_DEFAULT_PLATFORM.to_string());
    match plugin_platform_info(&candidate) {
        Some((key, _, _, _)) => key.to_string(),
        None => PLUGIN_DEFAULT_PLATFORM.to_string(),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn plugin_status(platform: Option<String>) -> Result<PluginStatus, String> {
    Ok(plugin_status_inner(&resolve_plugin_platform(
        platform.as_deref(),
    )))
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn plugin_check_update_blocking(platform: String) -> Result<PluginStatus, String> {
    let key = resolve_plugin_platform(Some(&platform));
    let mut status = plugin_status_inner(&key);
    match fetch_plugin_manifest(&key) {
        Ok(_) => Ok(plugin_status_inner(&key)),
        Err(error) => {
            let friendly = if error.contains("HTTP 404") {
                format!(
                    "服务器上还没有 {} 的插件清单，可能尚未发布。",
                    status.platform_label
                )
            } else {
                error
            };
            status.error = Some(friendly);
            Ok(status)
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn plugin_check_update(platform: Option<String>) -> Result<PluginStatus, String> {
    let key = resolve_plugin_platform(platform.as_deref());
    tauri::async_runtime::spawn_blocking(move || plugin_check_update_blocking(key))
        .await
        .map_err(|error| format!("检查插件更新任务失败：{}", error))?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn plugin_prepare_cache(
    platform: Option<String>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<PluginStatus, String> {
    let key = resolve_plugin_platform(platform.as_deref());
    let cache_lock = state.apx_cache_busy.clone();
    let cancel = state.plugin_cancel.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = cache_lock
            .lock()
            .map_err(|_| "APX 缓存锁已损坏".to_string())?;
        cancel.store(false, Ordering::Relaxed);
        prepare_plugin_cache(&key, &app, &cancel)?;
        Ok(plugin_status_inner(&key))
    })
    .await
    .map_err(|error| format!("准备 APX 缓存任务失败：{}", error))?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn plugin_install(
    platform: Option<String>,
    force: Option<bool>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<PluginStatus, String> {
    let key = resolve_plugin_platform(platform.as_deref());
    if key.eq_ignore_ascii_case("INT_STEAM") {
        return Err("Steam 国际服暂不支持插件安装".to_string());
    }
    let force = force.unwrap_or(false);
    let mutation = state.mutation.clone();
    let cache_lock = state.apx_cache_busy.clone();
    let cancel = state.plugin_cancel.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
        let _cache_guard = cache_lock.lock().map_err(|_| "APX 缓存锁已损坏")?;
        let mut lib = load_library();
        if lib.settings.plugin_platform.as_deref() != Some(key.as_str()) {
            lib.settings.plugin_platform = Some(key.clone());
            save_library(&lib)?;
        }
        cancel.store(false, Ordering::Relaxed);
        match plugin_install_blocking(&key, force, &app, &cancel) {
            Ok(count) => {
                emit_plugin_progress(
                    &app,
                    "complete",
                    Some(100),
                    &format!("安装完成，成功 {} 个文件", count),
                    None,
                    None,
                );
                Ok(plugin_status_inner(&key))
            }
            Err(error) => {
                emit_plugin_progress(
                    &app,
                    if error.contains("取消") {
                        "cancelled"
                    } else {
                        "error"
                    },
                    None,
                    &error,
                    None,
                    None,
                );
                Err(error)
            }
        }
    })
    .await
    .map_err(|error| format!("安装插件任务失败：{}", error))?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn plugin_restore(
    platform: Option<String>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<PluginStatus, String> {
    let key = resolve_plugin_platform(platform.as_deref());
    let mutation = state.mutation.clone();
    let cache_lock = state.apx_cache_busy.clone();
    let cancel = state.plugin_cancel.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = mutation.lock().map_err(|_| "库写入锁已损坏")?;
        let _cache_guard = cache_lock.lock().map_err(|_| "APX 缓存锁已损坏")?;
        cancel.store(false, Ordering::Relaxed);
        match plugin_restore_blocking(&key, &app, &cancel) {
            Ok(count) => {
                emit_plugin_progress(
                    &app,
                    "complete",
                    Some(100),
                    &format!("还原完成，成功 {} 个文件", count),
                    None,
                    None,
                );
                Ok(plugin_status_inner(&key))
            }
            Err(error) => {
                emit_plugin_progress(
                    &app,
                    if error.contains("取消") {
                        "cancelled"
                    } else {
                        "error"
                    },
                    None,
                    &error,
                    None,
                    None,
                );
                Err(error)
            }
        }
    })
    .await
    .map_err(|error| format!("还原插件任务失败：{}", error))?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn plugin_cancel(state: tauri::State<AppState>) -> Result<(), String> {
    state.plugin_cancel.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn plugin_reveal(path: String) -> Result<(), String> {
    let candidate = normalize_windows_path(&path);
    if candidate.as_os_str().is_empty() {
        return Err("目标路径为空，无法定位".into());
    }
    let canonical = candidate
        .canonicalize()
        .map_err(|_| format!("目标文件或目录不存在：{}", candidate.display()))?;
    let mut roots = Vec::new();
    for (key, _, _, _) in PLUGIN_PLATFORMS {
        if let Some(root) = locallow_game_dir(key) {
            roots.push(root);
        }
        if let Some(root) = game_data_dir(key) {
            roots.push(root);
        }
        if let Some(root) = game_root_dir(key) {
            roots.push(root);
        }
        roots.push(plugin_cache_dir(key));
    }
    let allowed = roots
        .into_iter()
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| canonical.starts_with(root));
    if !allowed {
        return Err("出于安全原因，只能定位 APX 插件目录中的文件".into());
    }
    if canonical.is_file() {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&canonical)
            .spawn()
            .map_err(|e| format!("无法打开资源管理器：{}", e))?;
        return Ok(());
    }
    if !canonical.is_dir() {
        return Err(format!("目标文件或目录不存在：{}", canonical.display()));
    }
    std::process::Command::new("explorer")
        .arg(&canonical)
        .spawn()
        .map_err(|e| format!("无法打开资源管理器：{}", e))?;
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn plugin_open_dir(platform: Option<String>, which: String) -> Result<(), String> {
    let key = resolve_plugin_platform(platform.as_deref());
    let target = match which.as_str() {
        "locallow" => locallow_game_dir(&key),
        "data" => game_data_dir(&key),
        "root" => game_root_dir(&key),
        "cache" => Some(apx_cache_root().join(&key)),
        _ => return Err("未知的目录类型".into()),
    }
    .ok_or_else(|| "目录尚未定位，请先在设置中检测游戏路径".to_string())?;
    if !target.is_dir() {
        return Err(format!("目录不存在：{}", target.to_string_lossy()));
    }
    std::process::Command::new("explorer")
        .arg(&target)
        .spawn()
        .map_err(|e| format!("无法打开资源管理器：{}", e))?;
    Ok(())
}
