use serde::{Deserialize, Serialize};
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
    /// 40 = WHEEL_DELTA/3 (good balance, default)
    /// 1 = per-frame (smoothest, modern apps only)
    pub inject_threshold: f64,
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
            inject_threshold: 40.0,
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
        assert!((cfg.output.inject_threshold - 40.0).abs() < f64::EPSILON);
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
            },
            general: GeneralConfig::default(),
        };

        cfg.sanitize();

        assert_eq!(cfg.scroll.frame_rate, 30);
        assert_eq!(cfg.scroll.animation_time, 1);
        assert_eq!(cfg.scroll.step_size, 100.0);
        assert_eq!(cfg.scroll.pulse_scale, 4.0);
        assert_eq!(cfg.scroll.pulse_normalize, 1.0);
        assert_eq!(cfg.acceleration.delta_ms, 1);
        assert_eq!(cfg.acceleration.max, 1.0);
        assert_eq!(cfg.output.inject_threshold, 40.0);
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
}
