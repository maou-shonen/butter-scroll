pub trait AutoStartService: Send + Sync {
    fn is_enabled(&self) -> bool;
    fn set_enabled(&self, enabled: bool) -> Result<(), String>;
}

#[cfg(target_os = "windows")]
pub struct WindowsAutoStart {
    app_name: String,
    exe_path: String,
}

#[cfg(target_os = "windows")]
impl WindowsAutoStart {
    pub fn new(app_name: impl Into<String>) -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|e| format!("current_exe error: {e}"))?;
        let exe_path = exe
            .to_str()
            .ok_or_else(|| "executable path is not valid UTF-8".to_string())?
            .to_string();
        Ok(Self {
            app_name: app_name.into(),
            exe_path,
        })
    }
}

#[cfg(target_os = "windows")]
impl AutoStartService for WindowsAutoStart {
    fn is_enabled(&self) -> bool {
        use crate::util::to_wide;
        use windows_sys::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ,
        };

        let key = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let value = to_wide(&self.app_name);

        let mut out_type: u32 = 0;
        let mut size: u32 = 0;
        // SAFETY: pointers are valid for call duration.
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_SZ,
                &mut out_type,
                std::ptr::null_mut(),
                &mut size,
            )
        };

        // No need for RegCloseKey when using predefined key handle.
        let _ = out_type;
        result == 0 && size > 0
    }

    fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        use crate::util::to_wide;
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
            REG_SZ,
        };

        let key_path = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let mut key: HKEY = std::ptr::null_mut();

        // SAFETY: pointers are valid and key output is initialized by API.
        let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, key_path.as_ptr(), &mut key) };

        if status != 0 {
            return Err(format!("RegCreateKeyW failed: {status}"));
        }

        let result = if enabled {
            let name = to_wide(&self.app_name);
            // Quote path to survive spaces (e.g. Program Files).
            let quoted_path = format!("\"{}\"", self.exe_path.replace('"', ""));
            let data = to_wide(&quoted_path);
            // UTF-16 bytes including null terminator.
            let bytes_len = (data.len() * 2) as u32;
            // SAFETY: registry key and buffers are valid.
            unsafe {
                RegSetValueExW(
                    key,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    data.as_ptr() as *const u8,
                    bytes_len,
                )
            }
        } else {
            let name = to_wide(&self.app_name);
            // SAFETY: registry key is valid.
            unsafe { RegDeleteValueW(key, name.as_ptr()) }
        };

        // SAFETY: handle created by RegCreateKeyW.
        unsafe {
            RegCloseKey(key);
        }

        if result != 0 {
            return Err(format!("registry update failed: {result}"));
        }
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub struct WindowsAutoStart;

#[cfg(not(target_os = "windows"))]
impl WindowsAutoStart {
    pub fn new(_app_name: impl Into<String>) -> Result<Self, String> {
        Ok(Self)
    }
}

#[cfg(not(target_os = "windows"))]
impl AutoStartService for WindowsAutoStart {
    fn is_enabled(&self) -> bool {
        false
    }

    fn set_enabled(&self, _enabled: bool) -> Result<(), String> {
        Ok(())
    }
}
