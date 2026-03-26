use crate::traits::EngineCommand;
use crossbeam_channel::Sender;

#[cfg(target_os = "windows")]
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetAncestor, GetWindowThreadProcessId, SetWindowsHookExW, UnhookWindowsHookEx,
    WindowFromPoint, GA_ROOT, HC_ACTION, HHOOK, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_MOUSEHWHEEL,
    WM_MOUSEWHEEL,
};

#[cfg(target_os = "windows")]
static ENGINE_TX: OnceLock<Sender<EngineCommand>> = OnceLock::new();

#[cfg(target_os = "windows")]
pub struct MouseHook {
    handle: HHOOK,
}

#[cfg(target_os = "windows")]
impl MouseHook {
    pub fn install(tx: Sender<EngineCommand>) -> Result<Self, String> {
        let _ = ENGINE_TX.set(tx);

        // SAFETY: null means current module.
        let h_instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        // SAFETY: callback is a valid function pointer and thread id 0 = global LL hook.
        let handle = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), h_instance, 0) };
        if handle.is_null() {
            return Err("SetWindowsHookExW(WH_MOUSE_LL) failed".to_string());
        }
        Ok(Self { handle })
    }
}

#[cfg(target_os = "windows")]
impl Drop for MouseHook {
    fn drop(&mut self) {
        // SAFETY: hook handle came from SetWindowsHookExW.
        unsafe {
            let _ = UnhookWindowsHookEx(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Bit 0 of MSLLHOOKSTRUCT.flags — set by the OS for any event
    // produced via SendInput / mouse_event.  This is the reliable way
    // to detect injected input; dwExtraInfo propagation is not
    // guaranteed across all Windows versions and configurations.
    const LLMHF_INJECTED: u32 = 0x01;

    if code == HC_ACTION as i32 {
        let msg = wparam as u32;
        if msg == WM_MOUSEWHEEL || msg == WM_MOUSEHWHEEL {
            // SAFETY: for HC_ACTION + wheel events, lparam points to MSLLHOOKSTRUCT.
            let info = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };

            // Pass through any injected event (ours or other programs').
            // Only intercept genuine hardware wheel input.
            if info.flags & LLMHF_INJECTED != 0 {
                log::debug!(
                    "[hook] pass-through injected event (flags=0x{:X})",
                    info.flags
                );
                // SAFETY: pass-through to next hook.
                return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
            }

            let delta = ((info.mouseData >> 16) as u16) as i16;
            let horizontal = msg == WM_MOUSEHWHEEL;

            // Resolve the PID of the window under the cursor.
            // If WindowFromPoint returns null, pid stays 0 (global fallback).
            let hwnd = unsafe { WindowFromPoint(info.pt) };
            let hwnd_root = unsafe { GetAncestor(hwnd, GA_ROOT) };
            let mut pid: u32 = 0;
            unsafe { GetWindowThreadProcessId(hwnd_root, &mut pid) };

            log::debug!(
                "[hook] wheel event: delta={delta}, horizontal={horizontal}, target_pid={pid}, mouseData=0x{:08X}",
                info.mouseData
            );
            if let Some(tx) = ENGINE_TX.get() {
                if tx
                    .send(EngineCommand::Scroll {
                        delta,
                        horizontal,
                        target_pid: pid,
                        target_hwnd: hwnd_root as isize,
                    })
                    .is_ok()
                {
                    // Swallow original event only when enqueue succeeds; smoothed
                    // events will be re-injected by the engine.
                    return 1;
                }
                log::warn!("[hook] WARNING: channel send failed, passing through");
            } else {
                log::warn!("[hook] WARNING: ENGINE_TX not set, passing through");
            }
        }
    }

    // SAFETY: pass-through to next hook.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

#[cfg(not(target_os = "windows"))]
pub struct MouseHook;

#[cfg(not(target_os = "windows"))]
impl MouseHook {
    pub fn install(_tx: Sender<EngineCommand>) -> Result<Self, String> {
        Ok(Self)
    }
}
