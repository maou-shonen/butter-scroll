use crate::config::Config;
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

// ---------------------------------------------------------------------------
// DI traits
// ---------------------------------------------------------------------------

/// Monotonic time source (milliseconds since an arbitrary epoch).
pub trait TimeSource: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// Scroll event injection — the only side-effect the engine produces.
pub trait ScrollOutput: Send + Sync {
    fn inject_wheel(&self, delta_x: i32, delta_y: i32);
}

// ---------------------------------------------------------------------------
// Commands sent to the scroll engine thread
// ---------------------------------------------------------------------------

pub enum EngineCommand {
    /// New scroll event from the mouse hook.
    Scroll { delta: i16, horizontal: bool },
    /// Pre-scaled scroll — bypasses `step_size / 120` normalization.
    /// The delta already represents the intended wheel output amount.
    /// Used for keyboard page/space scrolling where the distance should
    /// not depend on the mouse wheel sensitivity setting.
    ScrollRaw { delta_y: f64 },
    /// Toggle global enable/disable.
    SetEnabled(bool),
    /// Hot-reload config.
    Reload(Box<Config>),
    /// Shut down the engine thread.
    Stop,
}

// ---------------------------------------------------------------------------
// Production implementations
// ---------------------------------------------------------------------------

/// System clock backed by `std::time::Instant` (monotonic, high-resolution).
pub struct SystemClock {
    epoch: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl TimeSource for SystemClock {
    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }
}

// ---------------------------------------------------------------------------
// Test doubles
// ---------------------------------------------------------------------------

/// Controllable time source for deterministic tests.
#[cfg(test)]
pub struct MockTime {
    ms: AtomicU64,
}

#[cfg(test)]
impl MockTime {
    pub fn new() -> Self {
        Self {
            ms: AtomicU64::new(0),
        }
    }

    pub fn set(&self, ms: u64) {
        self.ms.store(ms, Ordering::SeqCst);
    }

    pub fn advance(&self, delta: u64) {
        self.ms.fetch_add(delta, Ordering::SeqCst);
    }
}

#[cfg(test)]
impl TimeSource for MockTime {
    fn now_ms(&self) -> u64 {
        self.ms.load(Ordering::SeqCst)
    }
}

/// Recording scroll output for tests.
#[cfg(test)]
pub struct MockOutput {
    events: std::sync::Mutex<Vec<(i32, i32)>>,
}

#[cfg(test)]
impl MockOutput {
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn drain(&self) -> Vec<(i32, i32)> {
        self.events.lock().unwrap().drain(..).collect()
    }

    pub fn total(&self) -> (i64, i64) {
        self.events
            .lock()
            .unwrap()
            .iter()
            .fold((0i64, 0i64), |(ax, ay), &(x, y)| {
                (ax + x as i64, ay + y as i64)
            })
    }
}

#[cfg(test)]
impl ScrollOutput for MockOutput {
    fn inject_wheel(&self, delta_x: i32, delta_y: i32) {
        self.events.lock().unwrap().push((delta_x, delta_y));
    }
}
