//! 统一的远程下载源选择与测速引擎。
//!
//! 所有远程资源（预览包、插件补丁、软件更新）都通过 [`RemoteSourcePool`]
//! 暴露多个候选源，并发探测后按「吞吐 × 稳定性 − 延迟惩罚」综合打分排序，
//! 由调用方择优下载。CNB 作为一等公民源（[`SourceKind::Cnb`]）与 GitHub 官方、
//! 第三方加速镜像并列，使「下载」与「更新」逻辑完全可复用、可扩展。
//!
//! 设计目标：
//! - 下载与更新共用同一套源选择策略，新增托管方只需追加一个 [`Source`]。
//! - 测速不仅看速度，还看稳定性（多轮采样成功率），避免偶发快源误导。
//! - 源类型可加权，CNB/官方优先于第三方镜像。

// 本模块是可复用引擎：部分 `SourceKind`/`SourceScore` 访问器与 `RemoteSourcePool`
// 的便捷方法供调用方按需使用，未全部在本 crate 内引用，故整体豁免 dead_code 提示。
#![allow(dead_code)]

use std::io::Read;
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::header::RANGE;
use reqwest::redirect::Policy;

const USER_AGENT: &str = concat!("AstralPartyModManager/", env!("CARGO_PKG_VERSION"));

/// 源类型。`Cnb` 为统一托管方案的首选源，`Official` 为 GitHub 官方，
/// `Mirror` 为第三方加速镜像（仅作传输代理，不会改写清单指向其他主机）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Official,
    Mirror,
    Cnb,
}

impl SourceKind {
    /// 综合排序时的基础权重：CNB 与官方优先于第三方镜像。
    pub fn base_weight(&self) -> f64 {
        match self {
            SourceKind::Cnb => 1.0,
            SourceKind::Official => 0.9,
            SourceKind::Mirror => 0.7,
        }
    }

    /// 供 UI / 日志使用的简短标签。
    pub fn as_label(&self) -> &'static str {
        match self {
            SourceKind::Cnb => "cnb",
            SourceKind::Official => "official",
            SourceKind::Mirror => "mirror",
        }
    }
}

/// 一个具体的下载源：名称（静态，用于日志/UI）、类型、完整 URL。
#[derive(Debug, Clone)]
pub struct Source {
    pub name: &'static str,
    pub kind: SourceKind,
    pub url: String,
}

impl Source {
    pub fn new(name: &'static str, kind: SourceKind, url: String) -> Self {
        Source { name, kind, url }
    }

    /// 由 CNB `raw` 前缀与仓库内相对路径拼出直链。
    pub fn cnb(prefix: &str, relative: &str) -> Source {
        Source {
            name: "CNB 主源",
            kind: SourceKind::Cnb,
            url: format!(
                "{}/{}",
                prefix.trim_end_matches('/'),
                relative.trim_start_matches('/')
            ),
        }
    }
}

/// 单个源经多轮探测后的评分结果。
#[derive(Debug, Clone)]
pub struct SourceScore {
    pub source: Source,
    pub latency_ms: u128,
    /// 平均吞吐（字节/秒）。
    pub bytes_per_second: u64,
    /// 稳定性：成功探测次数 / 总探测次数，范围 0..1。
    pub stability: f64,
    /// 是否至少有一轮探测成功。
    pub usable: bool,
    /// 综合得分（仅用于排序）。
    pub score: f64,
}

impl SourceScore {
    pub fn throughput_mbps(&self) -> f64 {
        self.bytes_per_second as f64 / 1_048_576.0
    }
}

/// 对单个源发起一轮小流量探测，返回 `(延迟 ms, 吞吐 bytes/s)`。
///
/// 优先使用 Range 请求（期望 206），但也接受 200 完整小响应；无论哪种，
/// 只读 `probe_bytes` 即断开，避免拉取整个大文件。CNB raw 与 GitHub Release
///  alike 均可被探测。
pub fn probe_source(
    source: &Source,
    probe_bytes: usize,
    connect_timeout: Duration,
    probe_timeout: Duration,
) -> Result<(u128, u64), String> {
    let client = Client::builder()
        .redirect(Policy::limited(10))
        .connect_timeout(connect_timeout)
        .timeout(probe_timeout)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("初始化测速失败：{}", e))?;
    let started = Instant::now();
    let mut response = client
        .get(&source.url)
        .header(RANGE, format!("bytes=0-{}", probe_bytes.saturating_sub(1)))
        .send()
        .map_err(|e| format!("测速请求失败：{}", e))?;
    let status = response.status().as_u16();
    if status != 206 && status != 200 {
        return Err(format!("测速未返回可用数据（HTTP {}）", status));
    }
    let latency = started.elapsed();
    let mut read = 0usize;
    let mut buffer = [0u8; 32 * 1024];
    while read < probe_bytes {
        let count = response
            .read(&mut buffer)
            .map_err(|e| format!("测速读取失败：{}", e))?;
        if count == 0 {
            break;
        }
        read += count;
    }
    if read == 0 {
        return Err("测速未读取到数据".into());
    }
    let elapsed_secs = started.elapsed().as_secs_f64().max(0.001);
    Ok((latency.as_millis(), (read as f64 / elapsed_secs) as u64))
}

/// 对全部候选源并发探测 `samples` 轮，按吞吐与稳定性综合排序。
///
/// 完全不可用的源（`stability == 0`）排在末尾、保持原顺序，作为兜底重试。
pub fn rank_sources(
    sources: &[Source],
    samples: usize,
    probe_bytes: usize,
    connect_timeout: Duration,
    probe_timeout: Duration,
) -> Vec<SourceScore> {
    let samples = samples.max(1);
    let mut scored: Vec<SourceScore> = Vec::with_capacity(sources.len());
    for source in sources {
        let handles: Vec<_> = (0..samples)
            .map(|_| {
                let src = Source {
                    name: source.name,
                    kind: source.kind,
                    url: source.url.clone(),
                };
                thread::spawn(move || {
                    probe_source(&src, probe_bytes, connect_timeout, probe_timeout)
                })
            })
            .collect();
        let mut latencies = Vec::new();
        let mut throughputs = Vec::new();
        let mut success = 0usize;
        for handle in handles {
            if let Ok(Ok((lat, tp))) = handle.join() {
                success += 1;
                latencies.push(lat);
                throughputs.push(tp);
            }
        }
        let usable = success > 0;
        let stability = success as f64 / samples as f64;
        let latency_ms = if latencies.is_empty() {
            u128::MAX
        } else {
            latencies.iter().sum::<u128>() / latencies.len() as u128
        };
        let bytes_per_second = if throughputs.is_empty() {
            0
        } else {
            throughputs.iter().sum::<u64>() / throughputs.len() as u64
        };
        // 吞吐主导（MB/s × 1000），稳定性加权，源类型加权，延迟做轻量惩罚。
        let score = if usable {
            let mbps = bytes_per_second as f64 / 1_048_576.0;
            mbps * 1000.0 * stability * source.kind.base_weight() - latency_ms as f64 * 0.02
        } else {
            f64::MIN
        };
        scored.push(SourceScore {
            source: source.clone(),
            latency_ms,
            bytes_per_second,
            stability,
            usable,
            score,
        });
    }
    let mut ranked = scored;
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.latency_ms.cmp(&b.latency_ms))
    });
    ranked
}

/// 逻辑上同一份资源的一组候选源。命名资源便于日志与 UI 展示，
/// 并集中持有其全部候选下载地址。
#[derive(Debug, Clone)]
pub struct RemoteSourcePool {
    pub name: &'static str,
    pub sources: Vec<Source>,
}

impl RemoteSourcePool {
    pub fn new(name: &'static str, sources: Vec<Source>) -> Self {
        RemoteSourcePool { name, sources }
    }

    /// 综合排序后的全部源（含不可用的兜底），不可用的排在尾部。
    pub fn rank(
        &self,
        samples: usize,
        probe_bytes: usize,
        connect_timeout: Duration,
        probe_timeout: Duration,
    ) -> Vec<SourceScore> {
        rank_sources(
            &self.sources,
            samples,
            probe_bytes,
            connect_timeout,
            probe_timeout,
        )
    }

    /// 仅保留可用源（稳定性 > 0），用于快速决策。
    pub fn usable(
        &self,
        samples: usize,
        probe_bytes: usize,
        connect_timeout: Duration,
        probe_timeout: Duration,
    ) -> Vec<SourceScore> {
        self.rank(samples, probe_bytes, connect_timeout, probe_timeout)
            .into_iter()
            .filter(|s| s.usable)
            .collect()
    }

    /// 当前最优可用源（无则 None）。
    pub fn best(
        &self,
        samples: usize,
        probe_bytes: usize,
        connect_timeout: Duration,
        probe_timeout: Duration,
    ) -> Option<SourceScore> {
        self.rank(samples, probe_bytes, connect_timeout, probe_timeout)
            .into_iter()
            .find(|s| s.usable)
    }
}
