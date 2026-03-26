use crate::resolve::ProcessResolver;
use crate::threshold::AppKey;

#[cfg(target_os = "windows")]
pub struct WindowsProcessResolver;

#[cfg(target_os = "windows")]
impl WindowsProcessResolver {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "windows")]
impl ProcessResolver for WindowsProcessResolver {
    fn resolve_pid(&self, pid: u32) -> Option<AppKey> {
        use std::path::PathBuf;
        use std::time::UNIX_EPOCH;
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };

        // 1. Open process handle with minimal permissions.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            log::warn!("[resolve] OpenProcess failed for pid={pid}");
            return None;
        }

        // 2. Query the full executable path.
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = unsafe {
            QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len)
        };

        // MUST close handle regardless of query result.
        unsafe { CloseHandle(handle) };

        if ok == 0 || len == 0 {
            log::warn!("[resolve] QueryFullProcessImageNameW failed for pid={pid}");
            return None;
        }

        let path = String::from_utf16_lossy(&buf[..len as usize]);

        // 3. Get file mtime for cache-busting on binary updates.
        let exe_mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        Some(AppKey {
            exe_path: PathBuf::from(path),
            exe_mtime,
        })
    }
}

// Non-Windows stub — always returns None.
#[cfg(not(target_os = "windows"))]
pub struct WindowsProcessResolver;

#[cfg(not(target_os = "windows"))]
impl WindowsProcessResolver {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_os = "windows"))]
impl ProcessResolver for WindowsProcessResolver {
    fn resolve_pid(&self, _pid: u32) -> Option<AppKey> {
        None
    }
}
