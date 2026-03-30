use std::path::Path;

/// Foreground application identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundApp {
    pub exe_path: String,
    pub app_name: String,
}

impl ForegroundApp {
    pub fn from_exe_path(exe_path: String) -> Option<Self> {
        let app_name = app_name_from_exe_path(&exe_path);

        Some(Self { exe_path, app_name })
    }
}

fn app_name_from_exe_path(exe_path: &str) -> String {
    let file_name = exe_path.rsplit(['\\', '/']).next().unwrap_or(exe_path);
    Path::new(file_name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

/// Captures the current foreground application.
pub trait ForegroundCapture: Send + Sync {
    fn capture(&self) -> Option<ForegroundApp>;
}

fn is_self_exe(exe_path: &str) -> bool {
    std::env::current_exe()
        .ok()
        .map(|current| current.to_string_lossy().eq_ignore_ascii_case(exe_path))
        .unwrap_or(false)
}

pub fn capture_filtered<C: ForegroundCapture>(capture: &C) -> Option<ForegroundApp> {
    capture.capture().and_then(|app| {
        if is_self_exe(&app.exe_path) {
            None
        } else {
            Some(app)
        }
    })
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    pub struct WindowsForegroundCapture;

    impl WindowsForegroundCapture {
        pub fn new() -> Self {
            Self
        }
    }

    impl ForegroundCapture for WindowsForegroundCapture {
        fn capture(&self) -> Option<ForegroundApp> {
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            };
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetForegroundWindow, GetWindowThreadProcessId,
            };

            // 1. Get foreground HWND.
            let hwnd = unsafe { GetForegroundWindow() };
            if hwnd.is_null() {
                return None;
            }

            // 2. Resolve PID from the foreground window.
            let mut pid = 0u32;
            unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
            if pid == 0 {
                return None;
            }

            // 3. Open the process.
            let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
            if handle.is_null() {
                log::warn!("[foreground] OpenProcess failed for pid={pid}");
                return None;
            }

            // 4. Query the full executable path.
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = unsafe {
                QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len)
            };

            // 5. Close the handle regardless of success.
            unsafe { CloseHandle(handle) };

            if ok == 0 || len == 0 {
                log::warn!("[foreground] QueryFullProcessImageNameW failed for pid={pid}");
                return None;
            }

            let exe_path = String::from_utf16_lossy(&buf[..len as usize]);
            let app = ForegroundApp::from_exe_path(exe_path)?;

            // 6. Self-detection guard.
            if is_self_exe(&app.exe_path) {
                return None;
            }

            Some(app)
        }
    }

    pub use WindowsForegroundCapture as PlatformForegroundCapture;
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;

    pub struct WindowsForegroundCapture;

    impl Default for WindowsForegroundCapture {
        fn default() -> Self {
            Self
        }
    }

    impl WindowsForegroundCapture {
        pub fn new() -> Self {
            Self
        }
    }

    impl ForegroundCapture for WindowsForegroundCapture {
        fn capture(&self) -> Option<ForegroundApp> {
            None
        }
    }

    pub use WindowsForegroundCapture as PlatformForegroundCapture;
}

pub use platform::PlatformForegroundCapture as WindowsForegroundCapture;

#[cfg(test)]
pub struct MockForegroundCapture {
    pub result: Option<ForegroundApp>,
}

#[cfg(test)]
impl ForegroundCapture for MockForegroundCapture {
    fn capture(&self) -> Option<ForegroundApp> {
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_capture_returns_some() {
        let mock = MockForegroundCapture {
            result: Some(ForegroundApp {
                exe_path: r"C:\Windows\System32\notepad.exe".to_string(),
                app_name: "notepad".to_string(),
            }),
        };

        let app = capture_filtered(&mock).unwrap();
        assert_eq!(app.exe_path, r"C:\Windows\System32\notepad.exe");
        assert_eq!(app.app_name, "notepad");
    }

    #[test]
    fn app_name_is_extracted_from_exe_path() {
        let app =
            ForegroundApp::from_exe_path(r"C:\Windows\System32\notepad.exe".to_string()).unwrap();

        assert_eq!(app.app_name, "notepad");
    }

    #[test]
    fn self_detection_returns_none_for_current_exe() {
        let current_exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let mock = MockForegroundCapture {
            result: Some(ForegroundApp::from_exe_path(current_exe).unwrap()),
        };

        assert!(capture_filtered(&mock).is_none());
    }
}
