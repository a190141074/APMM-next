//! 从旧版迁入的领域实现；保留协议与行为，在明确边界内逐步替换。
use crate::*;

/* ============ 可下载预览缓存 ============ */

pub(crate) const PREVIEW_MANIFEST_URL: &str = "https://github.com/a190141074/AstralPartyModManager-Previews/releases/latest/download/preview-manifest-v4.json";
pub(crate) const PREVIEW_RELEASE_DOWNLOAD_PREFIX: &str =
    "https://github.com/a190141074/AstralPartyModManager-Previews/releases/download/";
pub(crate) const PREVIEW_MAX_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const PREVIEW_CONNECT_TIMEOUT_SECS: u64 = 10;
pub(crate) const PREVIEW_REQUEST_TIMEOUT_SECS: u64 = 180;
pub(crate) const PREVIEW_SOURCE_ATTEMPTS: u32 = 2;
pub(crate) const PREVIEW_PROBE_BYTES: usize = 256 * 1024;
pub(crate) const PREVIEW_PROBE_TIMEOUT_SECS: u64 = 20;
pub(crate) const PREVIEW_PROBE_SAMPLES: usize = 2;
/// 预览下载源名称。CNB 已托管完整预览包，与 GitHub 官方、加速镜像并列参选。
pub(crate) const CNB_SOURCE_NAME: &str = "CNB 主源";
/// 预览图 CNB 源配置（编译期内嵌自 `config/cnb.json`）。
///
/// 预览包已镜像到 CNB（`astral-weird-egg/astral-party-mod-manager-assets`），
/// 因此 `preview.enabled = true`：CNB 主源与 GitHub 官方、加速镜像一并参与测速择优。
/// 若镜像仓库下线或不再维护，把 `preview.enabled` 改回 `false` 即可退回
/// 「GitHub 官方 + 3 镜像」，无需改动代码。
/// 与更新通道（`update_channel.rs`）共用同一套 `remote_source` 测速/择优引擎。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct CnbPreviewConfig {
    pub(crate) enabled: bool,
    pub(crate) raw_prefix: String,
    pub(crate) bundle_dir: String,
    pub(crate) manifest_path: String,
}

pub(crate) fn cnb_preview_config() -> CnbPreviewConfig {
    #[derive(Debug, Deserialize, Default)]
    struct CnbConfigFile {
        #[serde(default)]
        preview: Option<CnbPreviewConfig>,
    }
    let embedded = include_str!("../../config/cnb.json");
    let parsed: CnbConfigFile = serde_json::from_str(embedded).unwrap_or_default();
    let mut cfg = parsed.preview.unwrap_or(CnbPreviewConfig {
        enabled: false,
        raw_prefix: String::new(),
        bundle_dir: String::new(),
        manifest_path: String::new(),
    });
    if cfg.raw_prefix.is_empty() {
        return cfg;
    }
    if !cfg.raw_prefix.ends_with('/') {
        cfg.raw_prefix.push('/');
    }
    if cfg.bundle_dir.is_empty() {
        cfg.bundle_dir = "bundles".into();
    }
    if !cfg.bundle_dir.ends_with('/') {
        cfg.bundle_dir.push('/');
    }
    if cfg.manifest_path.is_empty() {
        cfg.manifest_path = "preview-manifest.json".into();
    }
    cfg
}

/// 已启用的 CNB 预览 raw 前缀；未启用或地址为空时返回 `None`（不加入候选源）。
pub(crate) fn cnb_preview_prefix() -> Option<String> {
    let cfg = cnb_preview_config();
    if cfg.enabled && !cfg.raw_prefix.is_empty() {
        Some(cfg.raw_prefix)
    } else {
        None
    }
}

/// 若 CNB 预览已启用，返回对应 GitHub 资产名的 raw 直链；否则返回 `None`。
pub(crate) fn cnb_preview_bundle_url(official_url: &str) -> Option<String> {
    let cfg = cnb_preview_config();
    if cfg.enabled && !cfg.raw_prefix.is_empty() {
        // CNB 清单携带版本目录时保留完整路径，不再只取文件名。
        if official_url.starts_with(&cfg.raw_prefix) && is_allowed_preview_bundle_url(official_url)
        {
            return Some(official_url.to_string());
        }
        let filename = official_url.rsplit('/').next().unwrap_or("").to_string();
        Some(format!("{}{}{}", cfg.raw_prefix, cfg.bundle_dir, filename))
    } else {
        None
    }
}

/// CNB 发布/更新通道配置。与预览通道一样来自 `config/cnb.json`，
/// 使「发布仓库地址」只在一处维护；改仓库无需改代码。
#[derive(Debug, Clone)]
pub struct CnbUpdateConfig {
    pub enabled: bool,
    pub raw_prefix: String,
    pub manifest_path: String,
    pub asset_dir: String,
    pub fallback: String,
}

/// 读取编译期内嵌的 `config/cnb.json` 的 `update` 段。
pub fn cnb_update_config() -> CnbUpdateConfig {
    #[derive(Debug, Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    struct CnbUpdateFile {
        enabled: bool,
        #[serde(default)]
        raw_prefix: String,
        #[serde(default)]
        manifest_path: String,
        #[serde(default)]
        asset_dir: String,
        #[serde(default)]
        fallback: String,
    }
    #[derive(Debug, Deserialize, Default)]
    struct CnbConfigFile2 {
        update: Option<CnbUpdateFile>,
    }
    let embedded = include_str!("../../config/cnb.json");
    let parsed: CnbConfigFile2 = serde_json::from_str(embedded).unwrap_or_default();
    let raw = parsed.update.unwrap_or(CnbUpdateFile {
        enabled: false,
        raw_prefix: String::new(),
        manifest_path: String::new(),
        asset_dir: String::new(),
        fallback: String::new(),
    });
    let mut prefix = raw.raw_prefix;
    if !prefix.is_empty() && !prefix.ends_with('/') {
        prefix.push('/');
    }
    let mut asset_dir = if raw.asset_dir.is_empty() {
        "releases".to_string()
    } else {
        raw.asset_dir
    };
    asset_dir = asset_dir.trim_matches('/').to_string();
    CnbUpdateConfig {
        enabled: raw.enabled && !prefix.is_empty(),
        raw_prefix: prefix,
        manifest_path: if raw.manifest_path.is_empty() {
            "update-manifest.json".to_string()
        } else {
            raw.manifest_path
        },
        asset_dir,
        fallback: raw.fallback,
    }
}

/// 发布仓库中某个版本安装包的 raw 直链（供发布工具与文档对齐使用）。
pub fn cnb_update_asset_url(filename: &str) -> Option<String> {
    let cfg = cnb_update_config();
    if !cfg.enabled {
        return None;
    }
    Some(format!("{}{}/{}", cfg.raw_prefix, cfg.asset_dir, filename))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemotePreviewManifest {
    #[serde(default)]
    pub(crate) schema: u32,
    #[serde(default)]
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) bundle_url: String,
    #[serde(default)]
    pub(crate) sha256: String,
    #[serde(default)]
    pub(crate) bytes: u64,
    #[serde(default)]
    pub(crate) files: usize,
    #[serde(default)]
    pub(crate) bundles: Vec<RemotePreviewBundle>,
    #[serde(skip)]
    pub(crate) bundle_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemotePreviewBundle {
    pub(crate) key: String,
    pub(crate) version: String,
    pub(crate) bundle_url: String,
    pub(crate) sha256: String,
    pub(crate) bytes: u64,
    pub(crate) files: usize,
}

/// Identity saved next to an interrupted bundle.  A partial archive may only
/// be resumed when it belongs to the exact same published preview release.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewDownloadPart {
    pub(crate) version: String,
    pub(crate) sha256: String,
    pub(crate) bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewCacheState {
    pub(crate) version: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewStatus {
    pub(crate) local_version: Option<String>,
    pub(crate) cached_files: usize,
    pub(crate) expected_files: usize,
    pub(crate) missing_files: usize,
    pub(crate) cache_bytes: u64,
    pub(crate) remote_version: Option<String>,
    pub(crate) remote_bytes: Option<u64>,
    pub(crate) update_available: Option<bool>,
    pub(crate) error: Option<String>,
    #[serde(default)]
    pub(crate) bundles: Vec<PreviewBundleStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewBundleStatus {
    pub(crate) key: String,
    pub(crate) local_version: Option<String>,
    pub(crate) cached_files: usize,
    pub(crate) expected_files: usize,
    pub(crate) missing_files: usize,
    pub(crate) state: String,
}

pub(crate) fn preview_cache_dir() -> PathBuf {
    data_dir().join("previews")
}

pub(crate) fn preview_cache_state_path(key: &str) -> PathBuf {
    preview_cache_dir().join(format!("state-{}.json", key))
}

pub(crate) fn clear_preview_download_part(key: &str) {
    let root = preview_cache_dir();
    let _ = fs::remove_file(root.join(format!(".preview-download-{}.part", key)));
    let _ = fs::remove_file(root.join(format!(".preview-download-{}.json", key)));
}

fn preview_groups() -> &'static HashMap<String, Vec<String>> {
    #[derive(Deserialize)]
    struct Group {
        key: String,
        sections: Vec<String>,
    }
    static GROUPS: std::sync::OnceLock<HashMap<String, Vec<String>>> = std::sync::OnceLock::new();
    GROUPS.get_or_init(|| {
        serde_json::from_str::<Vec<Group>>(include_str!("../../config/preview-groups.json"))
            .expect("内嵌预览分包配置必须有效")
            .into_iter()
            .map(|group| (group.key, group.sections))
            .collect()
    })
}

pub(crate) fn preview_bundle_keys() -> Vec<&'static str> {
    let mut keys: Vec<_> = preview_groups().keys().map(String::as_str).collect();
    keys.sort();
    keys
}

/// 新分包按部位限制体积；旧角色整包仍保留兼容入口。
pub(crate) fn valid_preview_bundle_key(key: &str) -> bool {
    matches!(key, "all" | "heroes") || preview_groups().contains_key(key)
}

pub(crate) fn expected_preview_files() -> Vec<String> {
    #[derive(Deserialize)]
    struct Entry {
        canonical: String,
        #[serde(default)]
        preview: Option<String>,
        #[serde(default, rename = "previewFile")]
        preview_file: Option<String>,
        #[serde(default, rename = "previewExt")]
        preview_ext: Option<String>,
    }
    let mut files: Vec<String> = serde_json::from_str::<Vec<Entry>>(HERO_MANIFEST)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.preview.is_none())
        .map(|entry| {
            entry.preview_file.unwrap_or_else(|| {
                format!(
                    "{}.{}",
                    entry.canonical,
                    entry.preview_ext.unwrap_or_else(|| "webp".into())
                )
            })
        })
        .collect();
    files.sort();
    files.dedup();
    files
}

pub(crate) fn expected_preview_files_for_bundle(key: &str) -> Vec<String> {
    #[derive(Deserialize)]
    struct Entry {
        section: String,
        canonical: String,
        #[serde(default)]
        preview: Option<String>,
        #[serde(default, rename = "previewFile")]
        preview_file: Option<String>,
        #[serde(default, rename = "previewExt")]
        preview_ext: Option<String>,
    }
    let mut files: Vec<String> = serde_json::from_str::<Vec<Entry>>(HERO_MANIFEST)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| {
            entry.preview.is_none()
                && preview_groups()
                    .get(key)
                    .map(|sections| sections.contains(&entry.section))
                    .unwrap_or_else(|| category_of(&entry.section) == key)
        })
        .map(|entry| {
            entry.preview_file.unwrap_or_else(|| {
                format!(
                    "{}.{}",
                    entry.canonical,
                    entry.preview_ext.unwrap_or_else(|| "webp".into())
                )
            })
        })
        .collect();
    files.sort();
    files.dedup();
    files
}

pub(crate) fn preview_cache_status() -> PreviewStatus {
    let expected = expected_preview_files();
    let root = preview_cache_dir();
    let mut cached_paths = std::collections::HashSet::new();
    let mut bytes = 0u64;
    for file in &expected {
        if let Ok(meta) = fs::metadata(root.join(file)) {
            if meta.is_file() {
                cached_paths.insert(file.as_str());
                bytes += meta.len();
            }
        }
    }
    let bundles: Vec<PreviewBundleStatus> = preview_bundle_keys()
        .into_iter()
        .map(|key| {
            let expected = expected_preview_files_for_bundle(key);
            let cached_files = expected
                .iter()
                .filter(|file| cached_paths.contains(file.as_str()))
                .count();
            let local_version = fs::read_to_string(preview_cache_state_path(key))
                .ok()
                .and_then(|s| serde_json::from_str::<PreviewCacheState>(&s).ok())
                .map(|state| state.version)
                .filter(|v| !v.is_empty());
            PreviewBundleStatus {
                key: key.into(),
                local_version,
                cached_files,
                expected_files: expected.len(),
                missing_files: expected.len().saturating_sub(cached_files),
                state: if cached_files == 0 {
                    "missing".into()
                } else if cached_files < expected.len() {
                    "partial".into()
                } else {
                    "ready".into()
                },
            }
        })
        .collect();
    let local_version = bundles.iter().filter_map(|b| b.local_version.clone()).max();
    let cached = cached_paths.len();
    PreviewStatus {
        local_version,
        cached_files: cached,
        expected_files: expected.len(),
        missing_files: expected.len().saturating_sub(cached),
        cache_bytes: bytes,
        remote_version: None,
        remote_bytes: None,
        update_available: None,
        error: None,
        bundles,
    }
}

pub(crate) fn preview_path_for_file(file: &str) -> Option<String> {
    let cached = preview_cache_dir().join(file);
    if cached.is_file() {
        return Some(cached.to_string_lossy().to_string());
    }
    // 开发时允许直接查看尚未发布的源图；release 永不使用该路径。
    if cfg!(debug_assertions) {
        let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../src/previews")
            .join(file);
        if dev.is_file() {
            return Some(dev.to_string_lossy().to_string());
        }
    }
    None
}

pub(crate) fn validate_preview_manifest(
    mut manifest: RemotePreviewManifest,
) -> Result<RemotePreviewManifest, String> {
    if manifest.bundles.is_empty() && !manifest.version.is_empty() {
        manifest.bundles.push(RemotePreviewBundle {
            key: "all".into(),
            version: manifest.version.clone(),
            bundle_url: manifest.bundle_url.clone(),
            sha256: manifest.sha256.clone(),
            bytes: manifest.bytes,
            files: manifest.files,
        });
    }
    if manifest.bundles.is_empty() {
        return Err("missing preview bundles".into());
    }
    let mut keys = HashSet::new();
    for bundle in &manifest.bundles {
        if !valid_preview_bundle_key(&bundle.key)
            || !keys.insert(bundle.key.clone())
            || bundle.version.is_empty()
            || bundle.sha256.len() != 64
            || bundle.bytes == 0
            || bundle.bytes > PREVIEW_MAX_BYTES
            || bundle.files == 0
            || bundle.files > 20_000
            || !is_allowed_preview_bundle_url(&bundle.bundle_url)
        {
            return Err("invalid preview bundle".into());
        }
    }
    if manifest.schema < 2
        && (manifest.version.is_empty() || manifest.bundle_url.is_empty() || manifest.files == 0)
    {
        return Err("预览更新信息不完整".into());
    }
    if manifest.schema < 2 && manifest.files > 20_000 {
        return Err("预览文件数量超过安全限制".into());
    }
    if manifest.schema < 2 && manifest.bytes > PREVIEW_MAX_BYTES {
        return Err("预览包超过安全大小限制".into());
    }
    if manifest.schema < 2 && !is_allowed_preview_bundle_url(&manifest.bundle_url) {
        return Err("预览包地址不是官方 GitHub Release".into());
    }
    Ok(manifest)
}

pub(crate) fn remote_as_bundle(bundle: &RemotePreviewBundle) -> RemotePreviewManifest {
    RemotePreviewManifest {
        schema: 2,
        version: bundle.version.clone(),
        bundle_url: bundle.bundle_url.clone(),
        sha256: bundle.sha256.clone(),
        bytes: bundle.bytes,
        files: bundle.files,
        bundles: Vec::new(),
        bundle_key: bundle.key.clone(),
    }
}

pub(crate) fn is_allowed_preview_bundle_url(url: &str) -> bool {
    if !url.ends_with(".zip") || url.contains('?') || url.contains('#') {
        return false;
    }
    let github_ok = url.starts_with(PREVIEW_RELEASE_DOWNLOAD_PREFIX);
    // CNB 直链仅在 config 启用预览源时纳入允许列表。
    let cnb_ok = cnb_preview_prefix()
        .map(|prefix| url.starts_with(&prefix))
        .unwrap_or(false);
    github_ok || cnb_ok
}

/// Every proxy URL is derived from one allow-listed GitHub Release URL.
/// Mirrors are transport-only: they cannot redirect a manifest to other hosts.
/// 预览源：CNB 主源 + 3 个 GitHub 加速镜像 + GitHub 官方，共 5 个候选。
/// CNB 已实际托管全部预览包（`astral-weird-egg/astral-party-mod-manager-assets`），
/// 因此由 `config/cnb.json` 的 `preview.enabled` 控制是否参与测速择优。
pub(crate) fn preview_source_candidates(official_url: &str) -> Vec<(&'static str, String)> {
    let without_scheme = official_url
        .strip_prefix("https://")
        .unwrap_or(official_url);
    let cfg = cnb_preview_config();
    let mut v: Vec<(&'static str, String)> = Vec::new();
    if cfg.enabled && !cfg.raw_prefix.is_empty() {
        v.push((
            CNB_SOURCE_NAME,
            format!("{}{}", cfg.raw_prefix, cfg.manifest_path),
        ));
    }
    v.push((
        "加速源 ghproxy.net",
        format!("https://ghproxy.net/{}", official_url),
    ));
    v.push((
        "加速源 cors.isteed.cc",
        format!("https://cors.isteed.cc/{}", without_scheme),
    ));
    v.push((
        "加速源 gh-proxy.com",
        format!("https://gh-proxy.com/{}", official_url),
    ));
    v.push(("GitHub 官方", official_url.to_string()));
    v
}

pub(crate) fn manifest_version_rank(version: &str) -> u64 {
    version.parse::<u64>().unwrap_or(0)
}

/// 换源时不以旧清单覆盖新缓存；同版本缺图仍需补齐。
pub(crate) fn preview_needs_update(local: Option<&PreviewBundleStatus>, remote: &str) -> bool {
    let Some(local) = local else {
        return true;
    };
    let Some(version) = local.local_version.as_deref() else {
        return true;
    };
    let remote_rank = manifest_version_rank(remote);
    let local_rank = manifest_version_rank(version);
    remote_rank > local_rank || (remote_rank == local_rank && local.missing_files > 0)
}

pub(crate) fn fetch_preview_manifest(
    app: Option<&tauri::AppHandle>,
    cancel: Option<&AtomicBool>,
) -> Result<RemotePreviewManifest, String> {
    if let Some(app) = app {
        emit_preview_progress(app, "probe", None, "正在并发测试更新源…", None, None);
    }
    let handles: Vec<_> = preview_source_candidates(PREVIEW_MANIFEST_URL)
        .into_iter()
        .map(|(source, url)| {
            std::thread::spawn(move || {
                let started = Instant::now();
                let result = http_download(&url, 1024 * 1024, None, "manifest", None, None, "");
                let elapsed = started.elapsed();
                let parsed = result
                    .and_then(|bytes| {
                        String::from_utf8(bytes).map_err(|_| "预览更新信息不是 UTF-8 文本".into())
                    })
                    .and_then(|text| {
                        serde_json::from_str::<RemotePreviewManifest>(&text)
                            .map_err(|e| format!("预览更新信息格式无效：{}", e))
                    })
                    .and_then(validate_preview_manifest);
                (source, elapsed, parsed)
            })
        })
        .collect();
    let mut valid = Vec::new();
    let mut errors = Vec::new();
    for handle in handles {
        let (source, elapsed, result) = handle.join().map_err(|_| "更新源测试线程异常")?;
        match result {
            Ok(manifest) => valid.push((source, elapsed, manifest)),
            Err(error) => errors.push(format!("{}：{}", source, error)),
        }
    }
    if cancel
        .map(|flag| flag.load(Ordering::Relaxed))
        .unwrap_or(false)
    {
        return Err("用户取消下载".into());
    }
    valid.sort_by(|a, b| {
        manifest_version_rank(&b.2.version)
            .cmp(&manifest_version_rank(&a.2.version))
            .then_with(|| a.1.cmp(&b.1))
    });
    if let Some((source, elapsed, manifest)) = valid.into_iter().next() {
        if let Some(app) = app {
            emit_preview_progress(
                app,
                "manifest",
                None,
                &format!("已选择{}（{} ms）", source, elapsed.as_millis()),
                None,
                None,
            );
        }
        return Ok(manifest);
    }
    Err(format!("无法获取预览更新信息：{}", errors.join("；")))
}

#[derive(Debug)]
pub(crate) struct PreviewProbe {
    pub(crate) source: &'static str,
    pub(crate) url: String,
    pub(crate) latency_ms: u128,
    pub(crate) bytes_per_second: u64,
}

// 单源测速逻辑已迁移至 `remote_source::probe_source`，由 `ranked_preview_bundle_sources`
// 借助统一引擎对多源并发探测并按「吞吐 × 稳定性 − 延迟」综合排序后择优。

/// 对预览包的全部候选源并发测速（含 CNB 主源），按「吞吐 × 稳定性 − 延迟」
/// 综合排序后返回。无论清单来自哪个源，这里都按 GitHub 资产名推导出一致的
/// CNB / 镜像 / 官方四组源，保证择优逻辑与清单来源解耦。
pub(crate) fn preview_bundle_sources(official_url: &str) -> Vec<Source> {
    let mut sources = Vec::new();
    // CNB 主源由 config 控制；已托管预览包时参与测速择优。
    if let Some(cnb_url) = cnb_preview_bundle_url(official_url) {
        let direct = cnb_url == official_url;
        sources.push(Source::new(CNB_SOURCE_NAME, SourceKind::Cnb, cnb_url));
        if direct {
            return sources;
        }
    }
    sources.push(Source::new(
        "加速源 ghproxy.net",
        SourceKind::Mirror,
        format!("https://ghproxy.net/{}", official_url),
    ));
    sources.push(Source::new(
        "加速源 cors.isteed.cc",
        SourceKind::Mirror,
        format!(
            "https://cors.isteed.cc/{}",
            official_url
                .strip_prefix("https://")
                .unwrap_or(official_url)
        ),
    ));
    sources.push(Source::new(
        "加速源 gh-proxy.com",
        SourceKind::Mirror,
        format!("https://gh-proxy.com/{}", official_url),
    ));
    sources.push(Source::new(
        "GitHub 官方",
        SourceKind::Official,
        official_url.to_string(),
    ));
    sources
}

/// 对实际托管的来源测速；CNB 直链不构造不存在的 GitHub 镜像。
pub(crate) fn ranked_preview_bundle_sources(official_url: &str) -> Vec<PreviewProbe> {
    let scores = RemoteSourcePool::new("preview-bundle", preview_bundle_sources(official_url))
        .rank(
            PREVIEW_PROBE_SAMPLES,
            PREVIEW_PROBE_BYTES,
            Duration::from_secs(PREVIEW_CONNECT_TIMEOUT_SECS),
            Duration::from_secs(PREVIEW_PROBE_TIMEOUT_SECS),
        );
    scores
        .into_iter()
        .map(|s| PreviewProbe {
            source: s.source.name,
            url: s.source.url,
            latency_ms: s.latency_ms,
            bytes_per_second: s.bytes_per_second,
        })
        .collect()
}

pub(crate) fn download_preview_bundle(
    source: &str,
    url: &str,
    remote: &RemotePreviewManifest,
    app: Option<&tauri::AppHandle>,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<u8>, String> {
    let progress_message = format!("正在通过{}下载更新包", source);
    if let Some(app) = app {
        emit_preview_progress(
            app,
            "download",
            Some(0),
            &progress_message,
            Some(0),
            Some(remote.bytes),
        );
    }
    let bytes = http_download_resumable(
        url,
        PREVIEW_MAX_BYTES,
        app,
        remote,
        cancel,
        &progress_message,
    )?;
    if bytes.len() as u64 > PREVIEW_MAX_BYTES {
        return Err("预览包超过安全大小限制".into());
    }
    if remote.bytes > 0 && bytes.len() as u64 != remote.bytes {
        return Err("预览包大小校验失败，请稍后重试".into());
    }
    Ok(bytes)
}

pub(crate) fn http_download_resumable(
    url: &str,
    max_bytes: u64,
    app: Option<&tauri::AppHandle>,
    remote: &RemotePreviewManifest,
    cancel: Option<&AtomicBool>,
    message: &str,
) -> Result<Vec<u8>, String> {
    if remote.bytes > max_bytes {
        return Err("预览包超过大小限制".into());
    }
    let flag = AtomicBool::new(false);
    let path = crate::downloads::fetch(
        crate::downloads::Download {
            identity: &format!("preview:{}:{}", remote.bundle_key, remote.version),
            urls: vec![url.to_string()],
            bytes: remote.bytes,
            sha256: &remote.sha256,
        },
        cancel.unwrap_or(&flag),
        |done, total, speed| {
            if let Some(app) = app {
                use tauri::Manager;
                app.state::<crate::tasks::TaskManager>()
                    .download_progress(app, done, total, speed);
                emit_preview_progress(
                    app,
                    "download",
                    Some((done * 100 / total.max(1)) as u8),
                    message,
                    Some(done),
                    Some(total),
                );
            }
        },
    )?;
    fs::read(path).map_err(|e| e.to_string())
}

pub(crate) fn http_download(
    url: &str,
    max_bytes: u64,
    app: Option<&tauri::AppHandle>,
    phase: &str,
    expected_bytes: Option<u64>,
    cancel: Option<&AtomicBool>,
    progress_message: &str,
) -> Result<Vec<u8>, String> {
    let client = Client::builder()
        .redirect(Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(PREVIEW_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(PREVIEW_REQUEST_TIMEOUT_SECS))
        .user_agent("AstralPartyModManager/0.1")
        .build()
        .map_err(|e| format!("初始化网络组件失败：{}", e))?;
    let mut last_error = String::new();
    for attempt in 0..PREVIEW_SOURCE_ATTEMPTS {
        let response = client.get(url).send();
        let mut response = match response {
            Ok(response) if response.status().is_success() => response,
            Ok(response) => {
                last_error = format!("服务器返回 HTTP {}", response.status());
                if !(response.status().is_server_error() || response.status().as_u16() == 429)
                    || attempt + 1 == PREVIEW_SOURCE_ATTEMPTS
                {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
                continue;
            }
            Err(error) => {
                last_error = format!("网络请求失败：{}", error);
                if attempt + 1 == PREVIEW_SOURCE_ATTEMPTS {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
                continue;
            }
        };
        let total = response.content_length().or(expected_bytes);
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let temp = std::env::temp_dir().join(format!("astral-preview-{}.tmp", unique));
        let result = (|| {
            let mut file =
                fs::File::create(&temp).map_err(|e| format!("创建临时下载文件失败：{}", e))?;
            let mut buffer = [0u8; 64 * 1024];
            let mut downloaded = 0u64;
            loop {
                if cancel
                    .map(|flag| flag.load(Ordering::Relaxed))
                    .unwrap_or(false)
                {
                    return Err("用户取消下载".into());
                }
                let count = response
                    .read(&mut buffer)
                    .map_err(|e| format!("读取下载数据失败：{}", e))?;
                if count == 0 {
                    break;
                }
                downloaded = downloaded.saturating_add(count as u64);
                if downloaded > max_bytes {
                    return Err("下载文件超过安全大小限制".into());
                }
                file.write_all(&buffer[..count])
                    .map_err(|e| format!("写入临时下载文件失败：{}", e))?;
                let percent = total
                    .filter(|size| *size > 0)
                    .map(|size| ((downloaded.min(size) * 100) / size) as u8);
                if let Some(app) = app {
                    emit_preview_progress(
                        app,
                        phase,
                        percent,
                        progress_message,
                        Some(downloaded),
                        total,
                    );
                }
            }
            file.flush()
                .map_err(|e| format!("刷新临时下载文件失败：{}", e))?;
            fs::read(&temp).map_err(|e| format!("读取下载文件失败：{}", e))
        })();
        let _ = fs::remove_file(&temp);
        match result {
            Ok(bytes) => return Ok(bytes),
            Err(error) => {
                last_error = error;
                if attempt + 1 < PREVIEW_SOURCE_ATTEMPTS {
                    std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
                }
            }
        }
    }
    Err(format!(
        "下载预览资源失败：{}",
        if last_error.is_empty() {
            "未知网络错误"
        } else {
            &last_error
        }
    ))
}

pub(crate) fn sha256_bytes(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

/// Install one independently published section without replacing resources
/// belonging to other sections. Missing local preview files are intentionally
/// allowed: the UI presents those as placeholders rather than a failed update.
pub(crate) fn install_preview_bundle_section(
    bundle: &[u8],
    remote: &RemotePreviewManifest,
    cancel: Option<&AtomicBool>,
) -> Result<(), String> {
    crate::preview_install::install(bundle, remote, cancel, false)
}
pub(crate) fn install_preview_bundle(
    bundle: &[u8],
    remote: &RemotePreviewManifest,
    cancel: Option<&AtomicBool>,
) -> Result<(), String> {
    crate::preview_install::install(bundle, remote, cancel, true)
}
