const HOTKEY_ID: i32 = 1;

const MOD_ALT_VALUE: u32 = 0x0001;
const MOD_CONTROL_VALUE: u32 = 0x0002;
const MOD_SHIFT_VALUE: u32 = 0x0004;
const MOD_NOREPEAT_VALUE: u32 = 0x4000;

const VK_B_VALUE: u32 = 0x42;
const VK_H_VALUE: u32 = 0x48;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedCombo {
    modifiers: u32,
    vk: u32,
}

fn parse_combo(combo: &str) -> Result<ParsedCombo, String> {
    let normalized = combo.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("Hotkey combo cannot be empty".to_string());
    }

    let mut modifiers = MOD_NOREPEAT_VALUE;
    let mut key: Option<u32> = None;

    for token in normalized
        .split('+')
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        match token {
            "ctrl" => modifiers |= MOD_CONTROL_VALUE,
            "shift" => modifiers |= MOD_SHIFT_VALUE,
            "alt" => modifiers |= MOD_ALT_VALUE,
            "b" => {
                if key.replace(VK_B_VALUE).is_some() {
                    return Err(format!("Invalid hotkey combo '{combo}': multiple keys"));
                }
            }
            "h" => {
                if key.replace(VK_H_VALUE).is_some() {
                    return Err(format!("Invalid hotkey combo '{combo}': multiple keys"));
                }
            }
            _ => return Err(format!("Unsupported hotkey token '{token}' in '{combo}'")),
        }
    }

    let key = key.ok_or_else(|| format!("Invalid hotkey combo '{combo}': missing key"))?;
    if modifiers & (MOD_CONTROL_VALUE | MOD_SHIFT_VALUE | MOD_ALT_VALUE) == 0 {
        return Err(format!(
            "Invalid hotkey combo '{combo}': at least one modifier is required"
        ));
    }

    Ok(ParsedCombo { modifiers, vk: key })
}

fn try_invoke_with_guard(guard: &std::sync::atomic::AtomicBool, callback: impl FnOnce()) -> bool {
    use std::sync::atomic::Ordering;

    if guard
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }

    callback();
    guard.store(false, Ordering::Release);
    true
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::sync::{atomic::AtomicBool, Arc, Mutex, OnceLock};

    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, WM_HOTKEY, WNDCLASSW,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    const WINDOW_CLASS_NAME: &str = "ButterScrollHotkeyWindow";
    const ERROR_HOTKEY_ALREADY_REGISTERED: u32 = 1409;
    const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;

    struct CallbackState {
        callback: Box<dyn Fn() + Send + 'static>,
        guard: AtomicBool,
    }

    static CALLBACK_STATE: OnceLock<Mutex<Option<Arc<CallbackState>>>> = OnceLock::new();

    fn callback_state() -> &'static Mutex<Option<Arc<CallbackState>>> {
        CALLBACK_STATE.get_or_init(|| Mutex::new(None))
    }

    pub struct PlatformHotkeyManager {
        hwnd: HWND,
        combo: ParsedCombo,
    }

    impl PlatformHotkeyManager {
        pub fn new(combo: &str, callback: impl Fn() + Send + 'static) -> Result<Self, String> {
            let parsed = parse_combo(combo)?;
            let hwnd = create_hidden_window()?;

            let state = Arc::new(CallbackState {
                callback: Box::new(callback),
                guard: AtomicBool::new(false),
            });
            *callback_state().lock().unwrap() = Some(state);

            if !register_hotkey(hwnd, parsed) {
                *callback_state().lock().unwrap() = None;
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                return Err(hotkey_error("register hotkey", combo));
            }

            Ok(Self {
                hwnd,
                combo: parsed,
            })
        }

        pub fn update_combo(&mut self, new_combo: &str) -> Result<(), String> {
            let new_parsed = parse_combo(new_combo)?;

            unsafe {
                let _ = UnregisterHotKey(self.hwnd, HOTKEY_ID);
            }

            if register_hotkey(self.hwnd, new_parsed) {
                self.combo = new_parsed;
                return Ok(());
            }

            if !register_hotkey(self.hwnd, self.combo) {
                return Err(format!(
                    "Failed to register new hotkey '{new_combo}' and rollback old hotkey"
                ));
            }

            Err(hotkey_error("update hotkey", new_combo))
        }
    }

    impl Drop for PlatformHotkeyManager {
        fn drop(&mut self) {
            *callback_state().lock().unwrap() = None;
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }

    unsafe extern "system" fn hotkey_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_HOTKEY && wparam as i32 == HOTKEY_ID {
            let state = callback_state().lock().unwrap().clone();
            if let Some(state) = state {
                let _ = try_invoke_with_guard(&state.guard, || {
                    (state.callback)();
                });
            }
            return 0;
        }

        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    fn create_hidden_window() -> Result<HWND, String> {
        // SAFETY: null means current module.
        let h_instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        let class_name = crate::util::to_wide(WINDOW_CLASS_NAME);

        let wnd_class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(hotkey_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        // SAFETY: valid class struct and UTF-16 class name.
        let atom = unsafe { RegisterClassW(&wnd_class) };
        if atom == 0 {
            let err = unsafe { GetLastError() };
            if err != ERROR_CLASS_ALREADY_EXISTS {
                return Err(format!("RegisterClassW failed with error code {err}"));
            }
        }

        // SAFETY: creating a hidden tool window with a registered class.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                h_instance,
                std::ptr::null(),
            )
        };

        if hwnd.is_null() {
            let err = unsafe { GetLastError() };
            return Err(format!("CreateWindowExW failed with error code {err}"));
        }

        Ok(hwnd)
    }

    fn register_hotkey(hwnd: HWND, combo: ParsedCombo) -> bool {
        unsafe { RegisterHotKey(hwnd, HOTKEY_ID, combo.modifiers, combo.vk) != 0 }
    }

    fn hotkey_error(action: &str, combo: &str) -> String {
        let err = unsafe { GetLastError() };
        if err == ERROR_HOTKEY_ALREADY_REGISTERED {
            return format!(
                "Cannot {action}: hotkey '{combo}' is already registered by another application"
            );
        }
        format!("Cannot {action} '{combo}': Win32 error code {err}")
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub struct PlatformHotkeyManager;

    impl PlatformHotkeyManager {
        pub fn new(_combo: &str, _callback: impl Fn() + Send + 'static) -> Result<Self, String> {
            Ok(Self)
        }

        pub fn update_combo(&mut self, _new_combo: &str) -> Result<(), String> {
            Ok(())
        }
    }
}

pub use platform::PlatformHotkeyManager as HotkeyManager;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn parse_combo_ctrl_shift_b() {
        let parsed = parse_combo("ctrl+shift+b").unwrap();
        assert_eq!(parsed.vk, VK_B_VALUE);
        assert_eq!(
            parsed.modifiers,
            MOD_CONTROL_VALUE | MOD_SHIFT_VALUE | MOD_NOREPEAT_VALUE
        );
    }

    #[test]
    fn parse_combo_ctrl_alt_b() {
        let parsed = parse_combo("ctrl+alt+b").unwrap();
        assert_eq!(parsed.vk, VK_B_VALUE);
        assert_eq!(
            parsed.modifiers,
            MOD_CONTROL_VALUE | MOD_ALT_VALUE | MOD_NOREPEAT_VALUE
        );
    }

    #[test]
    fn parse_combo_invalid_returns_error() {
        assert!(parse_combo("invalid").is_err());
    }

    #[test]
    fn atomic_guard_prevents_reentrant_trigger() {
        let guard = AtomicBool::new(false);
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_count_inner = Arc::clone(&callback_count);

        let invoked = try_invoke_with_guard(&guard, || {
            callback_count_inner.fetch_add(1, Ordering::SeqCst);
            let reentrant = try_invoke_with_guard(&guard, || {
                callback_count_inner.fetch_add(1, Ordering::SeqCst);
            });
            assert!(!reentrant, "re-entrant trigger must be skipped");
        });

        assert!(invoked);
        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
    }
}
