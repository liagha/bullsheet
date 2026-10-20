pub struct Decorations;

impl Decorations {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "windows")]
impl Decorations {
    pub fn apply(&self, frame: &mut eframe::Frame) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::Graphics::Dwm::{
            DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GWL_STYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WS_CAPTION, WS_SYSMENU,
        };
        let Ok(handle) = frame.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(window) = handle.as_raw() else {
            return;
        };
        let window_handle = window.hwnd.get() as *mut core::ffi::c_void;
        let style = unsafe { GetWindowLongPtrW(window_handle, GWL_STYLE) };
        if style & ((WS_CAPTION | WS_SYSMENU) as isize) != 0 {
            unsafe {
                let _ = SetWindowLongPtrW(
                    window_handle,
                    GWL_STYLE,
                    style & !((WS_CAPTION | WS_SYSMENU) as isize),
                );
                let _ = SetWindowPos(
                    window_handle,
                    std::ptr::null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
        let corner = DWMWCP_ROUND;
        let _ = unsafe {
            DwmSetWindowAttribute(
                window_handle,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &corner as *const i32 as *const core::ffi::c_void,
                std::mem::size_of::<i32>() as u32,
            )
        };
    }
}

#[cfg(not(target_os = "windows"))]
impl Decorations {
    pub fn apply(&self, _frame: &mut eframe::Frame) {}
}
