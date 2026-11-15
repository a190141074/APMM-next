//! 全虚拟桌面取色。先捕获物理像素，再用顶层窗口接收点击，避免操作被取色的应用。
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::w;
use windows::Win32::{Foundation::*, Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW, UI::{HiDpi::*, WindowsAndMessaging::*}};
static ACTIVE: AtomicBool = AtomicBool::new(false);

struct Session { dc: HDC, bitmap: HBITMAP, previous: HGDIOBJ, dpi: DPI_AWARENESS_CONTEXT, width: i32, height: i32, color: Option<String> }
impl Drop for Session {
    fn drop(&mut self) { unsafe {
        if !self.dc.is_invalid() { SelectObject(self.dc, self.previous); let _ = DeleteDC(self.dc); }
        if !self.bitmap.is_invalid() { let _ = DeleteObject(self.bitmap.into()); }
        SetThreadDpiAwarenessContext(self.dpi);
    } }
}
struct ActiveGuard;
impl Drop for ActiveGuard { fn drop(&mut self) { ACTIVE.store(false, Ordering::Release); } }

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Session;
    if !ptr.is_null() {
        let session = &mut *ptr;
        match message {
            WM_PAINT => {
                let mut paint = PAINTSTRUCT::default();
                let target = BeginPaint(hwnd, &mut paint);
                let _ = BitBlt(target, 0, 0, session.width, session.height, Some(session.dc), 0, 0, SRCCOPY);
                let _ = EndPaint(hwnd, &paint);
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                let x = (lp.0 as u16) as i16 as i32;
                let y = ((lp.0 >> 16) as u16) as i16 as i32;
                let color = GetPixel(session.dc, x, y).0;
                if color != u32::MAX { session.color = Some(format!("#{:02x}{:02x}{:02x}", color & 255, (color >> 8) & 255, (color >> 16) & 255)); }
                let _ = DestroyWindow(hwnd); return LRESULT(0);
            }
            WM_RBUTTONDOWN | WM_CLOSE => { let _ = DestroyWindow(hwnd); return LRESULT(0); }
            WM_KEYDOWN if wp.0 == 27 => { let _ = DestroyWindow(hwnd); return LRESULT(0); }
            WM_DESTROY => { PostQuitMessage(0); return LRESULT(0); }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, message, wp, lp)
}

pub(crate) fn pick() -> Result<Option<String>, String> {
    if ACTIVE.swap(true, Ordering::AcqRel) { return Err("已有取色窗口，请先完成或取消。".into()); }
    let _active = ActiveGuard;
    unsafe {
        let dpi = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let (x, y, width, height) = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN), GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN));
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        let previous = SelectObject(dc, bitmap.into());
        let capture = BitBlt(dc, 0, 0, width, height, Some(screen), x, y, SRCCOPY | CAPTUREBLT);
        ReleaseDC(None, screen);
        let mut session = Session { dc, bitmap, previous, dpi, width, height, color: None };
        capture.map_err(|e| format!("屏幕捕获失败：{e}"))?;
        let instance: HINSTANCE = GetModuleHandleW(None).map_err(|e| e.to_string())?.into();
        let name = w!("APMM.ScreenColorPicker");
        let class = WNDCLASSW { lpfnWndProc: Some(procedure), hInstance: instance, lpszClassName: name, hCursor: LoadCursorW(None, IDC_CROSS).map_err(|e| e.to_string())?, ..Default::default() };
        // 已注册时复用同一类，窗口数据仅属于当前取色会话。
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW, name, w!("屏幕取色：点击选择，Esc 或右键取消"), WS_POPUP | WS_VISIBLE, x, y, width, height, None, None, Some(instance), Some(&mut session as *mut _ as _)).map_err(|e| e.to_string())?;
        let _ = SetForegroundWindow(hwnd);
        let mut message = MSG::default();
        loop {
            let status = GetMessageW(&mut message, None, 0, 0).0;
            if status <= 0 { if status < 0 { let _ = DestroyWindow(hwnd); return Err("取色消息处理失败".into()); } break; }
            let _ = TranslateMessage(&message); DispatchMessageW(&message);
        }
        Ok(session.color.take())
    }
}
