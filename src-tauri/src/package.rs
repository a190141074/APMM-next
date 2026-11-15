//! Package format and runtime update payload boundary.

pub(crate) const RUNTIME_FILES: [&str; 4] = [
    "astral-mod-manager.exe",
    "astral-mod-manager.exe.manifest",
    "WebView2Loader.dll",
    "api-ms-win-core-synch-l1-2-0.dll",
];

pub(crate) fn is_runtime_file(name: &str) -> bool {
    RUNTIME_FILES.iter().any(|candidate| *candidate == name)
}
