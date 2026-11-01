//! Startup / refresh stopwatch.
//!
//! Every milestone logs `[perf] <elapsed> ms  <label>` relative to process start,
//! so `librus.log` alone answers "how long did the user wait, and for what?".
//! Cheap enough (one Instant + one log line) to leave on permanently.

use std::sync::OnceLock;
use std::time::Instant;

static BOOT: OnceLock<Instant> = OnceLock::new();

/// Call once, as early as possible in `main`.
pub fn boot() {
    BOOT.get_or_init(Instant::now);
}

/// Milliseconds since `boot()`. Marks the absolute position of an event on the
/// startup timeline — a duration alone cannot tell you *when* something happened.
pub fn since_boot_ms() -> u128 {
    BOOT.get_or_init(Instant::now).elapsed().as_millis()
}

/// Log a milestone with the milliseconds elapsed since `boot()`.
pub fn mark(label: &str) {
    log::info!("[perf] {:>6} ms  {}", since_boot_ms(), label);
}
