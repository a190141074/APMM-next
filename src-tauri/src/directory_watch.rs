//! 带路径的目录通知；重叠 I/O 的缓冲区保持固定地址，取消完成后才释放。
use crate::*;
use std::sync::{atomic::AtomicU64, mpsc};

#[derive(Default, Debug)]
pub(crate) struct Changes {
    pub paths: Vec<PathBuf>,
    pub full: bool,
    pub error: Option<String>,
}

pub(crate) struct WatchGroup {
    pub events: mpsc::Receiver<Changes>,
    pub generation: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl WatchGroup {
    pub fn start(roots: Vec<PathBuf>) -> Self {
        let (tx, events) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let generation = Arc::new(AtomicU64::new(0));
        let halt = stop.clone();
        let version = generation.clone();
        let thread = std::thread::spawn(move || {
            #[cfg(windows)]
            {
                let mut watchers = Vec::new();
                let mut retry = Instant::now();
                while !halt.load(Ordering::Acquire) {
                    if watchers.is_empty() && Instant::now() >= retry {
                        for root in &roots {
                            match native::Watch::open(root.clone()) {
                                Ok(w) => watchers.push(w),
                                Err(e) => {
                                    version.fetch_add(1, Ordering::AcqRel);
                                    let _ = tx.send(Changes {
                                        full: true,
                                        error: Some(e),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                        retry = Instant::now() + Duration::from_secs(5);
                    }
                    let mut broken = false;
                    for watcher in &mut watchers {
                        let change = match watcher.poll() {
                            Ok(change) => change,
                            Err(e) => {
                                broken = true;
                                Some(Changes {
                                    full: true,
                                    error: Some(e),
                                    ..Default::default()
                                })
                            }
                        };
                        if let Some(change) = change {
                            version.fetch_add(1, Ordering::AcqRel);
                            if tx.send(change).is_err() {
                                return;
                            }
                        }
                    }
                    if broken {
                        watchers.clear();
                    }
                    // 部分根建立失败时重试全部，不能永久漏掉其中一个目录。
                    if watchers.len() != roots.len() && Instant::now() >= retry {
                        watchers.clear();
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
            #[cfg(not(windows))]
            {
                let _ = roots;
                while !halt.load(Ordering::Acquire) {
                    version.fetch_add(1, Ordering::AcqRel);
                    if tx.send(Changes::default()).is_err() {
                        break;
                    }
                    for _ in 0..50 {
                        if halt.load(Ordering::Acquire) {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });
        Self {
            events,
            generation,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for WatchGroup {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// 解析通知时验证边界与相对路径，异常缓冲区要求完整核对。
fn decode(root: &Path, bytes: &[u8]) -> Changes {
    let mut result = Changes::default();
    if bytes.is_empty() {
        result.full = true;
        return result;
    }
    let mut offset = 0;
    loop {
        let Some(header) = bytes.get(offset..offset + 12) else {
            result.full = true;
            break;
        };
        let next = u32::from_le_bytes(header[0..4].try_into().unwrap()) as usize;
        let length = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
        let Some(name) = bytes.get(offset + 12..offset + 12 + length) else {
            result.full = true;
            break;
        };
        if length % 2 != 0 {
            result.full = true;
            break;
        }
        let text = String::from_utf16(
            &name
                .chunks_exact(2)
                .map(|s| u16::from_le_bytes([s[0], s[1]]))
                .collect::<Vec<_>>(),
        );
        let path = text.ok().and_then(|s| crate::vault::safe_relative_path(&s));
        if let Some(path) = path {
            result.paths.push(root.join(path));
        } else {
            result.full = true;
        }
        if next == 0 {
            break;
        }
        if next < 12 + length || next % 4 != 0 {
            result.full = true;
            break;
        }
        offset += next;
    }
    result.paths.sort();
    result.paths.dedup();
    result
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows::Win32::{
        Foundation::*,
        Storage::FileSystem::*,
        System::{Threading::*, IO::*},
    };
    pub(super) struct Watch {
        root: PathBuf,
        file: fs::File,
        identity: (u32, u32, u32),
        next_root_check: Instant,
        event: HANDLE,
        overlapped: Box<OVERLAPPED>,
        buffer: Box<[u32; 16384]>,
        pending: bool,
    }
    fn open_root(root: &Path, overlapped: bool) -> Result<fs::File, String> {
        fs::OpenOptions::new()
            .access_mode(FILE_LIST_DIRECTORY.0)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0)
            .custom_flags(
                FILE_FLAG_BACKUP_SEMANTICS.0
                    | if overlapped {
                        FILE_FLAG_OVERLAPPED.0
                    } else {
                        0
                    },
            )
            .open(root)
            .map_err(|e| format!("无法监听 {}：{e}", root.display()))
    }
    fn identity(file: &fs::File) -> Result<(u32, u32, u32), String> {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }
            .map_err(|e| e.to_string())?;
        Ok((
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
        ))
    }
    impl Watch {
        pub fn open(root: PathBuf) -> Result<Self, String> {
            let file = open_root(&root, true)?;
            let identity = identity(&file)?;
            let event =
                unsafe { CreateEventW(None, true, false, None) }.map_err(|e| e.to_string())?;
            let mut value = Self {
                root,
                file,
                identity,
                next_root_check: Instant::now() + Duration::from_secs(1),
                event,
                overlapped: Box::new(OVERLAPPED::default()),
                buffer: Box::new([0; 16384]),
                pending: false,
            };
            value.overlapped.hEvent = event;
            value.arm()?;
            Ok(value)
        }
        fn handle(&self) -> HANDLE {
            HANDLE(self.file.as_raw_handle())
        }
        fn arm(&mut self) -> Result<(), String> {
            unsafe {
                ResetEvent(self.event).map_err(|e| e.to_string())?;
                ReadDirectoryChangesW(
                    self.handle(),
                    self.buffer.as_mut_ptr().cast(),
                    65536,
                    true,
                    FILE_NOTIFY_CHANGE_FILE_NAME
                        | FILE_NOTIFY_CHANGE_DIR_NAME
                        | FILE_NOTIFY_CHANGE_SIZE
                        | FILE_NOTIFY_CHANGE_LAST_WRITE,
                    None,
                    Some(&mut *self.overlapped),
                    None,
                )
                .map_err(|e| e.to_string())?;
            }
            self.pending = true;
            Ok(())
        }
        pub fn poll(&mut self) -> Result<Option<Changes>, String> {
            // 路径通知只覆盖目录内部；根目录改名后旧句柄仍有效，必须另行核对身份。
            if Instant::now() >= self.next_root_check {
                self.next_root_check = Instant::now() + Duration::from_secs(1);
                if identity(&open_root(&self.root, false)?)? != self.identity {
                    return Err(format!("监听根目录已被替换：{}", self.root.display()));
                }
            }
            if unsafe { WaitForSingleObject(self.event, 0) } == WAIT_TIMEOUT {
                return Ok(None);
            }
            let mut size = 0;
            let result =
                unsafe { GetOverlappedResult(self.handle(), &*self.overlapped, &mut size, false) };
            self.pending = false;
            result.map_err(|e| e.to_string())?;
            let bytes = unsafe {
                std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), size as usize)
            };
            let value = decode(&self.root, bytes);
            self.arm()?;
            Ok(Some(value))
        }
    }
    impl Drop for Watch {
        fn drop(&mut self) {
            unsafe {
                if self.pending {
                    let _ = CancelIoEx(self.handle(), Some(&*self.overlapped));
                    let mut size = 0;
                    let _ = GetOverlappedResult(self.handle(), &*self.overlapped, &mut size, true);
                }
                let _ = CloseHandle(self.event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incremental_watch_overflow_and_malformed_require_full_scan() {
        assert!(decode(Path::new("root"), &[]).full);
        assert!(decode(Path::new("root"), &[0; 3]).full);
        let name = "folder\\image.png"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let mut bytes = vec![0; 12];
        bytes[8..12].copy_from_slice(&(name.len() as u32).to_le_bytes());
        bytes.extend(name);
        let event = decode(Path::new("root"), &bytes);
        assert!(!event.full);
        assert_eq!(event.paths.len(), 1);
    }
}
