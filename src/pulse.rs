/// Pulse easing — ported from gblazex/smoothscroll.
///
/// Based on Michael Herf's viscous fluid algorithm
/// (<http://stereopsis.com/stopping/>):
///
/// *  Phase 1 (x < 1 after scaling): damped acceleration
///    `val = x - (1 - e^(-x))`
/// *  Phase 2 (x >= 1): exponential deceleration (tail)
///    `val = e^(-1) + (1 - e^(-(x-1))) * (1 - e^(-1))`
///
/// The normalization factor ensures `pulse(1.0) == 1.0`.
#[derive(Debug, Clone)]
pub struct Pulse {
    scale: f64,
    normalize: f64,
}

impl Pulse {
    /// Create a new pulse with the given `scale` (default: 4.0).
    pub fn new(scale: f64) -> Self {
        // Compute normalization so that pulse(1.0) = 1.0.
        let raw_one = Self::raw(1.0, scale, 1.0);
        Self {
            scale,
            normalize: 1.0 / raw_one,
        }
    }

    /// Evaluate the pulse curve at `t` in \[0, 1\].
    pub fn apply(&self, t: f64) -> f64 {
        if t >= 1.0 {
            return 1.0;
        }
        if t <= 0.0 {
            return 0.0;
        }
        Self::raw(t, self.scale, self.normalize)
    }

    /// Raw (un-clamped) pulse evaluation.
    fn raw(t: f64, scale: f64, normalize: f64) -> f64 {
        let x = t * scale;
        let val = if x < 1.0 {
            // Acceleration phase — damped spring start.
            x - (1.0 - (-x).exp())
        } else {
            // Deceleration phase — exponential tail.
            let start = (-1.0_f64).exp(); // value at boundary
            let tail = x - 1.0;
            let decay = 1.0 - (-tail).exp();
            start + decay * (1.0 - start)
        };
        val * normalize
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_pulse() -> Pulse {
        Pulse::new(4.0)
    }

    #[test]
    fn boundaries() {
        let p = default_pulse();
        assert_eq!(p.apply(0.0), 0.0);
        assert_eq!(p.apply(1.0), 1.0);
        assert_eq!(p.apply(-0.5), 0.0);
        assert_eq!(p.apply(1.5), 1.0);
    }

    #[test]
    fn monotonically_increasing() {
        let p = default_pulse();
        let mut prev = 0.0;
        for i in 1..=1000 {
            let t = i as f64 / 1000.0;
            let v = p.apply(t);
            assert!(v >= prev, "not monotonic at t={t:.4}: {v:.6} < {prev:.6}");
            prev = v;
        }
    }

    #[test]
    fn midpoint_ease_out_behaviour() {
        // Pulse is ease-out: midpoint should be well above 0.5.
        let p = default_pulse();
        assert!(
            p.apply(0.5) > 0.5,
            "midpoint {:.4} should be > 0.5",
            p.apply(0.5)
        );
    }

    #[test]
    fn different_scales() {
        for &scale in &[1.0, 2.0, 4.0, 8.0] {
            let p = Pulse::new(scale);
            assert_eq!(p.apply(0.0), 0.0, "scale={scale}");
            assert!((p.apply(1.0) - 1.0).abs() < 1e-12, "scale={scale}");
        }
    }

    #[test]
    fn values_match_js_reference() {
        // Pre-computed from the original JS formula with pulseScale=4.
        let p = default_pulse();
        let expected = [
            (0.1, 0.072605),
            (0.25, 0.379833),
            (0.5, 0.792394),
            (0.75, 0.944166),
            (0.9, 0.984019),
        ];
        for (t, approx) in expected {
            let v = p.apply(t);
            assert!(
                (v - approx).abs() < 0.0005,
                "pulse({t}) = {v:.4}, expected ~{approx}"
            );
        }
    }
}
