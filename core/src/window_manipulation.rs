#[cfg(windows)]
use std::ffi::OsStr;

////////////////
/// fake borderless mode implementation 
///////////////

#[cfg(windows)]
pub fn apply_borderless_fullscreen(hwnd: isize) {
    extern "system" {
        fn GetWindowLongW(hwnd: isize, index: i32) -> i32;
        fn SetWindowLongW(hwnd: isize, index: i32, new_long: i32) -> i32;
        fn SetWindowPos(
            hwnd: isize,
            hwnd_insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
        fn GetSystemMetrics(index: i32) -> i32;
    }

    const GWL_STYLE: i32 = -16;
    const WS_CAPTION: i32 = 0x00C00000;
    const WS_THICKFRAME: i32 = 0x00040000;
    const WS_MINIMIZEBOX: i32 = 0x00020000;
    const WS_MAXIMIZEBOX: i32 = 0x00010000;
    const WS_SYSMENU: i32 = 0x00080000;

    const SWP_FRAMECHANGED: u32 = 0x0020;
    const SWP_SHOWWINDOW: u32 = 0x0040;

    const SM_CXSCREEN: i32 = 0;
    const SM_CYSCREEN: i32 = 1;

    unsafe {
        let mut style = GetWindowLongW(hwnd, GWL_STYLE);
        style &= !WS_CAPTION;
        style &= !WS_THICKFRAME;
        style &= !WS_MINIMIZEBOX;
        style &= !WS_MAXIMIZEBOX;
        style &= !WS_SYSMENU;

        SetWindowLongW(hwnd, GWL_STYLE, style);

        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);

        // +1 hack to prevent exclusive fullscreen detection
        SetWindowPos(
            hwnd,
            0,
            0,
            0,
            width,
            height + 1,
            SWP_FRAMECHANGED | SWP_SHOWWINDOW,
        );
    }
}

/// set the title of a Roblox window.
#[cfg(windows)]
pub fn set_window_title(hwnd: isize, title: &str) {
    use std::os::windows::ffi::OsStrExt;

    extern "system" {
        fn SetWindowTextW(hwnd: isize, text: *const u16) -> i32;
    }

    let wide: Vec<u16> = OsStr::new(title)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        SetWindowTextW(hwnd, wide.as_ptr());
    }
}

/// get the current window title.
#[cfg(windows)]
pub fn get_window_title(hwnd: isize) -> String {
    use std::os::windows::ffi::OsStringExt;

    extern "system" {
        fn GetWindowTextW(hwnd: isize, text: *mut u16, max_count: i32) -> i32;
    }

    let mut buf = [0u16; 256];
    unsafe {
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 256);
        if len <= 0 {
            return String::new();
        }
        std::ffi::OsString::from_wide(&buf[..len as usize])
            .to_string_lossy()
            .to_string()
    }
}

/// set the window icon by sending WM_SETICON.
#[cfg(windows)]
pub fn set_window_icon(hwnd: isize, icon_path: &str) {
    extern "system" {
        fn LoadImageW(
            hinst: isize,
            name: *const u16,
            kind: u32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> isize;
        fn SendMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
    }

    const IMAGE_ICON: u32 = 1;
    const LR_LOADFROMFILE: u32 = 0x0010;
    const LR_DEFAULTSIZE: u32 = 0x0040;
    const WM_SETICON: u32 = 0x0080;
    const ICON_SMALL: usize = 0;
    const ICON_BIG: usize = 1;

    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = OsStr::new(icon_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let hicon = LoadImageW(
            0,
            wide.as_ptr(),
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        );
        if hicon != 0 {
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL, hicon);
            SendMessageW(hwnd, WM_SETICON, ICON_BIG, hicon);
        }
    }
}

// stubs for non-Windows
#[cfg(not(windows))]
pub fn apply_borderless_fullscreen(_hwnd: isize) {}

#[cfg(not(windows))]
pub fn set_window_title(_hwnd: isize, _title: &str) {}

#[cfg(not(windows))]
pub fn get_window_title(_hwnd: isize) -> String {
    String::new()
}

#[cfg(not(windows))]
pub fn set_window_icon(_hwnd: isize, _icon_path: &str) {}
