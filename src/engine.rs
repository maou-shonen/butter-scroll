use crate::config::Config;
use crate::pulse::Pulse;
use crate::traits::{EngineCommand, ScrollOutput, TimeSource};
use crossbeam_channel::Receiver;
use std::sync::Arc;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Scroll queue item (mirrors the JS `que[]` entry)
// ---------------------------------------------------------------------------

struct ScrollItem {
    x: f64,
    y: f64,
    last_x: f64,
    last_y: f64,
    start: u64, // ms from TimeSource epoch
}

// ---------------------------------------------------------------------------
// Scroll engine — pure algorithm, zero platform dependencies
// ---------------------------------------------------------------------------

/// Windows WHEEL_DELTA constant — the minimum meaningful wheel event unit.
const WHEEL_DELTA: f64 = 120.0;

pub struct ScrollEngine {
    // DI dependencies
    time: Arc<dyn TimeSource>,
    output: Arc<dyn ScrollOutput>,
    rx: Receiver<EngineCommand>,

    // Config (owned copy — updated via Reload command)
    config: Config,

    // Animation state
    pulse: Pulse,
    queue: Vec<ScrollItem>,
    direction: (i8, i8),
    last_scroll_time: u64,

    // Accumulator — collects fractional animation output and only injects
    // when the accumulated amount reaches ±WHEEL_DELTA (120).  This is
    // necessary because Windows apps expect wheel events in multiples of
    // WHEEL_DELTA; sub-threshold values are silently ignored by most apps.
    pending_x: f64,
    pending_y: f64,
}

impl ScrollEngine {
    pub fn new(
        time: Arc<dyn TimeSource>,
        output: Arc<dyn ScrollOutput>,
        config: Config,
        rx: Receiver<EngineCommand>,
    ) -> Self {
        let pulse = Pulse::new(config.scroll.pulse_scale);
        Self {
            time,
            output,
            rx,
            config,
            pulse,
            queue: Vec::with_capacity(32),
            direction: (0, 0),
            last_scroll_time: 0,
            pending_x: 0.0,
            pending_y: 0.0,
        }
    }

    // -- public (for tests) -------------------------------------------------

    /// Main loop — blocks the calling thread until `Stop` is received.
    pub fn run(&mut self) {
        loop {
            let frame_start = std::time::Instant::now();

            // Drain all pending commands.
            if !self.drain_commands() {
                return;
            }

            // Idle when nothing to animate — block until next event.
            if self.queue.is_empty() {
                match self.rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(cmd) => {
                        if !self.handle_command(cmd) {
                            return;
                        }
                        continue; // jump straight into the animation loop
                    }
                    Err(_) => continue,
                }
            }

            // Tick all active animations, accumulate output, and inject
            // WHEEL_DELTA-sized chunks when the threshold is reached.
            let (dx, dy) = self.tick();
            self.pending_x += dx;
            self.pending_y += dy;
            self.flush_pending();

            // Frame pacing — sleep for the remainder of the frame budget.
            let budget =
                Duration::from_micros(1_000_000 / self.config.scroll.frame_rate.max(1) as u64);
            let spent = frame_start.elapsed();
            if spent < budget {
                std::thread::sleep(budget - spent);
            }
        }
    }

    // -- internals (pub(crate) for testing) ---------------------------------

    /// Process a single incoming scroll event from the hook.
    pub(crate) fn handle_scroll(&mut self, delta: i16, horizontal: bool) {
        if !self.config.general.enabled {
            // Disabled — pass through the raw delta.
            let (dx, dy) = if horizontal {
                (delta as i32, 0)
            } else {
                (0, delta as i32)
            };
            self.output.inject_wheel(dx, dy);
            return;
        }

        let sign = if self.config.scroll.inverted {
            -1.0
        } else {
            1.0
        };
        // step_size is a multiplier on the original wheel delta.
        // 1.0 = one WHEEL_DELTA per notch (1:1 mapping).
        // 3.0 = three WHEEL_DELTA injections per notch (faster scroll).
        let scaled = delta as f64 * self.config.scroll.step_size * sign;
        eprintln!(
            "[engine] handle_scroll: delta={delta}, step_size={}, scaled={scaled:.1}",
            self.config.scroll.step_size
        );

        let (dx, dy) = if horizontal {
            (scaled, 0.0)
        } else {
            (0.0, scaled)
        };

        self.on_scroll(dx, dy);
    }

    /// Queue a scroll and apply acceleration (port of JS `scrollArray`).
    pub(crate) fn on_scroll(&mut self, x: f64, y: f64) {
        self.direction_check(x, y);

        let (x, y) = self.apply_acceleration(x, y);

        let now = self.time.now_ms();
        self.queue.push(ScrollItem {
            x,
            y,
            last_x: 0.0,
            last_y: 0.0,
            start: now,
        });
    }

    /// Tick all queued animations and return the aggregate delta for this
    /// frame as exact floating-point values.  The caller (run loop)
    /// accumulates these into `pending_x/pending_y` and injects
    /// WHEEL_DELTA-sized chunks via `flush_pending()`.
    pub(crate) fn tick(&mut self) -> (f64, f64) {
        let now = self.time.now_ms();
        let anim_time = self.config.scroll.animation_time as u64;
        let use_pulse = self.config.scroll.pulse_algorithm;

        let mut scroll_x: f64 = 0.0;
        let mut scroll_y: f64 = 0.0;

        self.queue.retain_mut(|item| {
            let elapsed = now.saturating_sub(item.start);
            let finished = elapsed >= anim_time;

            let position = if finished {
                1.0
            } else {
                elapsed as f64 / anim_time as f64
            };

            let position = if use_pulse {
                self.pulse.apply(position)
            } else {
                position
            };

            let x = item.x * position - item.last_x;
            let y = item.y * position - item.last_y;

            scroll_x += x;
            scroll_y += y;

            item.last_x += x;
            item.last_y += y;

            !finished
        });

        (scroll_x, scroll_y)
    }

    /// Drain the accumulator in ±WHEEL_DELTA (120) chunks.  Each chunk
    /// becomes a single `SendInput(MOUSEEVENTF_WHEEL)` call whose delta
    /// is exactly ±120 — a value every Windows application handles.
    ///
    /// An epsilon tolerance (1e-9) prevents floating-point rounding from
    /// leaving the accumulator stuck at values like 119.9999999997.
    pub(crate) fn flush_pending(&mut self) {
        const EPS: f64 = 1e-9;

        while self.pending_y.abs() + EPS >= WHEEL_DELTA {
            let sign = if self.pending_y > 0.0 { 1 } else { -1 };
            let inject = sign * WHEEL_DELTA as i32;
            eprintln!(
                "[engine] flush: inject_wheel(0, {inject}), pending_y was {:.1}",
                self.pending_y
            );
            self.output.inject_wheel(0, inject);
            self.pending_y -= sign as f64 * WHEEL_DELTA;
        }
        // Zero out negligible residuals left by float arithmetic.
        if self.pending_y.abs() < EPS {
            self.pending_y = 0.0;
        }

        while self.pending_x.abs() + EPS >= WHEEL_DELTA {
            let sign = if self.pending_x > 0.0 { 1 } else { -1 };
            self.output.inject_wheel(sign * WHEEL_DELTA as i32, 0);
            self.pending_x -= sign as f64 * WHEEL_DELTA;
        }
        if self.pending_x.abs() < EPS {
            self.pending_x = 0.0;
        }
    }

    // -- private helpers ----------------------------------------------------

    /// If the scroll direction changed, clear the queue and reset
    /// acceleration (port of JS `directionCheck`).
    fn direction_check(&mut self, x: f64, y: f64) {
        let dx = if x > 0.0 {
            1i8
        } else if x < 0.0 {
            -1
        } else {
            0
        };
        let dy = if y > 0.0 {
            1i8
        } else if y < 0.0 {
            -1
        } else {
            0
        };

        // Only check axes that are actually moving.
        let changed = (dx != 0 && self.direction.0 != dx) || (dy != 0 && self.direction.1 != dy);

        if changed {
            self.direction = (dx, dy);
            self.queue.clear();
            self.pending_x = 0.0;
            self.pending_y = 0.0;
            self.last_scroll_time = 0;
        }
    }

    /// Frequency-based acceleration (port of JS acceleration logic).
    ///
    /// `factor = (1 + 50 / elapsed_ms) / 2`, clamped to `acceleration.max`.
    fn apply_acceleration(&mut self, mut x: f64, mut y: f64) -> (f64, f64) {
        if self.config.acceleration.max <= 1.0 {
            return (x, y);
        }

        let now = self.time.now_ms();

        // First observed scroll event establishes the baseline timestamp.
        if self.last_scroll_time == 0 {
            self.last_scroll_time = now;
            return (x, y);
        }

        let elapsed = now.saturating_sub(self.last_scroll_time);

        if elapsed < self.config.acceleration.delta_ms as u64 {
            // `elapsed == 0` occurs on very high-frequency devices when two
            // wheel events land in the same millisecond. Treat it as max boost
            // for parity with smoothscroll's clamped behavior.
            let factor = if elapsed == 0 {
                self.config.acceleration.max
            } else {
                ((1.0 + 50.0 / elapsed as f64) / 2.0).min(self.config.acceleration.max)
            };
            if factor > 1.0 {
                x *= factor;
                y *= factor;
            }
        }

        self.last_scroll_time = now;
        (x, y)
    }

    fn apply_config(&mut self, config: Config) {
        // Reload semantics: do not mix old queue items with new pulse/timing.
        self.queue.clear();
        self.pending_x = 0.0;
        self.pending_y = 0.0;
        self.last_scroll_time = 0;
        self.direction = (0, 0);
        self.pulse = Pulse::new(config.scroll.pulse_scale);
        self.config = config;
    }

    /// Drain and process all pending commands. Returns `false` on `Stop`.
    fn drain_commands(&mut self) -> bool {
        while let Ok(cmd) = self.rx.try_recv() {
            if !self.handle_command(cmd) {
                return false;
            }
        }
        true
    }

    fn handle_command(&mut self, cmd: EngineCommand) -> bool {
        match cmd {
            EngineCommand::Scroll { delta, horizontal } => {
                self.handle_scroll(delta, horizontal);
            }
            EngineCommand::SetEnabled(on) => {
                self.config.general.enabled = on;
                if !on {
                    self.queue.clear();
                    self.pending_x = 0.0;
                    self.pending_y = 0.0;
                    self.direction = (0, 0);
                    self.last_scroll_time = 0;
                }
            }
            EngineCommand::Reload(cfg) => {
                self.apply_config(*cfg);
            }
            EngineCommand::Stop => return false,
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{MockOutput, MockTime};

    /// Helper — build an engine with mock deps (no channel needed for
    /// direct method testing).
    fn test_engine() -> (ScrollEngine, Arc<MockTime>, Arc<MockOutput>) {
        let time = Arc::new(MockTime::new());
        let output = Arc::new(MockOutput::new());
        let config = Config::default();
        let (_tx, rx) = crossbeam_channel::unbounded();
        let engine = ScrollEngine::new(time.clone(), output.clone(), config, rx);
        (engine, time, output)
    }

    #[test]
    fn single_scroll_total_delta() {
        let (mut engine, time, _output) = test_engine();
        engine.config.scroll.step_size = 1.0; // pin for deterministic assertion
        let anim_time = engine.config.scroll.animation_time as u64;

        // One wheel-down notch (delta = -120).
        time.set(0);
        engine.handle_scroll(-120, false);

        // Walk through the full animation.
        let mut total_dy: f64 = 0.0;
        let step = 1000 / engine.config.scroll.frame_rate as u64; // ~6-7 ms
        let mut t = 0u64;
        while t <= anim_time + step {
            time.set(t);
            let (_dx, dy) = engine.tick();
            total_dy += dy;
            t += step;
        }

        // Total should be approximately delta * step_size = -120 * 1.0 = -120.
        assert!(
            (total_dy - (-120.0)).abs() < 1.0,
            "total_dy={total_dy:.2}, expected ~-120.0"
        );
    }

    #[test]
    fn disabled_passes_through_raw() {
        let (mut engine, _time, output) = test_engine();
        engine.config.general.enabled = false;

        engine.handle_scroll(-120, false);

        let events = output.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], (0, -120));
    }

    #[test]
    fn direction_change_clears_queue() {
        let (mut engine, time, _output) = test_engine();

        time.set(0);
        engine.on_scroll(0.0, -100.0); // down
        assert!(!engine.queue.is_empty());

        time.set(10);
        engine.on_scroll(0.0, 100.0); // up — opposite direction
                                      // Queue should have been cleared then one new item added.
        assert_eq!(engine.queue.len(), 1);
        assert!(engine.queue[0].y > 0.0);
    }

    #[test]
    fn acceleration_increases_delta() {
        let (mut engine, time, _output) = test_engine();
        let base = engine.config.scroll.step_size;

        // First scroll — no acceleration.
        time.set(100);
        engine.on_scroll(0.0, -base);
        let first_y = engine.queue.last().unwrap().y;

        // Second scroll within delta_ms window — should be accelerated.
        time.set(130); // 30 ms later (< delta_ms=50)
        engine.on_scroll(0.0, -base);
        let second_y = engine.queue.last().unwrap().y;

        assert!(
            second_y.abs() > first_y.abs(),
            "second ({second_y}) should be larger than first ({first_y})"
        );
    }

    #[test]
    fn acceleration_resets_outside_window() {
        let (mut engine, time, _output) = test_engine();
        let base = engine.config.scroll.step_size;

        time.set(100);
        engine.on_scroll(0.0, -base);

        // Wait beyond the acceleration window.
        time.set(300); // 200 ms later (> delta_ms=50)
        engine.on_scroll(0.0, -base);

        // Both scrolls should have the same magnitude (no acceleration).
        let q = &engine.queue;
        assert!(
            (q[0].y.abs() - q[1].y.abs()).abs() < 1.0,
            "magnitudes should be equal: {} vs {}",
            q[0].y,
            q[1].y
        );
    }

    #[test]
    fn multiple_scrolls_accumulate() {
        let (mut engine, time, _output) = test_engine();
        let anim_time = engine.config.scroll.animation_time as u64;

        // Two rapid scrolls.
        time.set(0);
        engine.on_scroll(0.0, -100.0);
        time.set(20);
        engine.on_scroll(0.0, -100.0);

        assert_eq!(engine.queue.len(), 2, "both items should be in queue");

        // Walk through full animation.
        let mut total_dy: f64 = 0.0;
        let step = 7u64;
        let mut t = 0u64;
        while t <= anim_time + step + 20 {
            time.set(t);
            let (_, dy) = engine.tick();
            total_dy += dy;
            t += step;
        }

        // Total should be roughly the sum of both (accounting for acceleration).
        assert!(
            total_dy.abs() >= 180.0, // at least ~200 minus rounding
            "accumulated total_dy={total_dy:.2}"
        );
    }

    #[test]
    fn inverted_direction() {
        let (mut engine, time, _output) = test_engine();
        engine.config.scroll.inverted = true;

        time.set(0);
        engine.handle_scroll(-120, false);

        // With inversion, a negative delta (scroll down) should become positive.
        assert!(
            engine.queue[0].y > 0.0,
            "inverted scroll should flip direction"
        );
    }

    #[test]
    fn zero_elapsed_uses_max_acceleration() {
        let (mut engine, time, _output) = test_engine();
        let base = 100.0;

        // First event sets last_scroll_time.
        time.set(42);
        engine.on_scroll(0.0, -base);
        let first = engine.queue.last().unwrap().y.abs();

        // Same timestamp => elapsed == 0, should use acceleration.max.
        time.set(42);
        engine.on_scroll(0.0, -base);
        let second = engine.queue.last().unwrap().y.abs();

        let expected = base * engine.config.acceleration.max;
        assert!((first - base).abs() < 0.001);
        assert!((second - expected).abs() < 0.001);
    }

    #[test]
    fn reload_clears_inflight_queue() {
        let (mut engine, time, _output) = test_engine();
        time.set(0);
        engine.on_scroll(0.0, -100.0);
        assert_eq!(engine.queue.len(), 1);

        // Simulate partial animation to build up some pending.
        time.set(50);
        let (_, dy) = engine.tick();
        engine.pending_y += dy;
        assert!(engine.pending_y.abs() > 0.0);

        let mut new_cfg = engine.config.clone();
        new_cfg.scroll.pulse_scale = 8.0;
        engine.apply_config(new_cfg);

        assert!(engine.queue.is_empty());
        assert_eq!(engine.pending_x, 0.0);
        assert_eq!(engine.pending_y, 0.0);
        assert_eq!(engine.last_scroll_time, 0);
        assert_eq!(engine.direction, (0, 0));
    }

    #[test]
    fn disable_clears_inflight_queue() {
        let (mut engine, time, _output) = test_engine();
        time.set(0);
        engine.on_scroll(0.0, -100.0);
        assert_eq!(engine.queue.len(), 1);

        // Build up some pending.
        time.set(50);
        let (_, dy) = engine.tick();
        engine.pending_y += dy;

        assert!(engine.handle_command(EngineCommand::SetEnabled(false)));
        assert!(engine.queue.is_empty());
        assert_eq!(engine.pending_x, 0.0);
        assert_eq!(engine.pending_y, 0.0);
    }

    // -- Accumulator / flush_pending tests ----------------------------------

    #[test]
    fn flush_injects_wheel_delta_chunks() {
        let (mut engine, time, output) = test_engine();
        // Use step_size=1.0 for a simple 1:1 test.
        engine.config.scroll.step_size = 1.0;
        let anim_time = engine.config.scroll.animation_time as u64;

        // Total animation output ≈ -120 → exactly one WHEEL_DELTA injection.
        time.set(0);
        engine.handle_scroll(-120, false);

        let step = 1000 / engine.config.scroll.frame_rate as u64;
        let mut t = 0u64;
        while t <= anim_time + step {
            time.set(t);
            let (dx, dy) = engine.tick();
            engine.pending_x += dx;
            engine.pending_y += dy;
            engine.flush_pending();
            t += step;
        }

        let events = output.drain();
        assert_eq!(events.len(), 1, "expected 1 WHEEL_DELTA injection");
        assert_eq!(events[0], (0, -120));
    }

    #[test]
    fn flush_multiple_chunks_default_step_size() {
        let (mut engine, time, output) = test_engine();
        let anim_time = engine.config.scroll.animation_time as u64;

        // Default step_size=3.0: total ≈ -360 → 3 injections of -120.
        time.set(0);
        engine.handle_scroll(-120, false);

        let step = 1000 / engine.config.scroll.frame_rate as u64;
        let mut t = 0u64;
        while t <= anim_time + step {
            time.set(t);
            let (dx, dy) = engine.tick();
            engine.pending_x += dx;
            engine.pending_y += dy;
            engine.flush_pending();
            t += step;
        }

        let events = output.drain();
        assert_eq!(events.len(), 3, "expected 3 WHEEL_DELTA injections");
        for &(dx, dy) in &events {
            assert_eq!((dx, dy), (0, -120));
        }
    }

    #[test]
    fn flush_remainder_carries_over() {
        let (mut engine, _time, output) = test_engine();

        // Manually set pending below threshold — no injection.
        engine.pending_y = -100.0;
        engine.flush_pending();
        assert!(
            output.drain().is_empty(),
            "should not inject below WHEEL_DELTA"
        );
        assert!(
            (engine.pending_y - (-100.0)).abs() < f64::EPSILON,
            "remainder should be preserved"
        );

        // Push over threshold — one injection, remainder kept.
        engine.pending_y -= 30.0; // now -130
        engine.flush_pending();
        let events = output.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], (0, -120));
        assert!(
            (engine.pending_y - (-10.0)).abs() < 0.01,
            "remainder after injection: {}",
            engine.pending_y
        );
    }

    #[test]
    fn direction_change_clears_pending() {
        let (mut engine, time, _output) = test_engine();

        time.set(0);
        engine.on_scroll(0.0, -100.0);
        engine.pending_y = -80.0; // simulate partial accumulation

        time.set(10);
        engine.on_scroll(0.0, 100.0); // reverse direction

        assert_eq!(
            engine.pending_y, 0.0,
            "pending should be cleared on direction change"
        );
    }

    #[test]
    fn disable_resets_all_state() {
        let (mut engine, time, _output) = test_engine();

        // Build up state: queue, pending, direction, acceleration timestamp.
        time.set(100);
        engine.on_scroll(0.0, -120.0);
        time.set(130);
        engine.on_scroll(0.0, -120.0); // sets last_scroll_time, direction
        engine.pending_y = -80.0;

        assert!(!engine.queue.is_empty());
        assert_ne!(engine.direction, (0, 0));
        assert_ne!(engine.last_scroll_time, 0);

        // Disable should reset everything, just like apply_config.
        engine.handle_command(EngineCommand::SetEnabled(false));

        assert!(engine.queue.is_empty());
        assert_eq!(engine.pending_x, 0.0);
        assert_eq!(engine.pending_y, 0.0);
        assert_eq!(engine.direction, (0, 0));
        assert_eq!(engine.last_scroll_time, 0);
    }

    #[test]
    fn flush_handles_float_epsilon() {
        let (mut engine, _time, output) = test_engine();

        // Simulate float rounding: 119.9999999997 should still flush.
        engine.pending_y = -119.9999999997;
        engine.flush_pending();

        let events = output.drain();
        assert_eq!(events.len(), 1, "epsilon-close value should flush");
        assert_eq!(events[0], (0, -120));
        // Residual should be zeroed out (below epsilon).
        assert_eq!(engine.pending_y, 0.0, "residual should be zeroed");
    }
}
