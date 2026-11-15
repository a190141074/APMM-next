//! 软件自更新通道（预留）。
//!
//! 与预览图、插件补丁共用 [`crate::remote_source`] 的源选择引擎，使「下载」
//! 与「更新」逻辑完全复用。CNB 作为更新清单与安装包的主源，GitHub Release
//! 作为兜底。当前阶段只实现「检测 + 拉取并校验安装包」，安装/重启由上层决定。

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::remote_source::{RemoteSourcePool, Source, SourceKind};

/// 更新清单兜底源（GitHub Release）。CNB 主源地址来自 `config/cnb.json`
/// 的 `update` 段（见 `crate::cnb_update_config()`），改仓库无需改代码。
pub const UPDATE_GITHUB_MANIFEST_URL: &str =
    "https://github.com/a190141074/APMM-next/releases/latest/download/update-manifest-v2.json";

const UPDATE_PROBE_BYTES: usize = 64 * 1024;
const UPDATE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const UPDATE_PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const UPDATE_PROBE_SAMPLES: usize = 2;
const UPDATE_MAX_BYTES: u64 = 512 * 1024 * 1024;
const UPDATE_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

const USER_AGENT: &str = concat!("AstralPartyModManager/", env!("CARGO_PKG_VERSION"));

/// 单个平台安装包。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    #[serde(default)]
    pub platform: String,
    pub url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub kind: String,
}

/// 远端更新清单。CNB 与 GitHub 两个主源返回同一份语义的清单。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    #[serde(default)]
    pub schema: u32,
    #[serde(
        alias = "latestVersion",
        alias = "latest_version",
        alias = "tagName",
        alias = "tag_name"
    )]
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub assets: Vec<UpdateAsset>,
}

/// 默认更新清单候选源：CNB 发布仓库在前（配置启用时），GitHub Release 兜底。
/// 地址来自 `config/cnb.json` 的 `update` 段，发布仓库与资源仓库职责分离。
pub fn default_update_sources() -> Vec<Source> {
    let cfg = crate::cnb_update_config();
    let mut sources = Vec::new();
    if cfg.enabled {
        sources.push(Source::cnb(&cfg.raw_prefix, &cfg.manifest_path));
    }
    let fallback = if cfg.fallback.is_empty() {
        UPDATE_GITHUB_MANIFEST_URL.to_string()
    } else {
        cfg.fallback
    };
    sources.push(Source::new("GitHub 官方", SourceKind::Official, fallback));
    sources
}

/// 安装包地址白名单：仅允许配置中的 CNB 发布仓库与 GitHub Release 资产。
pub fn is_allowed_update_asset_url(url: &str) -> bool {
    if url.contains('?') || url.contains('#') {
        return false;
    }
    let cfg = crate::cnb_update_config();
    if cfg.enabled && !cfg.raw_prefix.is_empty() && url.starts_with(&cfg.raw_prefix) {
        return true;
    }
    url.starts_with("https://github.com/a190141074/APMM-next/releases/download/")
        || url.starts_with("https://github.com/a190141074/APMM-next/releases/latest/download/")
}

/// 当前版本与远端版本比较：返回 `Ordering` 指代 `(a, b)`。
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| {
        s.split('.')
            .map(|p| {
                p.trim_matches(|c: char| !c.is_ascii_digit())
                    .parse::<u64>()
                    .unwrap_or(0)
            })
            .collect::<Vec<u64>>()
    };
    let (va, vb) = (parse(a), parse(b));
    va.cmp(&vb)
}

fn get_text(url: &str, max_bytes: u64) -> Result<String, String> {
    let client = Client::builder()
        .redirect(Policy::limited(10))
        .connect_timeout(UPDATE_CONNECT_TIMEOUT)
        .timeout(UPDATE_PROBE_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("初始化更新请求失败：{}", e))?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|e| format!("获取更新清单失败：{}", e))?;
    if !response.status().is_success() {
        return Err(format!("更新清单服务返回 HTTP {}", response.status()));
    }
    let mut collected: Vec<u8> = Vec::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = response
            .read(&mut buffer)
            .map_err(|e| format!("读取更新清单失败：{}", e))?;
        if count == 0 {
            break;
        }
        collected.extend_from_slice(&buffer[..count]);
        if collected.len() as u64 > max_bytes {
            return Err("更新清单超过大小上限".into());
        }
    }
    String::from_utf8(collected).map_err(|_| "更新清单不是 UTF-8 文本".into())
}

/// 并发测速并按可用性选出最优清单源，拉取并解析清单。
pub fn fetch_update_manifest() -> Result<UpdateManifest, String> {
    fetch_update_manifest_with_source().map(|(manifest, _)| manifest)
}

/// 同 [`fetch_update_manifest`]，额外返回实际命中的源名称（便于界面提示）。
pub fn fetch_update_manifest_with_source() -> Result<(UpdateManifest, String), String> {
    let sources = default_update_sources();
    let ranked = RemoteSourcePool::new("update-manifest", sources).rank(
        UPDATE_PROBE_SAMPLES,
        UPDATE_PROBE_BYTES,
        UPDATE_CONNECT_TIMEOUT,
        UPDATE_PROBE_TIMEOUT,
    );
    let mut last_error = String::new();
    for score in ranked.iter().filter(|s| s.usable) {
        match get_text(&score.source.url, 8 * 1024 * 1024) {
            Ok(text) => match serde_json::from_str::<UpdateManifest>(&text) {
                Ok(manifest)
                    if manifest.schema == 2
                        && semver::Version::parse(manifest.version.trim_start_matches('v'))
                            .is_ok() =>
                {
                    return Ok((manifest, score.source.name.to_string()))
                }
                Ok(_) => last_error = "更新清单缺少版本号".into(),
                Err(e) => last_error = format!("更新清单格式无效：{}", e),
            },
            Err(e) => last_error = e,
        }
    }
    Err(format!("无法获取更新清单：{}", last_error))
}

/// 比对当前版本，返回需要更新的清单（无更新返回 None）。
pub fn check_for_update(current: &str) -> Result<Option<UpdateManifest>, String> {
    let manifest = fetch_update_manifest()?;
    if version_cmp(&manifest.version, current) == std::cmp::Ordering::Greater {
        return Ok(Some(manifest));
    }
    Ok(None)
}

/// 为指定平台挑选安装包（默认 `win-x86_64`）。
pub fn pick_asset<'a>(
    manifest: &'a UpdateManifest,
    platform: &str,
    kind: &str,
) -> Option<&'a UpdateAsset> {
    if manifest.schema != 2 || platform.is_empty() || !matches!(kind, "portable" | "msi") {
        return None;
    }
    let mut matching = manifest
        .assets
        .iter()
        .filter(|a| a.platform == platform && a.kind == kind);
    let found = matching.next()?;
    if matching.next().is_some() {
        None
    } else {
        Some(found)
    }
}

/// 拉取安装包到临时文件并做 SHA-256 校验；取消、读取或校验失败都会清理临时文件。
pub fn download_update_asset(
    asset: &UpdateAsset,
    cancel: &AtomicBool,
    progress: impl Fn(u64, u64, f64),
) -> Result<PathBuf, String> {
    if !is_allowed_update_asset_url(&asset.url) {
        return Err("安装包地址不在允许的来源内".into());
    }
    let mut urls = vec![asset.url.clone()];
    if let Some(name) = asset.url.rsplit('/').next() {
        if let Some(url) = crate::cnb_update_asset_url(name) {
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
    }
    crate::downloads::fetch(
        crate::downloads::Download {
            identity: &asset.url,
            urls,
            bytes: asset.bytes,
            sha256: &asset.sha256,
        },
        cancel,
        progress,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_platform_and_installation_kind_are_required() {
        let mut manifest: UpdateManifest = serde_json::from_value(serde_json::json!({
            "schema":2,"version":"0.4.1","assets":[
                {"platform":"win-x86_64","kind":"msi","url":"https://example.test/app.msi","bytes":1,"sha256":"abc"},
                {"platform":"win-x86_64","kind":"portable","url":"https://example.test/app.zip","bytes":1,"sha256":"def"}
            ]
        })).unwrap();
        assert_eq!(
            pick_asset(&manifest, "win-x86_64", "portable")
                .unwrap()
                .kind,
            "portable"
        );
        assert_eq!(
            pick_asset(&manifest, "win-x86_64", "msi").unwrap().kind,
            "msi"
        );
        assert!(pick_asset(&manifest, "linux-x86_64", "portable").is_none());
        assert!(pick_asset(&manifest, "win-x86_64", "legacy-bridge").is_none());
        manifest.assets.push(manifest.assets[0].clone());
        assert!(pick_asset(&manifest, "win-x86_64", "msi").is_none());
        manifest.schema = 1;
        assert!(pick_asset(&manifest, "win-x86_64", "portable").is_none());
    }

    #[test]
    fn legacy_manifest_version_fields_are_accepted() {
        let manifest: UpdateManifest =
            serde_json::from_str(r#"{"latestVersion":"v0.3.5"}"#).unwrap();
        assert_eq!(manifest.version, "v0.3.5");
        assert_eq!(
            version_cmp(&manifest.version, "0.3.4"),
            std::cmp::Ordering::Greater
        );
    }
}
