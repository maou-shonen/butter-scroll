use crate::app::AppCommand;
use crossbeam_channel::Sender;

#[cfg(target_os = "windows")]
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::HBRUSH;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    GetCursorPos, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    TrackPopupMenu, CW_USEDEFAULT, HMENU, IDI_APPLICATION, MF_SEPARATOR, MF_STRING,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, WM_APP, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_NULL,
    WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
};

#[cfg(target_os = "windows")]
use crate::util::{to_wide, to_wide_fixed};

#[cfg(target_os = "windows")]
const WM_TRAYICON: u32 = WM_APP + 1;
#[cfg(target_os = "windows")]
const MENU_TOGGLE_ENABLED: usize = 1001;
#[cfg(target_os = "windows")]
const MENU_RELOAD_CONFIG: usize = 1002;
#[cfg(target_os = "windows")]
const MENU_TOGGLE_AUTOSTART: usize = 1003;
#[cfg(target_os = "windows")]
const MENU_EXIT: usize = 1004;

#[cfg(target_os = "windows")]
static APP_TX: OnceLock<Sender<AppCommand>> = OnceLock::new();

#[cfg(target_os = "windows")]
pub struct TrayIcon {
    hwnd: HWND,
    menu: HMENU,
    nid: NOTIFYICONDATAW,
}

#[cfg(target_os = "windows")]
impl TrayIcon {
    pub fn create(app_tx: Sender<AppCommand>) -> Result<Self, String> {
        let _ = APP_TX.set(app_tx);

        // Register lightweight hidden window class.
        let class_name = to_wide("SmoothScrollTrayClass");
        // SAFETY: null = current module
        let h_instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: 0 as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        // SAFETY: class struct points to valid static data during call.
        unsafe {
            RegisterClassW(&wc);
        }

        // SAFETY: creating hidden message-only style window (regular hidden overlapped).
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                WS_OVERLAPPED,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                h_instance,
                std::ptr::null(),
            )
        };

        if hwnd.is_null() {
            return Err("CreateWindowExW failed for tray window".to_string());
        }

        // SAFETY: CreatePopupMenu returns valid handle or null.
        let menu = unsafe { CreatePopupMenu() };
        if menu.is_null() {
            // SAFETY: hwnd created above.
            unsafe { DestroyWindow(hwnd) };
            return Err("CreatePopupMenu failed".to_string());
        }

        // SAFETY: menu handle valid.
        unsafe {
            AppendMenuW(
                menu,
                MF_STRING,
                MENU_TOGGLE_ENABLED,
                to_wide("啟用 / 停用").as_ptr(),
            );
            AppendMenuW(
                menu,
                MF_STRING,
                MENU_RELOAD_CONFIG,
                to_wide("重新載入設定").as_ptr(),
            );
            AppendMenuW(
                menu,
                MF_STRING,
                MENU_TOGGLE_AUTOSTART,
                to_wide("切換開機自啟").as_ptr(),
            );
            AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
            AppendMenuW(menu, MF_STRING, MENU_EXIT, to_wide("離開").as_ptr());
        }

        // SAFETY: built-in application icon.
        let icon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: icon,
            szTip: to_wide_fixed::<128>("Smooth Scroll"),
            ..unsafe { std::mem::zeroed() }
        };

        // SAFETY: nid is fully initialized.
        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &mut nid) };
        if ok == 0 {
            // SAFETY: cleanup handles.
            unsafe {
                DestroyMenu(menu);
                DestroyWindow(hwnd);
            }
            return Err("Shell_NotifyIconW(NIM_ADD) failed".to_string());
        }

        Ok(Self { hwnd, menu, nid })
    }
}

#[cfg(target_os = "windows")]
impl Drop for TrayIcon {
    fn drop(&mut self) {
        // SAFETY: handles belong to this struct.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &mut self.nid);
            DestroyMenu(self.menu);
            DestroyWindow(self.hwnd);
        }
    }
}

#[cfg(target_os = "windows")]
extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => 0,
        WM_TRAYICON => {
            if lparam as u32 == WM_RBUTTONUP {
                show_context_menu(hwnd);
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as usize;
            if let Some(tx) = APP_TX.get() {
                let _ = match id {
                    MENU_TOGGLE_ENABLED => tx.send(AppCommand::ToggleEnabled),
                    MENU_RELOAD_CONFIG => tx.send(AppCommand::ReloadConfig),
                    MENU_TOGGLE_AUTOSTART => tx.send(AppCommand::ToggleAutostart),
                    MENU_EXIT => tx.send(AppCommand::Exit),
                    _ => Ok(()),
                };
            }
            0
        }
        WM_DESTROY => {
            // SAFETY: signals end of GUI message loop.
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => {
            // SAFETY: default message handling.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }
}

#[cfg(target_os = "windows")]
fn show_context_menu(hwnd: HWND) {
    // SAFETY: menu belongs to window; retrieved from window userdata is omitted,
    // so we rebuild a temporary menu for reliability.
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return;
    }
    // SAFETY: valid menu and UTF-16 strings.
    unsafe {
        AppendMenuW(
            menu,
            MF_STRING,
            MENU_TOGGLE_ENABLED,
            to_wide("啟用 / 停用").as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            MENU_RELOAD_CONFIG,
            to_wide("重新載入設定").as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            MENU_TOGGLE_AUTOSTART,
            to_wide("切換開機自啟").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(menu, MF_STRING, MENU_EXIT, to_wide("離開").as_ptr());
    }

    let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
    // SAFETY: output point pointer valid.
    unsafe {
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        // Workaround for menu dismissal behavior in tray windows.
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
    }
}

#[cfg(not(target_os = "windows"))]
pub struct TrayIcon;

#[cfg(not(target_os = "windows"))]
impl TrayIcon {
    pub fn create(_app_tx: Sender<AppCommand>) -> Result<Self, String> {
        Ok(Self)
    }
}
