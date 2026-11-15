#![cfg(target_os = "windows")]

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use windows::core::{implement, Error, Result};
use windows_core::{BOOL, HRESULT};
use windows::Win32::Foundation::POINT;
use windows::Win32::System::Com::{IDataObject, IDataObject_Impl, FORMATETC, STGMEDIUM, STGMEDIUM_0, IEnumFORMATETC, IEnumFORMATETC_Impl, IEnumSTATDATA, IAdviseSink, TYMED_HGLOBAL};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT};
use windows::Win32::System::Ole::{DoDragDrop, DROPEFFECT, DROPEFFECT_COPY, IDropSource, IDropSource_Impl, OleInitialize, OleUninitialize};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow, GetSystemMetrics, SM_CXDRAG, SM_CYDRAG};

const CF_HDROP: u16 = 15;
const DV_E_FORMATETC: HRESULT = HRESULT(0x80040064u32 as i32);
const E_NOTIMPL: HRESULT = HRESULT(0x80004001u32 as i32);
const DVASPECT_CONTENT: u32 = 1;
const DATADIR_GET: u32 = 1;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[link(name = "user32")]
extern "system" { fn RegisterClipboardFormatW(name: *const u16) -> u16; }

#[implement(IDropSource)]
struct DropSource { missed_button_checks: Cell<u8> }

impl IDropSource_Impl for DropSource_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, key_state: MODIFIERKEYS_FLAGS) -> HRESULT {
        // WebView2 can hand OLE a stale MK_LBUTTON bit when DoDragDrop starts
        // from a dispatched pointer session. The async Win32 state is the
        // authoritative button state for this process.
        let escape_down = escape.as_bool() || unsafe { GetAsyncKeyState(VK_ESCAPE.0 as i32) } < 0;
        let left_down = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } < 0;
        if escape_down {
            let detail = format!("escape=true async_left={} ole_key_state=0x{:x}", left_down, key_state.0);
            crate::write_log("info", "native_file_drag", "QueryContinueDrag cancelled", Some(&detail));
            return HRESULT(0x00040101);
        }
        if left_down || key_state.0 & 0x0001 != 0 {
            self.missed_button_checks.set(0);
            return HRESULT(0);
        }
        // A pointerdown-triggered OLE session can receive one or two stale
        // button samples while WebView2 hands the capture to OLE. Give the
        // physical button a short grace period before cancelling the drag.
        let missed = self.missed_button_checks.get().saturating_add(1);
        self.missed_button_checks.set(missed);
        if missed < 5 { return HRESULT(0); }
        let detail = format!("escape=false async_left=false ole_key_state=0x{:x} missed_checks={missed}", key_state.0);
        crate::write_log("info", "native_file_drag", "QueryContinueDrag cancelled", Some(&detail));
        return HRESULT(0x00040101);
    }
    fn GiveFeedback(&self, _effect: DROPEFFECT) -> HRESULT { HRESULT(0x00040102) }
}

#[implement(IEnumFORMATETC)]
struct FormatEnumerator { formats: Vec<FORMATETC>, index: Cell<usize> }

impl IEnumFORMATETC_Impl for FormatEnumerator_Impl {
    fn Next(&self, count: u32, out: *mut FORMATETC, fetched: *mut u32) -> HRESULT {
        if out.is_null() { return HRESULT(0x80004003u32 as i32); }
        let remaining = self.formats.len().saturating_sub(self.index.get());
        let take = remaining.min(count as usize);
        unsafe { std::ptr::copy_nonoverlapping(self.formats.as_ptr().add(self.index.get()), out, take); if !fetched.is_null() { *fetched = take as u32; } }
        self.index.set(self.index.get() + take);
        if take == count as usize { HRESULT(0) } else { HRESULT(1) }
    }
    fn Skip(&self, count: u32) -> Result<()> { self.index.set((self.index.get() + count as usize).min(self.formats.len())); Ok(()) }
    fn Reset(&self) -> Result<()> { self.index.set(0); Ok(()) }
    fn Clone(&self) -> Result<IEnumFORMATETC> { Ok(FormatEnumerator { formats: self.formats.clone(), index: Cell::new(self.index.get()) }.into()) }
}

#[implement(IDataObject)]
struct FileDataObject { hdrop: Vec<u8>, preferred_format: u16 }

impl IDataObject_Impl for FileDataObject_Impl {
    fn GetData(&self, format: *const FORMATETC) -> Result<STGMEDIUM> {
        unsafe {
            if format.is_null() || ((*format).tymed & TYMED_HGLOBAL.0 as u32) == 0 { return Err(Error::from(DV_E_FORMATETC)); }
            let preferred_bytes = DROPEFFECT_COPY.0.to_le_bytes();
            let (payload, size): (&[u8], usize) = if (*format).cfFormat == CF_HDROP { (&self.hdrop[..], self.hdrop.len()) } else if (*format).cfFormat == self.preferred_format { (&preferred_bytes[..], 4) } else { return Err(Error::from(DV_E_FORMATETC)); };
            let hglobal = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size)?;
            let dst = GlobalLock(hglobal);
            if dst.is_null() { return Err(Error::from_win32()); }
            std::ptr::copy_nonoverlapping(payload.as_ptr(), dst as *mut u8, size);
            let _ = GlobalUnlock(hglobal);
            Ok(STGMEDIUM { tymed: TYMED_HGLOBAL.0 as u32, u: STGMEDIUM_0 { hGlobal: hglobal }, pUnkForRelease: std::mem::ManuallyDrop::new(None) })
        }
    }
    fn GetDataHere(&self, _format: *const FORMATETC, _medium: *mut STGMEDIUM) -> Result<()> { Err(Error::from(E_NOTIMPL)) }
    fn QueryGetData(&self, format: *const FORMATETC) -> HRESULT { unsafe { if !format.is_null() && ((*format).cfFormat == CF_HDROP || (*format).cfFormat == self.preferred_format) && ((*format).tymed & TYMED_HGLOBAL.0 as u32) != 0 { HRESULT(0) } else { DV_E_FORMATETC } } }
    fn GetCanonicalFormatEtc(&self, _input: *const FORMATETC, output: *mut FORMATETC) -> HRESULT { unsafe { if !output.is_null() { (*output).ptd = std::ptr::null_mut(); } } HRESULT(0x8004006Au32 as i32) }
    fn SetData(&self, _format: *const FORMATETC, _medium: *const STGMEDIUM, _release: BOOL) -> Result<()> { Err(Error::from(E_NOTIMPL)) }
    fn EnumFormatEtc(&self, direction: u32) -> Result<IEnumFORMATETC> {
        if direction != DATADIR_GET { return Err(Error::from(E_NOTIMPL)); }
        let tymed = TYMED_HGLOBAL.0 as u32;
        Ok(FormatEnumerator { formats: vec![
            FORMATETC { cfFormat: CF_HDROP, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT, lindex: -1, tymed },
            FORMATETC { cfFormat: self.preferred_format, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT, lindex: -1, tymed },
        ], index: Cell::new(0) }.into())
    }
    fn DAdvise(&self, _format: *const FORMATETC, _flags: u32, _sink: windows_core::Ref<'_, IAdviseSink>) -> Result<u32> { Err(Error::from(E_NOTIMPL)) }
    fn DUnadvise(&self, _connection: u32) -> Result<()> { Err(Error::from(E_NOTIMPL)) }
    fn EnumDAdvise(&self) -> Result<IEnumSTATDATA> { Err(Error::from(E_NOTIMPL)) }
}

fn shell_path(path: &Path) -> Result<PathBuf> {
    let raw = path.to_string_lossy();
    if raw.starts_with(r"\\?\UNC\") { return Ok(PathBuf::from(format!(r"\\{}", &raw[8..]))); }
    if raw.starts_with(r"\\?\") { return Ok(PathBuf::from(&raw[4..])); }
    Ok(path.to_path_buf())
}

fn make_hdrop(path: &Path) -> Vec<u8> {
    let wide: Vec<u16> = path.to_string_lossy().encode_utf16().chain(Some(0)).chain(Some(0)).collect();
    let mut bytes = vec![0u8; 20 + wide.len() * 2];
    bytes[0..4].copy_from_slice(&20u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, bytes.as_mut_ptr().add(20), wide.len() * 2); }
    bytes
}

fn validate_source(raw: &str) -> std::result::Result<PathBuf, String> {
    if raw.trim().is_empty() { return Err("路径校验失败：Mod 文件路径为空".into()); }
    let canonical = Path::new(raw).canonicalize()
        .map_err(|error| format!("路径校验失败：无法解析 Mod 文件：{error}"))?;
    if !canonical.is_file() { return Err("路径校验失败：Mod 路径不是文件".into()); }
    if !canonical.extension().and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png")) {
        return Err("路径校验失败：只能拖出 PNG Mod".into());
    }
    let settings = crate::load_library().settings;
    let mut roots = vec![crate::vault_dir()];
    if !settings.modimg_path.trim().is_empty() {
        roots.push(PathBuf::from(settings.modimg_path));
    }
    let allowed = roots.into_iter().filter_map(|root| root.canonicalize().ok())
        .any(|root| canonical.starts_with(root));
    if !allowed { return Err("路径校验失败：文件不在受控 Mod 目录内".into()); }
    let shell = shell_path(&canonical).map_err(|error| format!("Shell 路径转换失败：{error}"))?;
    if !shell.is_file() { return Err("Shell 路径校验失败：转换后的 PNG 文件不可访问".into()); }
    Ok(shell)
}

#[derive(Debug, PartialEq, Eq)]
enum GestureDecision { Wait, Cancel(&'static str), Start }

fn gesture_decision(held: bool, escape: bool, focused: bool, dx: i32, dy: i32, cx: i32, cy: i32) -> GestureDecision {
    if escape { return GestureDecision::Cancel("Escape"); }
    if !held { return GestureDecision::Cancel("left button released"); }
    if !focused { return GestureDecision::Cancel("window lost focus"); }
    if dx.abs() >= cx.max(1) || dy.abs() >= cy.max(1) { GestureDecision::Start }
    else { GestureDecision::Wait }
}

struct ActiveSession;
impl Drop for ActiveSession {
    fn drop(&mut self) { ACTIVE.store(false, Ordering::Release); }
}

fn log_session(level: &str, id: u64, event: &str, detail: &str) {
    crate::write_log(level, "native_file_drag", &format!("session={id} {event}"), Some(detail));
}

pub fn run_session(id: u64, path: PathBuf, hwnd: isize, frontend_x: Option<f64>, frontend_y: Option<f64>) {
    let _active = ActiveSession;
    let mut start = POINT::default();
    // Use the screen coordinate captured synchronously by pointerdown. Falling
    // back to GetCursorPos keeps the command usable from older frontends.
    if let (Some(x), Some(y)) = (frontend_x, frontend_y) {
        if x.is_finite() && y.is_finite() {
            start.x = x.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
            start.y = y.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
        } else if let Err(error) = unsafe { GetCursorPos(&mut start) } {
            log_session("error", id, "cancelled", &format!("initial cursor unavailable: {error}"));
            return;
        }
    } else if let Err(error) = unsafe { GetCursorPos(&mut start) } {
        log_session("error", id, "cancelled", &format!("initial cursor unavailable: {error}"));
        return;
    }
    let cx = unsafe { GetSystemMetrics(SM_CXDRAG) }.max(1);
    let cy = unsafe { GetSystemMetrics(SM_CYDRAG) }.max(1);
    log_session("info", id, "pointerdown", &format!(
        "native=({}, {}) frontend=({:?}, {:?}) threshold=({}, {}) path={}",
        start.x, start.y, frontend_x, frontend_y, cx, cy, path.display()
    ));
    let begun = Instant::now();
    loop {
        let held = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } < 0;
        let escape = unsafe { GetAsyncKeyState(VK_ESCAPE.0 as i32) } < 0;
        let focused = unsafe { GetForegroundWindow().0 as isize == hwnd };
        let mut cursor = POINT::default();
        if let Err(error) = unsafe { GetCursorPos(&mut cursor) } {
            log_session("error", id, "cancelled", &format!("cursor unavailable: {error}"));
            return;
        }
        let dx = cursor.x - start.x;
        let dy = cursor.y - start.y;
        match gesture_decision(held, escape, focused, dx, dy, cx, cy) {
            GestureDecision::Cancel(reason) => {
                log_session("info", id, "cancelled", &format!("reason={reason} delta=({dx},{dy})"));
                return;
            }
            GestureDecision::Start => {
                log_session("info", id, "threshold passed", &format!("delta=({dx},{dy}) elapsed_ms={}", begun.elapsed().as_millis()));
                break;
            }
            GestureDecision::Wait => {}
        }
        if begun.elapsed() > Duration::from_secs(30) {
            log_session("info", id, "cancelled", "reason=gesture timeout");
            return;
        }
        thread::sleep(Duration::from_millis(4));
    }
    // DoDragDrop must run on the window/UI thread that owns the pointer gesture.
    // OleInitialize also initializes the STA OLE apartment for this call.
    if let Err(error) = unsafe { OleInitialize(None) } {
        log_session("error", id, "OleInitialize failed", &format!("HRESULT={:#010x} {error}", error.code().0 as u32));
        return;
    }
    log_session("info", id, "OLE initialized", "OleInitialize succeeded on the Tauri window thread");
    unsafe {
        let preferred_name: Vec<u16> = "Preferred DropEffect".encode_utf16().chain(Some(0)).collect();
        let preferred_format = RegisterClipboardFormatW(preferred_name.as_ptr());
        if preferred_format == 0 {
            log_session("error", id, "data object creation failed", "RegisterClipboardFormatW returned 0");
            OleUninitialize();
            return;
        }
        {
            let data: IDataObject = FileDataObject { hdrop: make_hdrop(&path), preferred_format }.into();
            let source: IDropSource = DropSource { missed_button_checks: Cell::new(0) }.into();
            log_session("info", id, "OLE started", &format!("CF_HDROP IDataObject created path={}", path.display()));
            let mut effect = DROPEFFECT(0);
            let hresult = DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect);
            log_session("info", id, "OLE ended", &format!("HRESULT={:#010x} effect={}", hresult.0 as u32, effect.0));
        }
        OleUninitialize();
    }
}

pub fn start(path: String, frontend_x: Option<f64>, frontend_y: Option<f64>) -> std::result::Result<(u64, PathBuf), String> {
    let id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    log_session("info", id, "pointerdown received", &format!("frontend=({frontend_x:?},{frontend_y:?})"));
    let path = validate_source(&path).map_err(|error| {
        log_session("error", id, "cancelled", &error);
        error
    })?;
    log_session("info", id, "path validated", &path.to_string_lossy());
    // Check before scheduling any blocking work on the window thread.
    if ACTIVE.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return Err("上一拖拽会话尚未结束".into());
    }
    Ok((id, path))
}

pub fn abort_start(id: u64, reason: &str) {
    ACTIVE.store(false, Ordering::Release);
    log_session("error", id, "cancelled", &format!("reason={reason}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gesture_waits_until_system_threshold() {
        assert_eq!(gesture_decision(true, false, true, 3, 3, 4, 4), GestureDecision::Wait);
        assert_eq!(gesture_decision(true, false, true, 4, 0, 4, 4), GestureDecision::Start);
        assert_eq!(gesture_decision(true, false, true, 0, -4, 4, 4), GestureDecision::Start);
    }
    #[test]
    fn release_escape_and_focus_cancel() {
        assert_eq!(gesture_decision(false, false, true, 10, 0, 4, 4), GestureDecision::Cancel("left button released"));
        assert_eq!(gesture_decision(true, true, true, 10, 0, 4, 4), GestureDecision::Cancel("Escape"));
        assert_eq!(gesture_decision(true, false, false, 10, 0, 4, 4), GestureDecision::Cancel("window lost focus"));
    }
    #[test]
    fn hdrop_contains_unicode_path_and_double_nul() {
        let bytes = make_hdrop(Path::new(r"C:\测试\mod.png"));
        assert_eq!(u32::from_le_bytes(bytes[0..4].try_into().unwrap()), 20);
        assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 1);
        assert_eq!(&bytes[bytes.len() - 4..], &[0, 0, 0, 0]);
    }
    #[test]
    fn shell_path_removes_extended_prefix() {
        assert_eq!(shell_path(Path::new(r"\\?\C:\mods\a.png")).unwrap(), PathBuf::from(r"C:\mods\a.png"));
        assert_eq!(shell_path(Path::new(r"\\?\UNC\server\share\a.png")).unwrap(), PathBuf::from(r"\\server\share\a.png"));
    }
}
