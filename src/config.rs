use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Configuration types (matching gblazex/smoothscroll defaults)
// ---------------------------------------------------------------------------

/// Top-level configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub scroll: ScrollConfig,
    pub acceleration: AccelerationConfig,
    pub output: OutputConfig,
    pub general: GeneralConfig,
    pub keyboard: KeyboardConfig,
}

/// Scroll animation parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrollConfig {
    /// Animation frame rate in Hz (default: 150)
    pub frame_rate: u32,
    /// Duration of one scroll animation in ms (default: 400)
    pub animation_time: u32,
    /// Scroll amount per wheel notch (default: 100.0).
    pub step_size: f64,
    /// Enable pulse easing algorithm (default: true)
    pub pulse_algorithm: bool,
    /// Pulse intensity scaling (default: 4)
    pub pulse_scale: f64,
    /// Pulse normalization hint (default: 1)
    pub pulse_normalize: f64,
    /// Invert scroll direction (default: false)
    pub inverted: bool,
}

/// Acceleration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccelerationConfig {
    /// Time window (ms) for detecting continuous scroll (default: 50)
    pub delta_ms: u32,
    /// Maximum acceleration multiplier (default: 3, set to 1 to disable)
    pub max: f64,
}

/// Output/injection parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Minimum accumulated delta before injecting a wheel event.
    /// Lower = smoother, higher = more compatible with legacy apps.
    /// 120 = WHEEL_DELTA (most compatible, chunkiest)
    /// 40 = WHEEL_DELTA/3 (good balance)
    /// 1 = per-frame (default; smoothest, modern apps only)
    pub inject_threshold: f64,
    /// Automatically detect per-app scroll behavior on first scroll.
    /// When enabled, apps that over-scroll with small deltas (e.g. WPF)
    /// are auto-switched to threshold=120.
    /// When disabled, only inject_threshold and app_overrides are used.
    #[serde(default = "default_true")]
    pub auto_detect: bool,
    /// Per-app threshold overrides keyed by executable path.
    #[serde(default)]
    pub app_overrides: HashMap<String, f64>,
}

fn default_true() -> bool {
    true
}

/// General application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Start with Windows (default: false)
    pub autostart: bool,
    /// Enable smooth scrolling globally (default: true)
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Keyboard smooth scrolling
// ---------------------------------------------------------------------------

/// Per-key-group activation mode.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardMode {
    /// Never intercept this key group.
    Off,
    /// Always intercept and convert to smooth scroll.
    Always,
    /// Only intercept when the focused window has a standard Win32 scrollbar.
    Win32Scrollbar,
}

/// Configuration for a single key group (e.g. Page Up/Down).
///
/// When `mode` is `None`, the group inherits the parent `[keyboard].mode`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyGroupConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<KeyboardMode>,
}

/// Keyboard smooth scrolling configuration.
///
/// `mode` acts as the default for all key groups; each group can override
/// it with its own `mode` value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardConfig {
    /// Master switch — keyboard smooth scrolling is opt-in.
    pub enabled: bool,
    /// Default mode applied to key groups that don't specify their own.
    pub mode: KeyboardMode,
    /// Page Up / Page Down — low conflict risk.
    pub page_up_down: KeyGroupConfig,
    /// Arrow Up / Arrow Down — high conflict risk (cursor movement, etc.).
    pub arrow_keys: KeyGroupConfig,
    /// Space / Shift+Space — medium risk (character input in editors).
    pub space: KeyGroupConfig,
}

impl KeyboardConfig {
    /// Resolve the effective mode for a key group, falling back to the
    /// parent default when the group doesn't specify its own.
    pub fn effective_mode(&self, group: &KeyGroupConfig) -> KeyboardMode {
        group.mode.unwrap_or(self.mode)
    }
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: KeyboardMode::Always,
            // Page Up/Down inherits parent mode (low risk).
            page_up_down: KeyGroupConfig { mode: None },
            // Arrow keys default to off (high risk).
            arrow_keys: KeyGroupConfig {
                mode: Some(KeyboardMode::Off),
            },
            // Space defaults to off (medium risk).
            space: KeyGroupConfig {
                mode: Some(KeyboardMode::Off),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Config methods
// ---------------------------------------------------------------------------

impl Config {
    /// Normalize and clamp user-provided configuration to safe ranges.
    ///
    /// This avoids pathological values from hand-edited TOML, such as
    /// `pulse_scale <= 0` or non-finite floats.
    pub fn sanitize(&mut self) {
        // Keep frame pacing reasonable.
        self.scroll.frame_rate = self.scroll.frame_rate.clamp(30, 1000);

        // Avoid zero/negative animation time.
        self.scroll.animation_time = self.scroll.animation_time.clamp(1, 5_000);

        // Step size should stay finite and positive.
        if !self.scroll.step_size.is_finite() || self.scroll.step_size <= 0.0 {
            self.scroll.step_size = ScrollConfig::default().step_size;
        }
        self.scroll.step_size = self.scroll.step_size.clamp(1.0, 2000.0);

        // Pulse scale must be finite and > 0.
        if !self.scroll.pulse_scale.is_finite() || self.scroll.pulse_scale <= 0.0 {
            self.scroll.pulse_scale = ScrollConfig::default().pulse_scale;
        }
        self.scroll.pulse_scale = self.scroll.pulse_scale.clamp(0.1, 20.0);

        if !self.scroll.pulse_normalize.is_finite() || self.scroll.pulse_normalize <= 0.0 {
            self.scroll.pulse_normalize = ScrollConfig::default().pulse_normalize;
        }
        self.scroll.pulse_normalize = self.scroll.pulse_normalize.clamp(0.1, 10.0);

        // Acceleration parameters.
        self.acceleration.delta_ms = self.acceleration.delta_ms.clamp(1, 500);
        if !self.acceleration.max.is_finite() || self.acceleration.max < 1.0 {
            self.acceleration.max = 1.0;
        }
        self.acceleration.max = self.acceleration.max.clamp(1.0, 20.0);

        if !self.output.inject_threshold.is_finite() || self.output.inject_threshold <= 0.0 {
            self.output.inject_threshold = OutputConfig::default().inject_threshold;
        }
        self.output.inject_threshold = self.output.inject_threshold.clamp(1.0, 120.0);

        for value in self.output.app_overrides.values_mut() {
            if !value.is_finite() {
                *value = 1.0;
            }
            *value = value.clamp(1.0, 120.0);
        }

        // Keyboard config: nothing numeric to clamp, but ensure mode
        // inheritance is consistent — a group set to `None` is valid
        // (inherits parent), so no fixup needed.
    }
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            frame_rate: 150,
            animation_time: 400,
            step_size: 100.0,
            pulse_algorithm: true,
            pulse_scale: 4.0,
            pulse_normalize: 1.0,
            inverted: false,
        }
    }
}

impl Default for AccelerationConfig {
    fn default() -> Self {
        Self {
            delta_ms: 50,
            max: 3.0,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            inject_threshold: 1.0,
            auto_detect: true,
            app_overrides: HashMap::new(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            autostart: false,
            enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Config I/O — trait for DI
// ---------------------------------------------------------------------------

/// Abstraction over config persistence.
pub trait ConfigStore: Send + Sync {
    fn load(&self) -> Config;
    fn save(&self, config: &Config) -> Result<(), String>;
    fn path(&self) -> &Path;
}

/// File-based config store (production implementation).
pub struct FileConfigStore {
    path: PathBuf,
}

impl FileConfigStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Default config path: beside the executable.
    pub fn default_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("config.toml")))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }
}

impl ConfigStore for FileConfigStore {
    fn load(&self) -> Config {
        if self.path.exists() {
            match fs::read_to_string(&self.path) {
                Ok(content) => match toml::from_str::<Config>(&content) {
                    Ok(mut cfg) => {
                        cfg.sanitize();
                        return cfg;
                    }
                    Err(e) => eprintln!("[butter-scroll] config parse error: {e}"),
                },
                Err(e) => eprintln!("[butter-scroll] config read error: {e}"),
            }
        }
        // First run — create default config file.
        let mut cfg = Config::default();
        cfg.sanitize();
        if let Err(e) = self.save(&cfg) {
            eprintln!("[butter-scroll] failed to write default config: {e}");
        }
        cfg
    }

    fn save(&self, config: &Config) -> Result<(), String> {
        let text = toml::to_string_pretty(config).map_err(|e| format!("serialize error: {e}"))?;
        fs::write(&self.path, text).map_err(|e| format!("write error: {e}"))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.scroll.frame_rate, 150);
        assert_eq!(cfg.scroll.animation_time, 400);
        assert!((cfg.scroll.step_size - 100.0).abs() < f64::EPSILON);
        assert!(cfg.scroll.pulse_algorithm);
        assert!((cfg.scroll.pulse_scale - 4.0).abs() < f64::EPSILON);
        assert!((cfg.scroll.pulse_normalize - 1.0).abs() < f64::EPSILON);
        assert!(!cfg.scroll.inverted);
        assert_eq!(cfg.acceleration.delta_ms, 50);
        assert!((cfg.acceleration.max - 3.0).abs() < f64::EPSILON);
        assert!((cfg.output.inject_threshold - 1.0).abs() < f64::EPSILON);
        assert!(cfg.general.enabled);
        assert!(!cfg.general.autostart);
    }

    #[test]
    fn round_trip_toml() {
        let cfg = Config::default();
        let text = toml::to_string_pretty(&cfg).unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(parsed.scroll.frame_rate, cfg.scroll.frame_rate);
        assert!((parsed.scroll.step_size - cfg.scroll.step_size).abs() < f64::EPSILON);
    }

    #[test]
    fn partial_toml_fills_defaults() {
        let text = r#"
[scroll]
step_size = 2.5
"#;
        let cfg: Config = toml::from_str(text).unwrap();
        assert!((cfg.scroll.step_size - 2.5).abs() < f64::EPSILON);
        // Other fields should be defaults
        assert_eq!(cfg.scroll.frame_rate, 150);
        assert_eq!(cfg.scroll.animation_time, 400);
    }

    #[test]
    fn sanitize_rejects_invalid_values() {
        let mut cfg = Config {
            scroll: ScrollConfig {
                frame_rate: 0,
                animation_time: 0,
                step_size: f64::NAN,
                pulse_algorithm: true,
                pulse_scale: -10.0,
                pulse_normalize: -1.0,
                inverted: false,
            },
            acceleration: AccelerationConfig {
                delta_ms: 0,
                max: 0.0,
            },
            output: OutputConfig {
                inject_threshold: f64::NEG_INFINITY,
                auto_detect: true,
                app_overrides: HashMap::new(),
            },
            general: GeneralConfig::default(),
            keyboard: KeyboardConfig::default(),
        };

        cfg.sanitize();

        assert_eq!(cfg.scroll.frame_rate, 30);
        assert_eq!(cfg.scroll.animation_time, 1);
        assert_eq!(cfg.scroll.step_size, 100.0);
        assert_eq!(cfg.scroll.pulse_scale, 4.0);
        assert_eq!(cfg.scroll.pulse_normalize, 1.0);
        assert_eq!(cfg.acceleration.delta_ms, 1);
        assert_eq!(cfg.acceleration.max, 1.0);
        assert_eq!(cfg.output.inject_threshold, 1.0);
    }

    #[test]
    fn sanitize_clamps_ranges() {
        let mut cfg = Config::default();

        cfg.scroll.step_size = 5_000.0;
        cfg.scroll.pulse_scale = 0.01;
        cfg.scroll.pulse_normalize = 100.0;
        cfg.output.inject_threshold = 500.0;

        cfg.sanitize();

        assert_eq!(cfg.scroll.step_size, 2_000.0);
        assert_eq!(cfg.scroll.pulse_scale, 0.1);
        assert_eq!(cfg.scroll.pulse_normalize, 10.0);
        assert_eq!(cfg.output.inject_threshold, 120.0);
    }

    #[test]
    fn config_parses_app_overrides() {
        let text = r#"
[output]
inject_threshold = 40.0

[output.app_overrides]
"C:\\Windows\\System32\\notepad.exe" = 120.0
"C:\\Program Files\\App\\modern.exe" = 1.0
"#;
        let cfg: Config = toml::from_str(text).unwrap();

        assert_eq!(cfg.output.app_overrides.len(), 2);
        assert_eq!(
            cfg.output
                .app_overrides
                .get("C:\\Windows\\System32\\notepad.exe")
                .copied(),
            Some(120.0)
        );
        assert_eq!(
            cfg.output
                .app_overrides
                .get("C:\\Program Files\\App\\modern.exe")
                .copied(),
            Some(1.0)
        );
    }

    #[test]
    fn config_default_has_empty_overrides() {
        assert!(OutputConfig::default().app_overrides.is_empty());
    }

    #[test]
    fn config_sanitizes_override_values() {
        let mut cfg = Config::default();
        cfg.output
            .app_overrides
            .insert("high.exe".to_string(), 500.0);
        cfg.output
            .app_overrides
            .insert("low.exe".to_string(), -10.0);

        cfg.sanitize();

        assert_eq!(
            cfg.output.app_overrides.get("high.exe").copied(),
            Some(120.0)
        );
        assert_eq!(cfg.output.app_overrides.get("low.exe").copied(), Some(1.0));
    }

    // -- Keyboard config tests ----------------------------------------------

    #[test]
    fn keyboard_defaults() {
        let cfg = KeyboardConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.mode, KeyboardMode::Always);
        // page_up_down inherits parent.
        assert_eq!(cfg.page_up_down.mode, None);
        // arrow_keys and space default to off (safety).
        assert_eq!(cfg.arrow_keys.mode, Some(KeyboardMode::Off));
        assert_eq!(cfg.space.mode, Some(KeyboardMode::Off));
    }

    #[test]
    fn keyboard_effective_mode_inherits() {
        let cfg = KeyboardConfig::default();
        // page_up_down.mode is None → inherits parent "always".
        assert_eq!(cfg.effective_mode(&cfg.page_up_down), KeyboardMode::Always);
        // arrow_keys has explicit "off" → overrides parent.
        assert_eq!(cfg.effective_mode(&cfg.arrow_keys), KeyboardMode::Off);
    }

    #[test]
    fn keyboard_effective_mode_override() {
        let mut cfg = KeyboardConfig::default();
        cfg.mode = KeyboardMode::Win32Scrollbar;
        // page_up_down inherits new parent mode.
        assert_eq!(
            cfg.effective_mode(&cfg.page_up_down),
            KeyboardMode::Win32Scrollbar
        );
        // arrow_keys still has its own override.
        assert_eq!(cfg.effective_mode(&cfg.arrow_keys), KeyboardMode::Off);

        // Explicitly set arrow_keys to always.
        cfg.arrow_keys.mode = Some(KeyboardMode::Always);
        assert_eq!(cfg.effective_mode(&cfg.arrow_keys), KeyboardMode::Always);
    }

    #[test]
    fn keyboard_toml_minimal() {
        let text = r#"
[keyboard]
enabled = true
"#;
        let cfg: Config = toml::from_str(text).unwrap();
        assert!(cfg.keyboard.enabled);
        assert_eq!(cfg.keyboard.mode, KeyboardMode::Always);
        // Sub-groups fall back to defaults.
        assert_eq!(cfg.keyboard.page_up_down.mode, None);
        assert_eq!(cfg.keyboard.arrow_keys.mode, Some(KeyboardMode::Off));
    }

    #[test]
    fn keyboard_toml_full() {
        let text = r#"
[keyboard]
enabled = true
mode = "win32_scrollbar"

[keyboard.page_up_down]
# inherits win32_scrollbar

[keyboard.arrow_keys]
mode = "win32_scrollbar"

[keyboard.space]
mode = "always"
"#;
        let cfg: Config = toml::from_str(text).unwrap();
        assert!(cfg.keyboard.enabled);
        assert_eq!(cfg.keyboard.mode, KeyboardMode::Win32Scrollbar);
        assert_eq!(cfg.keyboard.page_up_down.mode, None);
        assert_eq!(
            cfg.keyboard.arrow_keys.mode,
            Some(KeyboardMode::Win32Scrollbar)
        );
        assert_eq!(cfg.keyboard.space.mode, Some(KeyboardMode::Always));

        // Effective mode checks.
        assert_eq!(
            cfg.keyboard.effective_mode(&cfg.keyboard.page_up_down),
            KeyboardMode::Win32Scrollbar
        );
        assert_eq!(
            cfg.keyboard.effective_mode(&cfg.keyboard.arrow_keys),
            KeyboardMode::Win32Scrollbar
        );
        assert_eq!(
            cfg.keyboard.effective_mode(&cfg.keyboard.space),
            KeyboardMode::Always
        );
    }

    #[test]
    fn keyboard_toml_round_trip() {
        let cfg = Config::default();
        let text = toml::to_string_pretty(&cfg).unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(parsed.keyboard.enabled, cfg.keyboard.enabled);
        assert_eq!(parsed.keyboard.mode, cfg.keyboard.mode);
        assert_eq!(
            parsed.keyboard.arrow_keys.mode,
            cfg.keyboard.arrow_keys.mode
        );
    }

    #[test]
    fn keyboard_absent_section_uses_defaults() {
        // No [keyboard] section at all → entire struct defaults.
        let text = r#"
[scroll]
step_size = 50.0
"#;
        let cfg: Config = toml::from_str(text).unwrap();
        assert!(cfg.keyboard.enabled);
        assert_eq!(cfg.keyboard.mode, KeyboardMode::Always);
        assert_eq!(cfg.keyboard.arrow_keys.mode, Some(KeyboardMode::Off));
    }
}
