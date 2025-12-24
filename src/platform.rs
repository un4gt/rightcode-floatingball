use iced::window::raw_window_handle::WindowHandle;

#[cfg(windows)]
use iced::window::raw_window_handle::RawWindowHandle;

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::{CreateEllipticRgn, DeleteObject, SetWindowRgn},
    UI::WindowsAndMessaging::GetClientRect,
};

pub fn set_round_window_region(handle: WindowHandle<'_>, round: bool) {
    #[cfg(windows)]
    set_round_window_region_windows(handle, round);

    #[cfg(not(windows))]
    {
        let _ = (handle, round);
    }
}

#[cfg(windows)]
fn set_round_window_region_windows(handle: WindowHandle<'_>, round: bool) {
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };

    let hwnd = win32.hwnd.get() as HWND;

    unsafe {
        if !round {
            let _ = SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
            return;
        }

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };

        if GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 {
            return;
        }

        let region = CreateEllipticRgn(0, 0, width, height);
        if region.is_null() {
            return;
        }

        if SetWindowRgn(hwnd, region, 1) == 0 {
            let _ = DeleteObject(region);
        }
    }
}
