//! 文件校验的可重建缓存。元数据只用于快速筛选，强制校验和实际应用不信任缓存。
use crate::*;
use std::collections::BTreeMap;

const SCHEMA: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Stamp {
    pub len: u64,
    pub modified: u128,
    pub identity: String,
}

impl Stamp {
    pub fn read(path: &Path) -> Result<Self, String> {
        let meta = fs::symlink_metadata(path).map_err(|e| format!("{}：{e}", path.display()))?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(format!("不是普通文件：{}", path.display()));
        }
        Ok(Self {
            len: meta.len(),
            modified: meta
                .modified()
                .map_err(|e| e.to_string())?
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos(),
            identity: crate::directory_files::file_identity(path)
                .ok_or_else(|| format!("无法确认文件标识：{}", path.display()))?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct VerifiedFile {
    pub stamp: Stamp,
    pub hash: String,
    pub png: bool,
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerificationStats {
    pub checked: usize,
    pub reused: usize,
    pub content_reads: usize,
    pub decoded: usize,
    pub hashed_bytes: u64,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub(crate) struct VerificationCache {
    schema: u32,
    context: String,
    pub entries: BTreeMap<PathBuf, VerifiedFile>,
}

impl VerificationCache {
    pub fn load(path: &Path, context: String) -> Self {
        fs::read(path)
            .ok()
            .filter(|bytes| bytes.len() <= 64 * 1024 * 1024)
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(|saved| saved.schema == SCHEMA && saved.context == context)
            .unwrap_or_else(|| Self {
                schema: SCHEMA,
                context,
                entries: BTreeMap::new(),
            })
    }

    pub fn cached(&self, path: &Path, stamp: &Stamp, png: bool) -> Option<&VerifiedFile> {
        self.entries.get(path).filter(|v| {
            &v.stamp == stamp
                && (!png || v.png)
                && v.hash.len() == 64
                && v.hash.bytes().all(|b| b.is_ascii_hexdigit())
        })
    }

    /// 读前、读后核对文件标识；重新验证失败时撤销旧缓存，候选记录由资源库保留。
    pub fn verify(
        &mut self,
        path: &Path,
        png: bool,
        force: bool,
        stats: &mut VerificationStats,
    ) -> Result<VerifiedFile, String> {
        let stamp = match Stamp::read(path) {
            Ok(stamp) => stamp,
            Err(error) => {
                self.entries.remove(path);
                return Err(error);
            }
        };
        stats.checked += 1;
        if !force {
            if let Some(hit) = self.cached(path, &stamp, png) {
                stats.reused += 1;
                return Ok(hit.clone());
            }
        }
        // 强制校验可能发现保留全部元数据的坏图，不能在下次启动再次命中已否定的结果。
        self.entries.remove(path);
        let hash = sha256_file(path)?;
        stats.content_reads += 1;
        stats.hashed_bytes += stamp.len;
        if png {
            crate::image_import::validate_png(path)?;
            stats.decoded += 1;
        }
        if Stamp::read(path)? != stamp {
            return Err(format!("文件仍在写入，等待稳定后重试：{}", path.display()));
        }
        let value = VerifiedFile { stamp, hash, png };
        self.entries.insert(path.to_path_buf(), value.clone());
        Ok(value)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        write_cache(path, self)
    }
}

/// 同目录原子替换，缓存写入失败不能影响已经提交的资源库。
pub(crate) fn write_cache(path: &Path, value: &impl Serialize) -> Result<(), String> {
    ensure_dir(path.parent().ok_or("缓存路径缺少目录")?).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let temp = path.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let result = {
        use std::os::windows::ffi::OsStrExt;
        use windows::{core::PCWSTR, Win32::Storage::FileSystem::*};
        let a = temp
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let b = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        unsafe {
            MoveFileExW(
                PCWSTR(a.as_ptr()),
                PCWSTR(b.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
            .map_err(|e| e.to_string())
        }
    };
    #[cfg(not(windows))]
    let result = fs::rename(&temp, path).map_err(|e| e.to_string());
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

pub(crate) fn library_token(lib: &Library) -> String {
    // JSON 对象按键排序，不能把 HashMap 的随机迭代顺序当成库修改。
    let value = serde_json::to_value(lib).expect("库可序列化");
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&value).expect("JSON 可序列化"))
    )
}
