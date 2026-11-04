//! Audio engine: buffers, mixing, limiting and the PipeWire streams.
//!
//! Pure DSP (no PipeWire): `buffer`, `mixer`, `limiter`, `engine`, `sim`.
//! PipeWire glue: `format`, `input`, `output`.

pub mod buffer;
pub mod engine;
pub mod format;
pub mod input;
pub mod limiter;
pub mod mixer;
pub mod output;
pub mod sim;
