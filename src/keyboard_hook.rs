use crate::config::KeyboardConfig;
use crate::traits::EngineCommand;
use crossbeam_channel::Sender;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const WHEEL_DELTA: i16 = 120;

/// Arrow key scroll: 1 wheel notch (goes through engine step_size scaling).
const NOTCHES_LINE: i16 = 1;

/// Assumed viewport height in lines for page scroll calculation.
/// Combined with the system's "lines per wheel notch" setting
/// (`SPI_GETWHEELSCROLLLINES`) to compute the wheel delta equivalent
/// of one page.  This is a best-effort approximation — native Page Down
/// scrolls the actual viewport height, which we cannot know from a
/// system-wide hook.
const PAGE_LINES: f64 = 28.0;

// ---------------------------------------------------------------------------
// Key classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum KeyGroup {
    PageUpDown,
    ArrowKeys,
    Space,
}

/// What kind of engine command a key produces.
enum KeyIntent {
    /// Line-level scroll (arrows) — goes through engine's step_size scaling.
    LineScroll { delta: i16, group: KeyGroup },
    /// Page-level scroll (PageUp/Down, Space) — bypasses step_size scaling.
    /// Uses pre-calculated delta based on viewport estimate + system settings.
    PageScroll { direction: f64, group: KeyGroup },
}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use crate::config::KeyboardMode;
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock, RwLock};

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_DOWN, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RWIN,
        VK_SHIFT, VK_SPACE, VK_UP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetForegroundWindow, GetGUIThreadInfo, GetParent, GetWindowLongW,
        GetWindowThreadProcessId, SetWindowsHookExW, UnhookWindowsHookEx, GUITHREADINFO, GWL_STYLE,
        HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
        WM_SYSKEYUP, WS_VSCROLL,
    };

    /// Shared state accessible from the hook callback.
    struct HookState {
        engine_tx: Sender<EngineCommand>,
        config: RwLock<KeyboardConfig>,
        /// VK codes whose keydown was swallowed — their keyup must also be
        /// swallowed to avoid unpaired events reaching the target app.
        intercepted_vkeys: Mutex<HashSet<u32>>,
    }

    static HOOK_STATE: OnceLock<HookState> = OnceLock::new();

    pub struct KeyboardHook {
        handle: HHOOK,
    }

    impl KeyboardHook {
        pub fn install(tx: Sender<EngineCommand>, config: KeyboardConfig) -> Result<Self, String> {
            let _ = HOOK_STATE.set(HookState {
                engine_tx: tx,
                config: RwLock::new(config),
                intercepted_vkeys: Mutex::new(HashSet::new()),
            });

            // SAFETY: null = current module; thread id 0 = global LL hook.
            let h_instance = unsafe { GetModuleHandleW(std::ptr::null()) };
            let handle =
                unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), h_instance, 0) };
            if handle.is_null() {
                return Err("SetWindowsHookExW(WH_KEYBOARD_LL) failed".to_string());
            }
            Ok(Self { handle })
        }

        /// Hot-update the keyboard config from the main thread.
        ///
        /// Does NOT clear `intercepted_vkeys`: if a keydown was already
        /// swallowed, the matching keyup must still be swallowed regardless
        /// of mode changes, otherwise the target app receives an unpaired
        /// keyup event.
        pub fn update_config(config: &KeyboardConfig) {
            if let Some(state) = HOOK_STATE.get() {
                *state.config.write().unwrap() = config.clone();
            }
        }
    }

    impl Drop for KeyboardHook {
        fn drop(&mut self) {
            // SAFETY: handle from SetWindowsHookExW.
            unsafe {
                let _ = UnhookWindowsHookEx(self.handle);
            }
        }
    }

    // -- Hook callback ------------------------------------------------------

    /// Bit flag: event was injected by SendInput / keybd_event.
    const LLKHF_INJECTED: u32 = 0x10;

    extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            // SAFETY: for HC_ACTION, lparam points to KBDLLHOOKSTRUCT.
            let info = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };

            // Never intercept injected events (ours or other programs').
            if info.flags & LLKHF_INJECTED != 0 {
                return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
            }

            if let Some(state) = HOOK_STATE.get() {
                let msg = wparam as u32;

                // -- KeyUp: swallow if we swallowed the matching keydown ----
                if msg == WM_KEYUP || msg == WM_SYSKEYUP {
                    if state.intercepted_vkeys.lock().unwrap().remove(&info.vkCode) {
                        return 1; // swallow paired keyup
                    }
                    return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
                }

                // -- KeyDown: evaluate whether to intercept ------------------
                if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                    let config = state.config.read().unwrap();

                    if !config.enabled {
                        return unsafe {
                            CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
                        };
                    }

                    // Ctrl / Alt / Win held → always pass through.
                    // Shift is allowed (used for Shift+Space reverse scroll).
                    if has_ctrl_alt_win() {
                        return unsafe {
                            CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
                        };
                    }

                    let shift_held = unsafe { GetAsyncKeyState(VK_SHIFT as i32) } < 0;

                    if let Some(intent) = classify_key(info.vkCode, shift_held) {
                        let group = match &intent {
                            KeyIntent::LineScroll { group, .. }
                            | KeyIntent::PageScroll { group, .. } => *group,
                        };

                        let mode = match group {
                            KeyGroup::PageUpDown => config.effective_mode(&config.page_up_down),
                            KeyGroup::ArrowKeys => config.effective_mode(&config.arrow_keys),
                            KeyGroup::Space => config.effective_mode(&config.space),
                        };

                        // For Page Up/Down and Arrow keys, Shift typically
                        // means selection — pass through.
                        if shift_held && group != KeyGroup::Space {
                            return unsafe {
                                CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
                            };
                        }

                        let should_intercept = match mode {
                            KeyboardMode::Off => false,
                            KeyboardMode::Always => true,
                            KeyboardMode::Win32Scrollbar => has_win32_scrollbar(),
                        };

                        if should_intercept {
                            let cmd = match intent {
                                KeyIntent::LineScroll { delta, .. } => EngineCommand::Scroll {
                                    delta,
                                    horizontal: false,
                                    target_pid: 0,
                                    target_hwnd: 0,
                                },
                                KeyIntent::PageScroll { direction, .. } => {
                                    EngineCommand::ScrollRaw {
                                        delta_y: direction * page_scroll_delta(),
                                    }
                                }
                            };

                            // Only swallow the key when the engine actually
                            // accepted the command.  If the channel is dead,
                            // let the original key through so user input is
                            // never silently eaten (mirrors mouse hook).
                            if state.engine_tx.send(cmd).is_ok() {
                                state.intercepted_vkeys.lock().unwrap().insert(info.vkCode);
                                return 1; // swallow keydown
                            }
                            eprintln!(
                                "[keyboard_hook] WARNING: channel send failed, passing through"
                            );
                        }
                    }
                }
            }
        }

        // SAFETY: pass-through to next hook.
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
    }

    // -- Helpers ------------------------------------------------------------

    /// Classify a virtual key code into a scroll intent.
    ///
    /// Page-level keys produce `PageScroll` (bypasses step_size scaling).
    /// Line-level keys produce `LineScroll` (uses engine's step_size).
    fn classify_key(vk: u32, shift_held: bool) -> Option<KeyIntent> {
        let vk16 = vk as u16;
        match vk16 {
            VK_PRIOR => Some(KeyIntent::PageScroll {
                direction: 1.0, // scroll up
                group: KeyGroup::PageUpDown,
            }),
            VK_NEXT => Some(KeyIntent::PageScroll {
                direction: -1.0, // scroll down
                group: KeyGroup::PageUpDown,
            }),
            VK_UP => Some(KeyIntent::LineScroll {
                delta: NOTCHES_LINE * WHEEL_DELTA,
                group: KeyGroup::ArrowKeys,
            }),
            VK_DOWN => Some(KeyIntent::LineScroll {
                delta: -(NOTCHES_LINE * WHEEL_DELTA),
                group: KeyGroup::ArrowKeys,
            }),
            VK_SPACE => Some(KeyIntent::PageScroll {
                direction: if shift_held { 1.0 } else { -1.0 },
                group: KeyGroup::Space,
            }),
            _ => None,
        }
    }

    /// Calculate the wheel delta equivalent of one page scroll.
    ///
    /// Reads `SPI_GETWHEELSCROLLLINES` (system "lines per wheel notch"
    /// setting, typically 3) and converts an assumed viewport height
    /// (`PAGE_LINES`) into a wheel delta:
    ///
    ///   `delta = (PAGE_LINES / lines_per_notch) × WHEEL_DELTA`
    ///
    /// This is a best-effort approximation — native Page Down scrolls
    /// the actual viewport height, which varies per window.
    fn page_scroll_delta() -> f64 {
        let lines_per_notch = get_wheel_scroll_lines();
        (PAGE_LINES / lines_per_notch as f64) * WHEEL_DELTA as f64
    }

    /// Read the Windows "lines per mouse wheel notch" system setting.
    fn get_wheel_scroll_lines() -> u32 {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPI_GETWHEELSCROLLLINES,
        };

        let mut lines: u32 = 3;
        // SAFETY: writing to a stack u32 through valid pointer.
        unsafe {
            SystemParametersInfoW(
                SPI_GETWHEELSCROLLLINES,
                0,
                &mut lines as *mut u32 as *mut _,
                0,
            );
        }
        // WHEEL_PAGESCROLL (0xFFFFFFFF) means "one page per notch" —
        // in that case one notch already is a full page.
        if lines == 0 || lines == u32::MAX {
            return 1;
        }
        lines
    }

    /// Check whether Ctrl, Alt, or Win is currently held.
    fn has_ctrl_alt_win() -> bool {
        unsafe {
            GetAsyncKeyState(VK_CONTROL as i32) < 0
                || GetAsyncKeyState(VK_MENU as i32) < 0
                || GetAsyncKeyState(VK_LWIN as i32) < 0
                || GetAsyncKeyState(VK_RWIN as i32) < 0
        }
    }

    /// Detect whether the focused control (or any ancestor) has a standard
    /// Win32 vertical scrollbar (`WS_VSCROLL`).
    ///
    /// Uses `GetGUIThreadInfo` to find the actual focused HWND within the
    /// foreground window, then walks up the parent chain.
    fn has_win32_scrollbar() -> bool {
        unsafe {
            let hwnd_fg = GetForegroundWindow();
            if hwnd_fg.is_null() {
                return false;
            }

            let thread_id = GetWindowThreadProcessId(hwnd_fg, std::ptr::null_mut());

            let mut gui_info: GUITHREADINFO = std::mem::zeroed();
            gui_info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;

            // Start from the focused control if available, else top-level.
            let start = if GetGUIThreadInfo(thread_id, &mut gui_info) != 0
                && !gui_info.hwndFocus.is_null()
            {
                gui_info.hwndFocus
            } else {
                hwnd_fg
            };

            check_scrollbar_chain(start)
        }
    }

    /// Walk the parent chain of `hwnd` checking for `WS_VSCROLL`.
    unsafe fn check_scrollbar_chain(mut hwnd: HWND) -> bool {
        while !hwnd.is_null() {
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            if style & WS_VSCROLL != 0 {
                return true;
            }
            hwnd = GetParent(hwnd);
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Non-Windows stub
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;

    pub struct KeyboardHook;

    impl KeyboardHook {
        pub fn install(
            _tx: Sender<EngineCommand>,
            _config: KeyboardConfig,
        ) -> Result<Self, String> {
            Ok(Self)
        }

        pub fn update_config(_config: &KeyboardConfig) {}
    }
}

// On non-Windows, no external code references KeyboardHook (all callers
// are behind #[cfg(target_os = "windows")]).
#[allow(unused_imports)]
pub use platform::KeyboardHook;
