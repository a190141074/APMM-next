//! Scanning infrastructure shared by asset and modimg workflows.
//!
//! The scanner owns only deterministic, side-effect-free helpers. Cache
//! persistence and library mutation remain in the application layer so there
//! is still one authoritative state and one mutation lock.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Stable directory snapshot used before parsing or hashing files.
pub(crate) fn sorted_entries(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut entries = std::fs::read_dir(root)?
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    Ok(entries)
}

/// Throttles progress events while preserving the first and final event.
#[derive(Debug)]
pub(crate) struct ProgressGate {
    interval: Duration,
    last: Instant,
}

impl ProgressGate {
    pub(crate) fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: Instant::now() - interval,
        }
    }

    pub(crate) fn should_emit(&mut self, first: bool, final_event: bool) -> bool {
        if first || final_event || self.last.elapsed() >= self.interval {
            self.last = Instant::now();
            true
        } else {
            false
        }
    }
}
