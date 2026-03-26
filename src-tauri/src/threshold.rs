use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Detection state for a given application's scroll behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppKey {
    pub exe_path: PathBuf,
    pub exe_mtime: Option<u64>,
}

// ---------------------------------------------------------------------------
// Persistence DTO
// ---------------------------------------------------------------------------

/// On-disk representation of a single cache entry.
#[derive(Serialize, Deserialize)]
struct CacheEntry {
    exe_path: PathBuf,
    exe_mtime: u64,
    mode: ThresholdMode,
}

/// Read the mtime of a file as seconds since UNIX epoch.
fn file_mtime_secs(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// Per-application threshold cache with optional JSON persistence.
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

    // -- Persistence --------------------------------------------------------

    /// Persist resolved entries to a JSON file (atomic write via temp+rename).
    ///
    /// Only `SmoothOk` and `Legacy120` entries are saved — transient states
    /// (`Unknown`, `Detecting`) are intentionally omitted.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let entries: Vec<CacheEntry> = self
            .modes
            .iter()
            .filter_map(|(key, mode)| match mode {
                ThresholdMode::SmoothOk | ThresholdMode::Legacy120 => {
                    key.exe_mtime.map(|mtime| CacheEntry {
                        exe_path: key.exe_path.clone(),
                        exe_mtime: mtime,
                        mode: mode.clone(),
                    })
                }
                _ => None,
            })
            .collect();
        let count = entries.len();

        let json =
            serde_json::to_string_pretty(&entries).map_err(|e| format!("serialize error: {e}"))?;

        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json).map_err(|e| format!("write error: {e}"))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("rename error: {e}"))?;
        eprintln!("[threshold] cache saved: {} entries", count);

        Ok(())
    }

    /// Load a cache from a JSON file, skipping stale or invalid entries.
    ///
    /// Returns an empty cache on any I/O or parse error (never panics).
    pub fn load(path: &Path) -> Self {
        let mut cache = Self::new();

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("[threshold] cache loaded: 0 entries");
                return cache;
            }
        };

        let entries: Vec<CacheEntry> = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => {
                eprintln!("[threshold] cache loaded: 0 entries");
                return cache;
            }
        };

        for entry in entries {
            // Skip entries whose exe no longer exists or whose mtime changed.
            match file_mtime_secs(&entry.exe_path) {
                Some(actual) if actual == entry.exe_mtime => {}
                _ => continue,
            }

            let key = AppKey {
                exe_path: entry.exe_path,
                exe_mtime: Some(entry.exe_mtime),
            };
            cache.modes.insert(key, entry.mode);
        }

        eprintln!("[threshold] cache loaded: {} entries", cache.modes.len());
        cache
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
    fn threshold_mode_values() {
        assert!((ThresholdMode::Unknown.threshold() - 1.0).abs() < f64::EPSILON);
        assert!((ThresholdMode::Detecting.threshold() - 1.0).abs() < f64::EPSILON);
        assert!((ThresholdMode::SmoothOk.threshold() - 1.0).abs() < f64::EPSILON);
        assert!((ThresholdMode::Legacy120.threshold() - 120.0).abs() < f64::EPSILON);
    }

    // -- Persistence tests --------------------------------------------------

    /// Helper: create an AppKey whose exe_path points to a real file with a
    /// matching mtime so that `load()` considers it fresh.
    fn make_real_key(dir: &Path, name: &str, mode: ThresholdMode) -> (AppKey, ThresholdMode) {
        let file = dir.join(name);
        std::fs::write(&file, b"fake-exe").unwrap();
        let mtime = file_mtime_secs(&file).unwrap();
        let key = AppKey {
            exe_path: file,
            exe_mtime: Some(mtime),
        };
        (key, mode)
    }

    #[test]
    fn cache_save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("cache.json");

        let mut cache = AppThresholdCache::new();
        let (k1, m1) = make_real_key(dir.path(), "smooth.exe", ThresholdMode::SmoothOk);
        let (k2, m2) = make_real_key(dir.path(), "legacy.exe", ThresholdMode::Legacy120);
        cache.set_mode(k1.clone(), m1);
        cache.set_mode(k2.clone(), m2);

        cache.save(&cache_path).unwrap();

        let loaded = AppThresholdCache::load(&cache_path);
        assert_eq!(loaded.get_mode(&k1), Some(&ThresholdMode::SmoothOk));
        assert_eq!(loaded.get_mode(&k2), Some(&ThresholdMode::Legacy120));
    }

    #[test]
    fn cache_skips_unknown_on_save() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("cache.json");

        let mut cache = AppThresholdCache::new();
        let (k1, _) = make_real_key(dir.path(), "smooth.exe", ThresholdMode::SmoothOk);
        cache.set_mode(k1, ThresholdMode::SmoothOk);

        // Unknown and Detecting entries should be excluded from the file.
        cache.set_mode(make_key("C:\\unknown.exe"), ThresholdMode::Unknown);
        cache.set_mode(make_key("C:\\detecting.exe"), ThresholdMode::Detecting);

        cache.save(&cache_path).unwrap();

        let content = std::fs::read_to_string(&cache_path).unwrap();
        assert!(!content.contains("unknown.exe"));
        assert!(!content.contains("detecting.exe"));
        assert!(!content.contains("Unknown"));
        assert!(!content.contains("Detecting"));
    }

    #[test]
    fn cache_handles_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("nonexistent.json");

        let loaded = AppThresholdCache::load(&cache_path);
        // Should return an empty cache, no panic.
        assert_eq!(loaded.get_mode(&make_key("anything")), None);
    }

    #[test]
    fn cache_skips_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("cache.json");

        // Manually write a cache file with a fake exe path that doesn't exist.
        let entries = serde_json::json!([
            {
                "exe_path": dir.path().join("gone.exe"),
                "exe_mtime": 999,
                "mode": "Legacy120"
            }
        ]);
        std::fs::write(&cache_path, entries.to_string()).unwrap();

        let loaded = AppThresholdCache::load(&cache_path);
        // The entry should be skipped because the exe doesn't exist.
        assert!(loaded.modes.is_empty());
    }
}
