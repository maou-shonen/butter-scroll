use crate::pulse::Pulse;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Easing type — selectable animation curve (preview feature)
// ---------------------------------------------------------------------------

/// Available easing algorithms for scroll animation.
///
/// `Pulse` is the original algorithm designed specifically for scrolling.
/// The others are standard Penner easing functions provided for comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EasingType {
    /// No easing — constant speed.
    Linear,
    /// Michael Herf pulse algorithm (original, scroll-optimized).
    #[default]
    Pulse,
    /// Cubic ease-out — standard, reliable.
    OutCubic,
    /// Quintic ease-out — snappier start.
    OutQuint,
    /// Exponential ease-out — very fast start, long tail (iOS-like).
    OutExpo,
    /// Circular ease-out — geometrically distinct from polynomial.
    OutCirc,
    /// Back ease-out — slight overshoot (~10%) then settle.
    OutBack,
}

impl EasingType {
    /// All variants, for UI enumeration.
    pub const ALL: &[EasingType] = &[
        EasingType::Pulse,
        EasingType::OutCubic,
        EasingType::OutQuint,
        EasingType::OutExpo,
        EasingType::OutCirc,
        EasingType::OutBack,
        EasingType::Linear,
    ];

    /// Whether this type uses the Pulse struct (which has its own config).
    pub fn is_pulse(self) -> bool {
        matches!(self, EasingType::Pulse)
    }
}

// ---------------------------------------------------------------------------
// Easing function — unified interface
// ---------------------------------------------------------------------------

/// Applies an easing curve to a linear progress value.
///
/// Wraps either the stateful `Pulse` struct or a stateless Penner function,
/// providing a uniform `apply(t) -> f64` interface to the engine.
#[derive(Debug, Clone)]
pub struct Easing {
    easing_type: EasingType,
    pulse: Pulse,
}

impl Easing {
    pub fn new(easing_type: EasingType, pulse_scale: f64, pulse_normalize: f64) -> Self {
        Self {
            easing_type,
            pulse: Pulse::new(pulse_scale, pulse_normalize),
        }
    }

    /// Evaluate the selected easing curve at `t` in \[0, 1\].
    pub fn apply(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return 0.0;
        }
        if t >= 1.0 {
            return 1.0;
        }
        match self.easing_type {
            EasingType::Linear => t,
            EasingType::Pulse => self.pulse.apply(t),
            EasingType::OutCubic => out_cubic(t),
            EasingType::OutQuint => out_quint(t),
            EasingType::OutExpo => out_expo(t),
            EasingType::OutCirc => out_circ(t),
            EasingType::OutBack => out_back(t),
        }
    }

    pub fn easing_type(&self) -> EasingType {
        self.easing_type
    }
}

// ---------------------------------------------------------------------------
// Penner easing functions — all take t ∈ (0, 1) and return f64
// ---------------------------------------------------------------------------

/// `1 - (1 - t)^3`
fn out_cubic(t: f64) -> f64 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

/// `1 - (1 - t)^5`
fn out_quint(t: f64) -> f64 {
    let inv = 1.0 - t;
    let sq = inv * inv;
    1.0 - sq * sq * inv
}

/// `1 - 2^(-10t)`
fn out_expo(t: f64) -> f64 {
    1.0 - (-10.0 * t).exp2()
}

/// `sqrt(1 - (t - 1)^2)`
fn out_circ(t: f64) -> f64 {
    let inv = t - 1.0;
    (1.0 - inv * inv).sqrt()
}

/// Back ease-out with standard overshoot constant (c1 = 1.70158).
/// `1 + c3 * (t-1)^3 + c1 * (t-1)^2`
fn out_back(t: f64) -> f64 {
    const C1: f64 = 1.70158;
    const C3: f64 = C1 + 1.0;
    let inv = t - 1.0;
    1.0 + C3 * inv * inv * inv + C1 * inv * inv
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_boundaries(easing: &Easing) {
        assert_eq!(easing.apply(0.0), 0.0);
        assert_eq!(easing.apply(1.0), 1.0);
        assert_eq!(easing.apply(-0.5), 0.0);
        assert_eq!(easing.apply(1.5), 1.0);
    }

    fn assert_monotonic(easing: &Easing) {
        let mut prev = 0.0;
        for i in 1..=1000 {
            let t = i as f64 / 1000.0;
            let v = easing.apply(t);
            assert!(
                v >= prev,
                "{:?} not monotonic at t={t:.4}: {v:.6} < {prev:.6}",
                easing.easing_type()
            );
            prev = v;
        }
    }

    #[test]
    fn all_types_satisfy_boundaries() {
        for &et in EasingType::ALL {
            let e = Easing::new(et, 4.0, 1.0);
            assert_boundaries(&e);
        }
    }

    #[test]
    fn monotonic_ease_out_types() {
        // OutBack intentionally overshoots, so skip monotonicity check.
        let monotonic_types = [
            EasingType::Linear,
            EasingType::Pulse,
            EasingType::OutCubic,
            EasingType::OutQuint,
            EasingType::OutExpo,
            EasingType::OutCirc,
        ];
        for &et in &monotonic_types {
            let e = Easing::new(et, 4.0, 1.0);
            assert_monotonic(&e);
        }
    }

    #[test]
    fn out_back_overshoots() {
        let e = Easing::new(EasingType::OutBack, 4.0, 1.0);
        // OutBack should exceed 1.0 somewhere in the middle.
        let has_overshoot = (1..1000).any(|i| {
            let t = i as f64 / 1000.0;
            e.apply(t) > 1.0
        });
        assert!(has_overshoot, "OutBack should overshoot past 1.0");
    }

    #[test]
    fn ease_out_midpoint_above_half() {
        // All ease-out curves should have midpoint > 0.5 (fast start).
        for &et in &[
            EasingType::Pulse,
            EasingType::OutCubic,
            EasingType::OutQuint,
            EasingType::OutExpo,
            EasingType::OutCirc,
        ] {
            let e = Easing::new(et, 4.0, 1.0);
            let mid = e.apply(0.5);
            assert!(mid > 0.5, "{et:?} midpoint {mid:.4} should be > 0.5");
        }
    }

    #[test]
    fn linear_is_identity() {
        let e = Easing::new(EasingType::Linear, 4.0, 1.0);
        for i in 0..=100 {
            let t = i as f64 / 100.0;
            assert!(
                (e.apply(t) - t).abs() < 1e-12,
                "linear({t}) should equal {t}"
            );
        }
    }

    #[test]
    fn penner_reference_values() {
        // Pre-computed reference values for validation.
        let cases: &[(EasingType, f64, f64)] = &[
            (EasingType::OutCubic, 0.5, 0.875),
            (EasingType::OutQuint, 0.5, 0.96875),
            (EasingType::OutExpo, 0.5, 0.96875),   // 1 - 2^(-5)
            (EasingType::OutCirc, 0.5, 0.8660254), // sqrt(3)/2
        ];
        for &(et, t, expected) in cases {
            let e = Easing::new(et, 4.0, 1.0);
            let v = e.apply(t);
            assert!(
                (v - expected).abs() < 0.001,
                "{et:?}({t}) = {v:.6}, expected ~{expected:.6}"
            );
        }
    }

    #[test]
    fn serde_round_trip() {
        for &et in EasingType::ALL {
            let json = serde_json::to_string(&et).unwrap();
            let parsed: EasingType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, et, "round-trip failed for {et:?} (json: {json})");
        }
    }

    #[test]
    fn serde_toml_round_trip() {
        // Simulate how it appears in TOML config.
        for &et in EasingType::ALL {
            let toml_str = toml::to_string(&et).unwrap();
            let parsed: EasingType = toml::from_str(&toml_str).unwrap();
            assert_eq!(parsed, et, "TOML round-trip failed for {et:?}");
        }
    }
}
