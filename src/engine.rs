use crate::config::Config;
use crate::detector::ScrollDetector;
use crate::pulse::Pulse;
use crate::resolve::ProcessResolver;
use crate::threshold::{AppKey, AppThresholdCache, ThresholdMode};
use crate::traits::{DetectRequest, EngineCommand, ScrollOutput, TimeSource};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
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

pub struct ScrollEngine {
    // DI dependencies
    time: Arc<dyn TimeSource>,
    output: Arc<dyn ScrollOutput>,
    resolver: Arc<dyn ProcessResolver>,
    rx: Receiver<EngineCommand>,
    detect_tx: Sender<DetectRequest>,

    // Config (owned copy — updated via Reload command)
    config: Config,

    // Animation state
    pulse: Pulse,
    queue: Vec<ScrollItem>,
    direction: (i8, i8),
    last_scroll_time: u64,
    current_target_pid: u32,
    pid_to_key: HashMap<u32, AppKey>,
    threshold_cache: Arc<Mutex<AppThresholdCache>>,

    // Accumulator — collects animation output and injects once configured
    // threshold is reached.
    pending_x: f64,
    pending_y: f64,
}

impl ScrollEngine {
    pub fn new(
        time: Arc<dyn TimeSource>,
        output: Arc<dyn ScrollOutput>,
        resolver: Arc<dyn ProcessResolver>,
        detector: Box<dyn ScrollDetector>,
        config: Config,
        tx: Sender<EngineCommand>,
        rx: Receiver<EngineCommand>,
    ) -> Self {
        let (detect_tx, detect_rx) = unbounded::<DetectRequest>();
        thread::spawn(move || {
            while let Ok(req) = detect_rx.recv() {
                let mode = detector.detect(req.hwnd, req.expected_delta);
                let _ = tx.send(EngineCommand::DetectResult {
                    app_key: req.app_key,
                    mode,
                });
            }
        });

        let pulse = Pulse::new(config.scroll.pulse_scale, config.scroll.pulse_normalize);
        Self {
            time,
            output,
            resolver,
            rx,
            detect_tx,
            config,
            pulse,
            queue: Vec::with_capacity(32),
            direction: (0, 0),
            last_scroll_time: 0,
            current_target_pid: 0,
            pid_to_key: HashMap::new(),
            threshold_cache: Arc::new(Mutex::new(AppThresholdCache::new())),
            pending_x: 0.0,
            pending_y: 0.0,
        }
    }

    pub fn set_threshold_cache(&mut self, cache: Arc<Mutex<AppThresholdCache>>) {
        self.threshold_cache = cache;
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
            // once threshold is reached.
            let (dx, dy) = self.tick();
            self.pending_x += dx as f64;
            self.pending_y += dy as f64;
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
        let delta_f = delta as f64;
        // Match original: only scale when |delta| > 1.2.
        let scaled = if delta_f.abs() > 1.2 {
            delta_f * self.config.scroll.step_size / 120.0 * sign
        } else {
            delta_f * sign
        };
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

    /// Process a pre-scaled scroll event (keyboard page/space scroll).
    /// Bypasses `step_size` normalization — the delta already represents
    /// the intended wheel output amount.  Only `inverted` is applied.
    pub(crate) fn handle_scroll_raw(&mut self, delta_y: f64) {
        if !self.config.general.enabled {
            let inject = delta_y.trunc() as i32;
            if inject != 0 {
                self.output.inject_wheel(0, inject);
            }
            return;
        }

        let sign = if self.config.scroll.inverted {
            -1.0
        } else {
            1.0
        };
        self.on_scroll(0.0, delta_y * sign);
    }

    /// Queue a scroll and apply acceleration (port of JS `scrollArray`).
    pub(crate) fn on_scroll(&mut self, x: f64, y: f64) {
        self.direction_check(x, y);

        let (x, y) = self.apply_acceleration(x, y);

        let now = self.time.now_ms();
        self.queue.push(ScrollItem {
            x,
            y,
            last_x: if x < 0.0 { 0.99 } else { -0.99 },
            last_y: if y < 0.0 { 0.99 } else { -0.99 },
            start: now,
        });
    }

    /// Tick all queued animations and return aggregate integer deltas for
    /// this frame (matching JS `>> 0` truncation semantics).
    pub(crate) fn tick(&mut self) -> (i32, i32) {
        let now = self.time.now_ms();
        let anim_time = self.config.scroll.animation_time as u64;
        let use_pulse = self.config.scroll.pulse_algorithm;

        let mut scroll_x: i32 = 0;
        let mut scroll_y: i32 = 0;

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

            let x = (item.x * position - item.last_x) as i32;
            let y = (item.y * position - item.last_y) as i32;

            scroll_x += x;
            scroll_y += y;

            item.last_x += x as f64;
            item.last_y += y as f64;

            !finished
        });

        (scroll_x, scroll_y)
    }

    pub(crate) fn flush_pending(&mut self) {
        let threshold = self.threshold_for_current_pid();
        const EPS: f64 = 1e-9;

        if self.pending_y.abs() + EPS >= threshold {
            let inject = self.pending_y.trunc() as i32;
            if inject != 0 {
                eprintln!(
                    "[engine] flush: inject_wheel(0, {inject}), pending_y was {:.1}",
                    self.pending_y
                );
                self.output.inject_wheel(0, inject);
                self.pending_y -= inject as f64;
            }
        }
        if self.pending_y.abs() < EPS {
            self.pending_y = 0.0;
        }

        if self.pending_x.abs() + EPS >= threshold {
            let inject = self.pending_x.trunc() as i32;
            if inject != 0 {
                self.output.inject_wheel(inject, 0);
                self.pending_x -= inject as f64;
            }
        }
        if self.pending_x.abs() < EPS {
            self.pending_x = 0.0;
        }
    }

    // -- private helpers ----------------------------------------------------

    fn threshold_for_current_pid(&self) -> f64 {
        if self.current_target_pid == 0 {
            return self.config.output.inject_threshold;
        }

        let app_key = self.pid_to_key.get(&self.current_target_pid);
        if app_key.is_none() {
            return self.config.output.inject_threshold;
        }

        if let Ok(cache) = self.threshold_cache.lock() {
            cache.get_threshold(app_key)
        } else {
            self.config.output.inject_threshold
        }
    }

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
        self.pulse = Pulse::new(config.scroll.pulse_scale, config.scroll.pulse_normalize);
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
            EngineCommand::Scroll {
                delta,
                horizontal,
                target_pid,
            } => {
                self.current_target_pid = target_pid;

                // Resolve new PIDs to AppKey and apply user overrides
                if target_pid != 0 && !self.pid_to_key.contains_key(&target_pid) {
                    if let Some(app_key) = self.resolver.resolve_pid(target_pid) {
                        // Check for user override first — bypasses detection
                        let override_val = self
                            .config
                            .output
                            .app_overrides
                            .get(app_key.exe_path.to_str().unwrap_or(""))
                            .copied();
                        if let Some(val) = override_val {
                            let mode = if val >= 100.0 {
                                ThresholdMode::Legacy120
                            } else {
                                ThresholdMode::SmoothOk
                            };
                            if let Ok(mut cache) = self.threshold_cache.lock() {
                                cache.set_mode(app_key.clone(), mode);
                            }
                        }
                        self.pid_to_key.insert(target_pid, app_key);
                    }
                }

                if let Some(app_key) = self.pid_to_key.get(&target_pid).cloned() {
                    let should_detect = {
                        if let Ok(mut cache) = self.threshold_cache.lock() {
                            cache.start_detecting(app_key.clone())
                        } else {
                            false
                        }
                    };

                    if should_detect {
                        let _ = self.detect_tx.send(DetectRequest {
                            hwnd: 0,
                            app_key,
                            expected_delta: self.config.scroll.step_size,
                        });
                    }
                }

                self.handle_scroll(delta, horizontal);
            }
            EngineCommand::ScrollRaw { delta_y } => {
                self.handle_scroll_raw(delta_y);
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
            EngineCommand::DetectResult { app_key, mode } => {
                if let Ok(mut cache) = self.threshold_cache.lock() {
                    cache.set_mode(app_key, mode);
                }
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
    use crate::detector::MockScrollDetector;
    use crate::resolve::MockProcessResolver;
    use crate::threshold::ThresholdMode;
    use crate::traits::{MockOutput, MockTime};
    use std::path::PathBuf;

    /// Helper — build an engine with mock deps (no channel needed for
    /// direct method testing).
    fn test_engine() -> (ScrollEngine, Arc<MockTime>, Arc<MockOutput>) {
        test_engine_with_resolver(None)
    }

    fn test_engine_with_resolver(
        result: Option<AppKey>,
    ) -> (ScrollEngine, Arc<MockTime>, Arc<MockOutput>) {
        let time = Arc::new(MockTime::new());
        let output = Arc::new(MockOutput::new());
        let resolver = Arc::new(MockProcessResolver { result });
        let detector = Box::new(MockScrollDetector {
            result: ThresholdMode::SmoothOk,
        });
        let config = Config::default();
        let (tx, rx) = crossbeam_channel::unbounded();
        let engine = ScrollEngine::new(time.clone(), output.clone(), resolver, detector, config, tx, rx);
        (engine, time, output)
    }

    #[test]
    fn single_scroll_total_delta() {
        let (mut engine, time, _output) = test_engine();
        engine.config.scroll.step_size = 100.0;
        let anim_time = engine.config.scroll.animation_time as u64;

        // One wheel-down notch (delta = -120).
        time.set(0);
        engine.handle_scroll(-120, false);

        // Walk through the full animation.
        let mut total_dy: i32 = 0;
        let step = 1000 / engine.config.scroll.frame_rate as u64; // ~6-7 ms
        let mut t = 0u64;
        while t <= anim_time + step {
            time.set(t);
            let (_dx, dy) = engine.tick();
            total_dy += dy;
            t += step;
        }

        // Match original: -120 * 100 / 120 = -100 (±2 from int truncation).
        assert!(
            (total_dy - (-100)).abs() <= 2,
            "total_dy={total_dy}, expected ~-100"
        );
    }

    #[test]
    fn touchpad_threshold_behavior() {
        let (mut engine, time, _output) = test_engine();
        engine.config.acceleration.max = 1.0;

        time.set(0);
        engine.handle_scroll(-2, false);
        let scaled_two = engine.queue.last().unwrap().y;

        time.set(1000);
        engine.handle_scroll(-1, false);
        let scaled_one = engine.queue.last().unwrap().y;

        assert!(
            (scaled_two - (-2.0 * engine.config.scroll.step_size / 120.0)).abs() < 1e-9,
            "delta=-2 should be scaled"
        );
        assert!(
            (scaled_one - (-1.0)).abs() < 1e-9,
            "delta=-1 should pass through"
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
        let mut total_dy: i32 = 0;
        let step = 7u64;
        let mut t = 0u64;
        while t <= anim_time + step + 20 {
            time.set(t);
            let (_, dy) = engine.tick();
            total_dy += dy;
            t += step;
        }

        // Total should be roughly the sum of both (accounting for acceleration).
        assert!(total_dy.abs() >= 180, "accumulated total_dy={total_dy}");
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
        engine.pending_y += dy as f64;
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
        engine.pending_y += dy as f64;

        assert!(engine.handle_command(EngineCommand::SetEnabled(false)));
        assert!(engine.queue.is_empty());
        assert_eq!(engine.pending_x, 0.0);
        assert_eq!(engine.pending_y, 0.0);
    }

    // -- Accumulator / flush_pending tests ----------------------------------

    #[test]
    fn flush_injects_at_threshold() {
        let (mut engine, time, output) = test_engine();
        engine.config.scroll.step_size = 100.0;
        engine.config.output.inject_threshold = 40.0;
        let anim_time = engine.config.scroll.animation_time as u64;

        // step_size=100 => total ≈ -100. threshold=40 should split into
        // multiple injections.
        time.set(0);
        engine.handle_scroll(-120, false);

        let step = 1000 / engine.config.scroll.frame_rate as u64;
        let mut t = 0u64;
        while t <= anim_time + step {
            time.set(t);
            let (dx, dy) = engine.tick();
            engine.pending_x += dx as f64;
            engine.pending_y += dy as f64;
            engine.flush_pending();
            t += step;
        }

        let events = output.drain();
        assert!(
            events.len() >= 2,
            "expected multiple injections, got {}",
            events.len()
        );
        let total: i32 = events.iter().map(|(_, y)| y).sum();
        let combined = total as f64 + engine.pending_y;
        assert!(
            (combined + 100.0).abs() <= 2.0,
            "injected+remainder should be ≈ -100, got injected={total}, remainder={}",
            engine.pending_y
        );
        for &(_, dy) in &events {
            assert!(dy < 0, "injection should be negative");
        }
    }

    #[test]
    fn flush_threshold_120_produces_single_wheel_delta_injection() {
        let (mut engine, time, output) = test_engine();
        engine.config.scroll.step_size = 120.0;
        engine.config.output.inject_threshold = 120.0;
        let anim_time = engine.config.scroll.animation_time as u64;

        time.set(0);
        engine.handle_scroll(-120, false);

        let step = 1000 / engine.config.scroll.frame_rate as u64;
        let mut t = 0u64;
        while t <= anim_time + step {
            time.set(t);
            let (dx, dy) = engine.tick();
            engine.pending_x += dx as f64;
            engine.pending_y += dy as f64;
            engine.flush_pending();
            t += step;
        }

        let events = output.drain();
        assert_eq!(
            events.len(),
            1,
            "threshold=120 should inject once for one notch"
        );
        let total: i32 = events.iter().map(|(_, y)| y).sum();
        assert_eq!(total, -120, "should inject a single WHEEL_DELTA");
    }

    #[test]
    fn flush_remainder_carries_over() {
        let (mut engine, _time, output) = test_engine();

        // Manually set pending below threshold — no injection.
        engine.pending_y = -30.0;
        engine.flush_pending();
        assert!(
            output.drain().is_empty(),
            "should not inject below threshold"
        );
        assert!(
            (engine.pending_y - (-30.0)).abs() < f64::EPSILON,
            "remainder should be preserved"
        );

        // Push over threshold — one injection, remainder kept.
        engine.pending_y = -55.0;
        engine.flush_pending();
        let events = output.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], (0, -55)); // injects the integer part
        assert!(
            engine.pending_y.abs() < 1.0,
            "fractional remainder: {}",
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
        engine.config.output.inject_threshold = 40.0;

        // Simulate float rounding around threshold: should still flush.
        engine.pending_y = -39.9999999997;
        engine.flush_pending();

        let events = output.drain();
        assert_eq!(events.len(), 1, "epsilon-close value should flush");
        assert_eq!(events[0], (0, -39));
        // trunc() keeps a small remainder close to -1 here.
        assert!(
            engine.pending_y < -0.9 && engine.pending_y > -1.1,
            "residual should be preserved, got {}",
            engine.pending_y
        );
    }

    #[test]
    fn flush_remainder_carries_across_frames_with_threshold_120() {
        let (mut engine, _time, output) = test_engine();
        engine.config.output.inject_threshold = 120.0;

        engine.pending_y = -100.0;
        engine.flush_pending();
        assert!(output.drain().is_empty());
        assert_eq!(engine.pending_y, -100.0);

        engine.pending_y += -30.0;
        engine.flush_pending();
        let events = output.drain();
        assert_eq!(events, vec![(0, -130)]);
        assert_eq!(engine.pending_y, 0.0);
    }

    #[test]
    fn flush_uses_per_app_threshold_legacy() {
        let (mut engine, _time, output) = test_engine();
        engine.config.output.inject_threshold = 40.0;

        let app_key = AppKey {
            exe_path: PathBuf::from("C:/legacy/app.exe"),
            exe_mtime: None,
        };
        let cache = Arc::new(Mutex::new(AppThresholdCache::new()));
        cache
            .lock()
            .unwrap()
            .set_mode(app_key.clone(), ThresholdMode::Legacy120);
        engine.set_threshold_cache(cache);
        engine.pid_to_key.insert(1234, app_key);
        engine.current_target_pid = 1234;

        engine.pending_y = -100.0;
        engine.flush_pending();
        assert!(output.drain().is_empty(), "legacy threshold=120 should not flush at 100");

        engine.pending_y = -120.0;
        engine.flush_pending();
        assert_eq!(output.drain(), vec![(0, -120)]);
    }

    #[test]
    fn flush_uses_per_app_threshold_smooth() {
        let (mut engine, _time, output) = test_engine();
        engine.config.output.inject_threshold = 120.0;

        let app_key = AppKey {
            exe_path: PathBuf::from("C:/smooth/app.exe"),
            exe_mtime: None,
        };
        let cache = Arc::new(Mutex::new(AppThresholdCache::new()));
        cache
            .lock()
            .unwrap()
            .set_mode(app_key.clone(), ThresholdMode::SmoothOk);
        engine.set_threshold_cache(cache);
        engine.pid_to_key.insert(5678, app_key);
        engine.current_target_pid = 5678;

        engine.pending_y = -1.0;
        engine.flush_pending();
        assert_eq!(output.drain(), vec![(0, -1)]);
    }

    #[test]
    fn flush_falls_back_to_global() {
        let (mut engine, _time, output) = test_engine();
        engine.config.output.inject_threshold = 40.0;
        engine.current_target_pid = 7777;

        engine.pending_y = -39.0;
        engine.flush_pending();
        assert!(output.drain().is_empty(), "should honor global threshold without pid mapping");

        engine.pending_y = -40.0;
        engine.flush_pending();
        assert_eq!(output.drain(), vec![(0, -40)]);
    }

    // -- PID resolution tests -----------------------------------------------

    #[test]
    fn pid_resolution_populates_key_map() {
        let app_key = AppKey {
            exe_path: PathBuf::from("C:/apps/test.exe"),
            exe_mtime: Some(12345),
        };
        let (mut engine, time, _output) = test_engine_with_resolver(Some(app_key.clone()));

        assert!(engine.pid_to_key.is_empty());

        time.set(0);
        engine.handle_command(EngineCommand::Scroll {
            delta: -120,
            horizontal: false,
            target_pid: 42,
        });

        assert_eq!(engine.pid_to_key.get(&42), Some(&app_key));
        assert_eq!(engine.current_target_pid, 42);
    }

    #[test]
    fn failed_resolution_uses_global_default() {
        // MockResolver returns None — simulates OpenProcess failure
        let (mut engine, time, output) = test_engine_with_resolver(None);
        engine.config.output.inject_threshold = 40.0;

        time.set(0);
        engine.handle_command(EngineCommand::Scroll {
            delta: -120,
            horizontal: false,
            target_pid: 999,
        });

        // No entry in pid_to_key
        assert!(!engine.pid_to_key.contains_key(&999));
        // Engine should still function — uses global threshold
        assert_eq!(engine.current_target_pid, 999);
        // Scroll was processed (queue has items or output was produced)
        assert!(!engine.queue.is_empty() || !output.drain().is_empty());
    }

    #[test]
    fn pid_resolution_applies_user_override_legacy() {
        let app_key = AppKey {
            exe_path: PathBuf::from("C:/legacy/notepad.exe"),
            exe_mtime: None,
        };
        let (mut engine, time, _output) = test_engine_with_resolver(Some(app_key.clone()));

        // Configure user override for this exe path
        engine
            .config
            .output
            .app_overrides
            .insert("C:/legacy/notepad.exe".to_string(), 120.0);

        time.set(0);
        engine.handle_command(EngineCommand::Scroll {
            delta: -120,
            horizontal: false,
            target_pid: 100,
        });

        // Should have resolved and applied override
        assert_eq!(engine.pid_to_key.get(&100), Some(&app_key));
        let cache = engine.threshold_cache.lock().unwrap();
        assert_eq!(
            cache.get_mode(&app_key),
            Some(&ThresholdMode::Legacy120),
            "user override >= 100 should set Legacy120"
        );
    }

    #[test]
    fn pid_resolution_applies_user_override_smooth() {
        let app_key = AppKey {
            exe_path: PathBuf::from("C:/modern/app.exe"),
            exe_mtime: None,
        };
        let (mut engine, time, _output) = test_engine_with_resolver(Some(app_key.clone()));

        engine
            .config
            .output
            .app_overrides
            .insert("C:/modern/app.exe".to_string(), 1.0);

        time.set(0);
        engine.handle_command(EngineCommand::Scroll {
            delta: -120,
            horizontal: false,
            target_pid: 200,
        });

        assert_eq!(engine.pid_to_key.get(&200), Some(&app_key));
        let cache = engine.threshold_cache.lock().unwrap();
        assert_eq!(
            cache.get_mode(&app_key),
            Some(&ThresholdMode::SmoothOk),
            "user override < 100 should set SmoothOk"
        );
    }

    #[test]
    fn pid_resolution_skips_already_resolved() {
        let app_key = AppKey {
            exe_path: PathBuf::from("C:/apps/cached.exe"),
            exe_mtime: None,
        };
        let (mut engine, time, _output) = test_engine_with_resolver(Some(app_key.clone()));

        // Pre-populate pid_to_key
        engine.pid_to_key.insert(50, app_key.clone());

        time.set(0);
        engine.handle_command(EngineCommand::Scroll {
            delta: -120,
            horizontal: false,
            target_pid: 50,
        });

        // Should still have the same entry (resolver not called again)
        assert_eq!(engine.pid_to_key.get(&50), Some(&app_key));
    }

    #[test]
    fn detection_triggers_for_unknown_app() {
        let app_key = AppKey {
            exe_path: PathBuf::from("C:/apps/detect.exe"),
            exe_mtime: None,
        };
        let (mut engine, time, _output) = test_engine_with_resolver(Some(app_key.clone()));

        time.set(0);
        engine.handle_command(EngineCommand::Scroll {
            delta: -120,
            horizontal: false,
            target_pid: 314,
        });

        std::thread::sleep(Duration::from_millis(10));
        assert!(engine.drain_commands());

        let cache = engine.threshold_cache.lock().unwrap();
        assert_eq!(cache.get_mode(&app_key), Some(&ThresholdMode::SmoothOk));
    }

    #[test]
    fn no_duplicate_detection_for_detecting_app() {
        let app_key = AppKey {
            exe_path: PathBuf::from("C:/apps/detect-once.exe"),
            exe_mtime: None,
        };
        let (mut engine, time, _output) = test_engine_with_resolver(Some(app_key.clone()));

        time.set(0);
        engine.handle_command(EngineCommand::Scroll {
            delta: -120,
            horizontal: false,
            target_pid: 2718,
        });

        {
            let cache = engine.threshold_cache.lock().unwrap();
            assert_eq!(cache.get_mode(&app_key), Some(&ThresholdMode::Detecting));
        }

        engine.handle_command(EngineCommand::Scroll {
            delta: -120,
            horizontal: false,
            target_pid: 2718,
        });

        {
            let cache = engine.threshold_cache.lock().unwrap();
            assert_eq!(cache.get_mode(&app_key), Some(&ThresholdMode::Detecting));
        }
    }
}
