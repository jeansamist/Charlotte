//! Charlotte audio-server: headless multi-source Bluetooth audio mixer.
//!
//! Layers (strictly separated):
//! - [`config`]: TOML configuration and validation.
//! - [`audio`]: real-time engine (buffers, mixer, limiter) + PipeWire streams.
//! - [`devices`]: PipeWire node discovery and classification.
//! - [`monitoring`]: CPU / memory sampling.
//! - [`app`]: wires everything to the PipeWire main loop.
//!
//! Bluetooth pairing itself is handled by BlueZ (see `scripts/bt-sink.sh`);
//! the D-Bus based Bluetooth manager comes in a later milestone.

pub mod app;
pub mod audio;
pub mod config;
pub mod devices;
pub mod errors;
pub mod monitoring;
