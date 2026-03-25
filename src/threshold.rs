use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Detection state for a given application's scroll behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThresholdMode {
    /// Not yet examined — use safe default (1.0).
    Unknown,
    /// Detection in progress — use safe default (1.0) until resolved.
    Detecting,
    /// App handles smooth (hi-res) scroll correctly.
    SmoothOk,
    /// App expects legacy 120-unit wheel deltas.
    Legacy120,
}

impl ThresholdMode {
    /// The `inject_threshold` value implied by this mode.
    pub fn threshold(&self) -> f64 {
        match self {
            Self::Unknown | Self::Detecting | Self::SmoothOk => 1.0,
            Self::Legacy120 => 120.0,
        }
    }
}

/// Cache key identifying an application binary.
///
/// Keyed on the executable path (normalized) plus an optional mtime so
/// that re-detection triggers when the binary is updated.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppKey {
    pub exe_path: PathBuf,
    pub exe_mtime: Option<u64>,
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// Per-application threshold cache.
///
/// Pure data structure — no file I/O, no platform deps.
/// Persistence will be layered on top in a later task.
pub struct AppThresholdCache {
    modes: HashMap<AppKey, ThresholdMode>,
}

impl AppThresholdCache {
    pub fn new() -> Self {
        Self {
            modes: HashMap::new(),
        }
    }

    /// Resolve the effective `inject_threshold` for an application.
    ///
    /// Returns the mode-derived threshold, or the safe default (1.0) when
    /// the app has not been seen before.
    pub fn get_threshold(&self, app_key: Option<&AppKey>) -> f64 {
        match app_key.and_then(|k| self.modes.get(k)) {
            Some(mode) => mode.threshold(),
            None => ThresholdMode::Unknown.threshold(),
        }
    }

    /// Explicitly set the detection mode for an application.
    pub fn set_mode(&mut self, app_key: AppKey, mode: ThresholdMode) {
        self.modes.insert(app_key, mode);
    }

    /// Begin detection for an app — returns `true` if the mode was set to
    /// `Detecting`, or `false` if the app is already being detected or has
    /// a resolved mode (dedup guard).
    pub fn start_detecting(&mut self, app_key: AppKey) -> bool {
        match self.modes.get(&app_key) {
            None | Some(ThresholdMode::Unknown) => {
                self.modes.insert(app_key, ThresholdMode::Detecting);
                true
            }
            Some(_) => false,
        }
    }

    /// Look up the current mode for an application.
    pub fn get_mode(&self, app_key: &AppKey) -> Option<&ThresholdMode> {
        self.modes.get(app_key)
    }

    /// Look up a user-configured override for the given exe path.
    ///
    /// Stub — will be wired to `Config.output.app_overrides` in Task 6.
    pub fn lookup_override(&self, _exe_path: &str) -> Option<f64> {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(path: &str) -> AppKey {
        AppKey {
            exe_path: PathBuf::from(path),
            exe_mtime: None,
        }
    }

    #[test]
    fn unknown_app_returns_default() {
        let cache = AppThresholdCache::new();
        let key = make_key("C:\\Program Files\\App\\app.exe");
        assert!((cache.get_threshold(Some(&key)) - 1.0).abs() < f64::EPSILON);
        // Also test the None path
        assert!((cache.get_threshold(None) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn legacy_app_returns_120() {
        let mut cache = AppThresholdCache::new();
        let key = make_key("C:\\legacy\\old.exe");
        cache.set_mode(key.clone(), ThresholdMode::Legacy120);
        assert!((cache.get_threshold(Some(&key)) - 120.0).abs() < f64::EPSILON);
    }

    #[test]
    fn smooth_app_returns_1() {
        let mut cache = AppThresholdCache::new();
        let key = make_key("C:\\modern\\smooth.exe");
        cache.set_mode(key.clone(), ThresholdMode::SmoothOk);
        assert!((cache.get_threshold(Some(&key)) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn start_detecting_prevents_duplicate() {
        let mut cache = AppThresholdCache::new();
        let key = make_key("/usr/bin/firefox");

        // First call: Unknown → Detecting, returns true
        assert!(cache.start_detecting(key.clone()));
        assert_eq!(cache.get_mode(&key), Some(&ThresholdMode::Detecting));

        // Second call: already Detecting, returns false
        assert!(!cache.start_detecting(key.clone()));

        // Also blocked after resolution
        cache.set_mode(key.clone(), ThresholdMode::SmoothOk);
        assert!(!cache.start_detecting(key));
    }

    #[test]
    fn detecting_uses_default_threshold() {
        let mut cache = AppThresholdCache::new();
        let key = make_key("/opt/app/bin");
        cache.set_mode(key.clone(), ThresholdMode::Detecting);
        assert!((cache.get_threshold(Some(&key)) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lookup_override_returns_none_stub() {
        let cache = AppThresholdCache::new();
        assert_eq!(cache.lookup_override("anything.exe"), None);
    }

    #[test]
    fn threshold_mode_values() {
        assert!((ThresholdMode::Unknown.threshold() - 1.0).abs() < f64::EPSILON);
        assert!((ThresholdMode::Detecting.threshold() - 1.0).abs() < f64::EPSILON);
        assert!((ThresholdMode::SmoothOk.threshold() - 1.0).abs() < f64::EPSILON);
        assert!((ThresholdMode::Legacy120.threshold() - 120.0).abs() < f64::EPSILON);
    }
}
