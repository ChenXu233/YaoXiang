//! Time compatibility for wasm and native targets.
//!
//! On native: re-exports `std::time::Instant`
//! On wasm: re-exports `web_time::Instant` (uses performance.now())

#[cfg(target_arch = "wasm32")]
pub type Instant = web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
pub type Instant = std::time::Instant;
