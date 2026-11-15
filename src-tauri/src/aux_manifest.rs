//! 辅助插件信任边界：签名先于解析，文件清单先于启动和安装。
use crate::*;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub schema: u32,
    pub id: String,
    pub version: String,
    pub app_range: String,
    pub protocol: u32,
    pub platform: String,
    pub entrypoint: String,
    pub files: Vec<PluginFile>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PluginFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}
pub fn config() -> serde_json::Value {
    serde_json::from_str(include_str!("../../config/auxiliary.json")).expect("辅助插件配置无效")
}
fn unhex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("签名编码无效".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}
pub fn verify(raw: &[u8], signature: &str) -> Result<PluginManifest, String> {
    if raw.len() > 2 * 1024 * 1024 {
        return Err("插件清单过大".into());
    }
    let cfg = config();
    let public: [u8; 32] = unhex(cfg["publicKey"].as_str().ok_or("缺少插件公钥")?)?
        .try_into()
        .map_err(|_| "公钥长度无效")?;
    let key = VerifyingKey::from_bytes(&public).map_err(|e| e.to_string())?;
    let signature = Signature::from_slice(&unhex(signature.trim())?).map_err(|e| e.to_string())?;
    key.verify(raw, &signature)
        .map_err(|_| "插件签名校验失败，拒绝安装或执行")?;
    let m: PluginManifest = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
    if m.schema != 1 || m.protocol != 1 || m.id != "asset-tool" || m.platform != "windows-x86_64" {
        return Err("插件标识、协议或平台不兼容".into());
    }
    let version = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
    if !semver::VersionReq::parse(&m.app_range)
        .map_err(|e| e.to_string())?
        .matches(&version)
    {
        return Err("插件与当前 APMM 版本不兼容".into());
    }
    semver::Version::parse(&m.version).map_err(|e| e.to_string())?;
    if m.entrypoint != "apmm-asset-tool.exe"
        || !m.files.iter().any(|f| f.path == m.entrypoint)
        || m.files.len() > 10000
        || m.files.is_empty()
    {
        return Err("插件入口或文件数量无效".into());
    }
    let mut seen = HashSet::new();
    let mut bytes = 0u64;
    for f in &m.files {
        if safe_relative_path(&f.path).is_none()
            || !seen.insert(f.path.to_ascii_lowercase())
            || f.sha256.len() != 64
            || !f.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("插件文件清单无效或存在重复路径".into());
        }
        bytes = bytes.checked_add(f.bytes).ok_or("插件大小溢出")?;
    }
    if bytes > 2 * 1024 * 1024 * 1024 {
        return Err("插件解压大小超限".into());
    }
    Ok(m)
}
pub fn load(root: &Path) -> Result<PluginManifest, String> {
    let raw = fs::read(root.join("plugin.json")).map_err(|e| e.to_string())?;
    let sig = fs::read_to_string(root.join("plugin.sig")).map_err(|e| e.to_string())?;
    verify(&raw, &sig)
}
pub fn validate_files(root: &Path, m: &PluginManifest, cancel: &AtomicBool) -> Result<(), String> {
    let real = root.canonicalize().map_err(|e| e.to_string())?;
    for f in &m.files {
        if cancel.load(Ordering::Acquire) {
            return Err("插件校验已取消".into());
        }
        let path = root.join(&f.path);
        if !path
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(&real)
            || path.is_symlink()
            || !crate::downloads::checked(&path, f.bytes, &f.sha256)
        {
            return Err(format!("插件文件校验失败：{}", f.path));
        }
    }
    Ok(())
}
pub fn unpack(
    zip_path: &Path,
    stage: &Path,
    cancel: &AtomicBool,
) -> Result<PluginManifest, String> {
    let mut archive = zip::ZipArchive::new(fs::File::open(zip_path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let raw = {
        let f = archive
            .by_name("plugin.json")
            .map_err(|_| "插件包缺少签名清单")?;
        let mut bytes = vec![];
        f.take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        bytes
    };
    let sig = {
        let f = archive
            .by_name("plugin.sig")
            .map_err(|_| "插件包缺少签名")?;
        let mut value = String::new();
        f.take(1024)
            .read_to_string(&mut value)
            .map_err(|e| e.to_string())?;
        value
    };
    let m = verify(&raw, &sig)?;
    if archive.len() != m.files.len() + 2 {
        return Err("插件包包含清单以外的文件".into());
    }
    let mut names = HashSet::new();
    for i in 0..archive.len() {
        let e = archive.by_index(i).map_err(|e| e.to_string())?;
        if !names.insert(e.name().to_lowercase()) {
            return Err("插件 ZIP 中存在重复路径".into());
        }
    }
    for f in &m.files {
        if cancel.load(Ordering::Acquire) {
            return Err("插件安装已取消".into());
        }
        let mut entry = archive.by_name(&f.path).map_err(|e| e.to_string())?;
        if entry.size() != f.bytes
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("插件文件大小或类型不符".into());
        }
        let target = stage.join(&f.path);
        ensure_dir(target.parent().ok_or("插件路径无效")?).map_err(|e| e.to_string())?;
        let mut file = fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
    }
    fs::write(stage.join("plugin.json"), raw).map_err(|e| e.to_string())?;
    fs::write(stage.join("plugin.sig"), sig).map_err(|e| e.to_string())?;
    validate_files(stage, &m, cancel)?;
    Ok(m)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "需要 APMM_PLUGIN_ZIP 指向本次签名产物"]
    fn signed_artifact_roundtrip_rejects_tampering() {
        let zip = PathBuf::from(std::env::var("APMM_PLUGIN_ZIP").expect("请指定独立插件 ZIP"));
        let stage = crate::staging::create("signed-plugin-test").unwrap();
        let manifest = unpack(&zip, &stage, &AtomicBool::new(false)).unwrap();
        validate_files(&stage, &manifest, &AtomicBool::new(false)).unwrap();
        fs::write(stage.join(&manifest.entrypoint), b"tampered executable").unwrap();
        assert!(validate_files(&stage, &manifest, &AtomicBool::new(false)).is_err());
        let raw = fs::read(stage.join("plugin.json")).unwrap();
        let mut corrupted = raw.clone();
        corrupted[0] ^= 1;
        let signature = fs::read_to_string(stage.join("plugin.sig")).unwrap();
        assert!(verify(&corrupted, &signature).is_err());
        crate::staging::remove(stage).unwrap();
    }
}
