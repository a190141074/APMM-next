//! 所有二进制下载共用的断点、范围校验与原子提交；未校验文件只能留在缓存中。
use crate::*;
use std::sync::OnceLock;

pub struct Download<'a> {
    pub identity: &'a str,
    pub urls: Vec<String>,
    pub bytes: u64,
    pub sha256: &'a str,
}

/// 严格验证完整 Content-Range，避免错误范围被拼接到已有文件。
fn valid_range(header: &str, offset: u64, total: u64) -> bool {
    let Some(value) = header.strip_prefix("bytes ") else {
        return false;
    };
    let Some((range, length)) = value.split_once('/') else {
        return false;
    };
    let Some((start, end)) = range.split_once('-') else {
        return false;
    };
    matches!((start.parse::<u64>(),end.parse::<u64>(),length.parse::<u64>()),
        (Ok(s),Ok(e),Ok(t)) if s==offset && t==total && s<=e && e==total.saturating_sub(1))
}

pub fn checked(path: &Path, bytes: u64, hash: &str) -> bool {
    path.is_file()
        && fs::metadata(path)
            .map(|m| m.len() == bytes)
            .unwrap_or(false)
        && sha256_file(path)
            .map(|value| value.eq_ignore_ascii_case(hash))
            .unwrap_or(false)
}

pub fn fetch(
    request: Download<'_>,
    cancel: &AtomicBool,
    progress: impl Fn(u64, u64, f64),
) -> Result<PathBuf, String> {
    fetch_at(
        &data_dir().join("cache/downloads"),
        request,
        cancel,
        progress,
    )
}

fn fetch_at(
    root: &Path,
    request: Download<'_>,
    cancel: &AtomicBool,
    progress: impl Fn(u64, u64, f64),
) -> Result<PathBuf, String> {
    // 单一下载槽让旧调用者也受有界并发保护；等待时可取消。
    static SLOT: OnceLock<Mutex<()>> = OnceLock::new();
    let gate = SLOT.get_or_init(|| Mutex::new(()));
    let _guard = loop {
        if cancel.load(Ordering::Acquire) {
            return Err("下载已取消，可重试续传".into());
        }
        match gate.try_lock() {
            Ok(lock) => break lock,
            Err(std::sync::TryLockError::Poisoned(error)) => break error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => {}
        }
        std::thread::sleep(Duration::from_millis(30));
    };
    if request.bytes == 0
        || request.bytes > 4 * 1024 * 1024 * 1024
        || request.sha256.len() != 64
        || !request.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || request.urls.is_empty()
    {
        return Err("下载清单缺少有效大小、SHA-256 或来源".into());
    }
    ensure_dir(root).map_err(|e| e.to_string())?;
    let identity = sha256_bytes(
        format!(
            "{}:{}:{}",
            request.identity,
            request.sha256.to_lowercase(),
            request.bytes
        )
        .as_bytes(),
    );
    let target = root.join(format!("{identity}.verified"));
    if checked(&target, request.bytes, request.sha256) {
        progress(request.bytes, request.bytes, 0.);
        return Ok(target);
    }
    let part = root.join(format!("{identity}.part"));
    // blocking 的 timeout 覆盖每次读取；异步 read_timeout 需要反应器，不能用于此同步流。
    let client = Client::builder()
        .redirect(Policy::limited(5))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .user_agent("APMM/0.3.37")
        .build()
        .map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    for url in &request.urls {
        for _ in 0..2 {
            if cancel.load(Ordering::Acquire) {
                return Err("下载已取消，可重试续传".into());
            }
            let attempt = (|| {
                let mut offset = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
                if offset >= request.bytes {
                    offset = 0;
                }
                let mut query = client.get(url);
                if offset > 0 {
                    query = query.header(RANGE, format!("bytes={offset}-"));
                }
                let mut response = query.send().map_err(|e| e.to_string())?;
                match response.status().as_u16() {
                    200 => offset = 0,
                    206 => {
                        let range = response
                            .headers()
                            .get(CONTENT_RANGE)
                            .and_then(|h| h.to_str().ok())
                            .unwrap_or("");
                        if !valid_range(range, offset, request.bytes) {
                            return Err(format!("服务器返回错误范围：{range}"));
                        }
                    }
                    code => return Err(format!("HTTP {code}")),
                }
                if response
                    .content_length()
                    .is_some_and(|n| n != request.bytes - offset)
                {
                    return Err("下载响应长度与清单不一致".into());
                }
                let mut output = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(offset == 0)
                    .append(offset > 0)
                    .open(&part)
                    .map_err(|e| e.to_string())?;
                let start = Instant::now();
                let mut last = Instant::now();
                let mut done = offset;
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    if cancel.load(Ordering::Acquire) {
                        return Err("下载已取消，可重试续传".into());
                    }
                    let count = response.read(&mut buffer).map_err(|e| e.to_string())?;
                    if count == 0 {
                        break;
                    }
                    done += count as u64;
                    if done > request.bytes {
                        return Err("下载超出清单大小".into());
                    }
                    output
                        .write_all(&buffer[..count])
                        .map_err(|e| e.to_string())?;
                    if last.elapsed() >= Duration::from_millis(150) || done == request.bytes {
                        progress(
                            done,
                            request.bytes,
                            (done - offset) as f64 / start.elapsed().as_secs_f64().max(0.001),
                        );
                        last = Instant::now();
                    }
                }
                output.sync_all().map_err(|e| e.to_string())?;
                drop(output);
                if !checked(&part, request.bytes, request.sha256) {
                    fs::File::create(&part).map_err(|e| e.to_string())?;
                    return Err("下载大小或 SHA-256 校验失败，已丢弃损坏内容".into());
                }
                if target.exists() {
                    fs::remove_file(&target).map_err(|e| e.to_string())?;
                }
                fs::rename(&part, &target).map_err(|e| e.to_string())?;
                Ok(())
            })();
            match attempt {
                Ok(()) => return Ok(target),
                Err(e) => errors.push(format!("{url}: {e}")),
            }
        }
    }
    Err(errors.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn validates_entire_range() {
        assert!(valid_range("bytes 3-9/10", 3, 10));
        for value in [
            "bytes 30-99/100",
            "bytes 3-8/10",
            "bytes 3-9/*",
            "bytes 3-10/10",
        ] {
            assert!(!valid_range(value, 3, 10));
        }
    }

    fn serve(response: Vec<u8>, requests: usize) -> (String, std::thread::JoinHandle<()>) {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            for stream in server.incoming().take(requests) {
                let mut stream = stream.unwrap();
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request);
                stream.write_all(&response).unwrap();
            }
        });
        (url, handle)
    }

    #[test]
    fn ignored_range_restarts_and_hash_failure_never_commits() {
        let root = crate::staging::create("download-test").unwrap();
        let hash = sha256_bytes(b"complete");
        let id = sha256_bytes(format!("fixture:{hash}:8").as_bytes());
        fs::write(root.join(format!("{id}.part")), b"bad").unwrap();
        let (url, server) = serve(
            b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\ncomplete".to_vec(),
            1,
        );
        let result = fetch_at(
            &root,
            Download {
                identity: "fixture",
                urls: vec![url],
                bytes: 8,
                sha256: &hash,
            },
            &AtomicBool::new(false),
            |_, _, _| {},
        )
        .unwrap();
        assert_eq!(fs::read(result).unwrap(), b"complete");
        server.join().unwrap();
        let (url, server) = serve(
            b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\ncorrupt!".to_vec(),
            2,
        );
        assert!(fetch_at(
            &root,
            Download {
                identity: "corrupt",
                urls: vec![url],
                bytes: 8,
                sha256: &hash
            },
            &AtomicBool::new(false),
            |_, _, _| {}
        )
        .is_err());
        server.join().unwrap();
        crate::staging::remove(root).unwrap();
    }

    #[test]
    fn bad_range_fails_over_and_valid_206_resumes_original_prefix() {
        let root = crate::staging::create("range-failover-test").unwrap();
        let hash = sha256_bytes(b"complete");
        let id = sha256_bytes(format!("range:{hash}:8").as_bytes());
        fs::write(root.join(format!("{id}.part")), b"comp").unwrap();
        let (bad, first) = serve(b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-7/8\r\nContent-Length: 8\r\nConnection: close\r\n\r\nwrong!!!".to_vec(), 2);
        let (good, second) = serve(b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 4-7/8\r\nContent-Length: 4\r\nConnection: close\r\n\r\nlete".to_vec(), 1);
        let result = fetch_at(
            &root,
            Download {
                identity: "range",
                urls: vec![bad, good],
                bytes: 8,
                sha256: &hash,
            },
            &AtomicBool::new(false),
            |_, _, _| {},
        )
        .unwrap();
        assert_eq!(fs::read(result).unwrap(), b"complete");
        first.join().unwrap();
        second.join().unwrap();
        crate::staging::remove(root).unwrap();
    }

    #[test]
    fn cancelled_transfer_keeps_partial_but_never_publishes_verified_file() {
        let root = crate::staging::create("download-cancel-test").unwrap();
        let bytes = vec![42u8; 128 * 1024];
        let hash = sha256_bytes(&bytes);
        let id = sha256_bytes(format!("cancel:{hash}:{}", bytes.len()).as_bytes());
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        )
        .into_bytes();
        response.extend(&bytes);
        let (url, server) = serve(response, 1);
        let cancel = AtomicBool::new(false);
        let result = fetch_at(
            &root,
            Download {
                identity: "cancel",
                urls: vec![url],
                bytes: bytes.len() as u64,
                sha256: &hash,
            },
            &cancel,
            |_, _, _| {
                cancel.store(true, Ordering::Release);
            },
        );
        assert!(result.is_err());
        assert!(root.join(format!("{id}.part")).is_file());
        assert!(!root.join(format!("{id}.verified")).exists());
        server.join().unwrap();
        crate::staging::remove(root).unwrap();
    }
}
