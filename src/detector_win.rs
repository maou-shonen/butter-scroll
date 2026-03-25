use crate::detector::ScrollDetector;
use crate::threshold::ThresholdMode;

const WPF_CLASS_PREFIX: &str = "HwndWrapper";
pub(crate) const WS_VSCROLL_STYLE: u32 = 0x0020_0000;

fn classify_detection(
    hwnd: isize,
    expected_delta: f64,
    class_name: Option<&str>,
    style: Option<u32>,
    before_pos: Option<i32>,
    after_pos: Option<i32>,
) -> ThresholdMode {
    if hwnd == 0 || expected_delta <= f64::EPSILON {
        return ThresholdMode::Unknown;
    }

    let class_name = match class_name {
        Some(name) => name,
        None => return ThresholdMode::Unknown,
    };

    if class_name.starts_with(WPF_CLASS_PREFIX) {
        return ThresholdMode::Legacy120;
    }

    let style = match style {
        Some(v) => v,
        None => return ThresholdMode::Unknown,
    };

    if style & WS_VSCROLL_STYLE == 0 {
        return ThresholdMode::SmoothOk;
    }

    let before_pos = match before_pos {
        Some(v) => v,
        None => return ThresholdMode::Unknown,
    };
    let after_pos = match after_pos {
        Some(v) => v,
        None => return ThresholdMode::Unknown,
    };

    let actual_delta = (after_pos - before_pos).abs() as f64;
    if actual_delta <= f64::EPSILON {
        // 邊界或沒有滾動，不足以判斷。
        return ThresholdMode::Unknown;
    }

    if actual_delta > 5.0 * expected_delta.abs() {
        ThresholdMode::Legacy120
    } else {
        ThresholdMode::SmoothOk
    }
}

#[cfg(test)]
pub(crate) struct MockDetectionInput {
    pub class_name: Option<String>,
    pub style: Option<u32>,
    pub before_pos: Option<i32>,
    pub after_pos: Option<i32>,
}

#[cfg(test)]
pub(crate) fn detect_with_mock(
    input: &MockDetectionInput,
    hwnd: isize,
    expected_delta: f64,
) -> ThresholdMode {
    classify_detection(
        hwnd,
        expected_delta,
        input.class_name.as_deref(),
        input.style,
        input.before_pos,
        input.after_pos,
    )
}

#[cfg(target_os = "windows")]
pub struct WindowsScrollDetector;

#[cfg(target_os = "windows")]
impl WindowsScrollDetector {
    pub fn new() -> Self {
        Self
    }

    fn get_class_name(hwnd: isize) -> Option<String> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetClassNameW;

        let mut buf = [0u16; 256];
        let len = unsafe { GetClassNameW(hwnd as HWND, buf.as_mut_ptr(), buf.len() as i32) };
        if len <= 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }

    fn get_style(hwnd: isize) -> Option<u32> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongW, GWL_STYLE};

        let style = unsafe { GetWindowLongW(hwnd as HWND, GWL_STYLE) };
        Some(style as u32)
    }

    fn get_scroll_pos(hwnd: isize) -> Option<i32> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::Controls::{GetScrollInfo, SCROLLINFO, SIF_POS};
        use windows_sys::Win32::UI::WindowsAndMessaging::SB_VERT;

        let mut info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_POS,
            nMin: 0,
            nMax: 0,
            nPage: 0,
            nPos: 0,
            nTrackPos: 0,
        };

        let ok = unsafe { GetScrollInfo(hwnd as HWND, SB_VERT, &mut info) };
        if ok == 0 {
            return None;
        }
        Some(info.nPos)
    }

    fn wait_for_scroll_change(hwnd: isize, before_pos: i32) -> Option<i32> {
        use std::time::{Duration, Instant};

        let deadline = Instant::now() + Duration::from_millis(250);
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
            let pos = Self::get_scroll_pos(hwnd)?;
            if pos != before_pos {
                return Some(pos);
            }
        }
        Some(before_pos)
    }
}

#[cfg(target_os = "windows")]
impl ScrollDetector for WindowsScrollDetector {
    fn detect(&self, hwnd: isize, expected_delta: f64) -> ThresholdMode {
        use windows_sys::Win32::UI::WindowsAndMessaging::WS_VSCROLL;

        if hwnd == 0 {
            return ThresholdMode::Unknown;
        }

        let class_name = match Self::get_class_name(hwnd) {
            Some(v) => v,
            None => return ThresholdMode::Unknown,
        };

        if class_name.starts_with(WPF_CLASS_PREFIX) {
            return ThresholdMode::Legacy120;
        }

        let style = match Self::get_style(hwnd) {
            Some(v) => v,
            None => return ThresholdMode::Unknown,
        };

        if style & WS_VSCROLL == 0 {
            return ThresholdMode::SmoothOk;
        }

        let before_pos = match Self::get_scroll_pos(hwnd) {
            Some(v) => v,
            None => return ThresholdMode::Unknown,
        };

        let after_pos = match Self::wait_for_scroll_change(hwnd, before_pos) {
            Some(v) => v,
            None => return ThresholdMode::Unknown,
        };

        classify_detection(
            hwnd,
            expected_delta,
            Some(&class_name),
            Some(style),
            Some(before_pos),
            Some(after_pos),
        )
    }
}

#[cfg(not(target_os = "windows"))]
pub struct WindowsScrollDetector;

#[cfg(not(target_os = "windows"))]
impl WindowsScrollDetector {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_os = "windows"))]
impl ScrollDetector for WindowsScrollDetector {
    fn detect(&self, _hwnd: isize, _expected_delta: f64) -> ThresholdMode {
        ThresholdMode::SmoothOk
    }
}
