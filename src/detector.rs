use crate::threshold::{AppKey, ThresholdMode};

pub trait ScrollDetector: Send {
    fn detect(&self, hwnd: isize, expected_delta: f64) -> ThresholdMode;
}

#[cfg(test)]
pub struct MockScrollDetector {
    pub result: ThresholdMode,
}

#[cfg(test)]
impl ScrollDetector for MockScrollDetector {
    fn detect(&self, _hwnd: isize, _expected_delta: f64) -> ThresholdMode {
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector_win::{detect_with_mock, MockDetectionInput};

    #[test]
    fn wpf_class_detected() {
        let input = MockDetectionInput {
            class_name: Some("HwndWrapper[app]".to_string()),
            style: Some(0),
            before_pos: Some(100),
            after_pos: Some(100),
        };

        let mode = detect_with_mock(&input, 123, 1.0);
        assert_eq!(mode, ThresholdMode::Legacy120);
    }

    #[test]
    fn modern_app_smooth_ok() {
        let input = MockDetectionInput {
            class_name: Some("Chrome_WidgetWin_1".to_string()),
            style: Some(0), // no WS_VSCROLL
            before_pos: None,
            after_pos: None,
        };

        let mode = detect_with_mock(&input, 456, 1.0);
        assert_eq!(mode, ThresholdMode::SmoothOk);
    }

    #[test]
    fn all_failure_paths_return_unknown() {
        // bad hwnd
        let ok_input = MockDetectionInput {
            class_name: Some("AnyWindow".to_string()),
            style: Some(0),
            before_pos: None,
            after_pos: None,
        };
        assert_eq!(detect_with_mock(&ok_input, 0, 1.0), ThresholdMode::Unknown);

        // class name fetch failed
        let class_fail = MockDetectionInput {
            class_name: None,
            style: Some(0),
            before_pos: None,
            after_pos: None,
        };
        assert_eq!(detect_with_mock(&class_fail, 1, 1.0), ThresholdMode::Unknown);

        // scrollbar path but GetScrollInfo failed
        let scroll_fail = MockDetectionInput {
            class_name: Some("Win32Window".to_string()),
            style: Some(crate::detector_win::WS_VSCROLL_STYLE),
            before_pos: None,
            after_pos: None,
        };
        assert_eq!(detect_with_mock(&scroll_fail, 1, 1.0), ThresholdMode::Unknown);
    }

}
