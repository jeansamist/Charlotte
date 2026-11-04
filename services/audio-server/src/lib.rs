//! Charlotte audio-server: headless multi-source Bluetooth audio mixer.
//!
//! Layers (strictly separated):
//! - [`config`]: TOML configuration and validation.
//! - [`audio`]: real-time engine (buffers, mixer, limiter) + PipeWire streams.
//! - [`devices`]: PipeWire node discovery and classification.
//! - [`monitoring`]: CPU / memory sampling.
//! - [`app`]: wires everything to the PipeWire main loop.
//! - [`api`]: local HTTP API (axum) used by the Charlotte gateway; [`shared`] holds its view of the mixer.
//! - [`bluetooth`]: adapter identity (name, class) over BlueZ D-Bus.
//!
//! Pairing is handled by BlueZ (see `scripts/bt-sink.sh`). The full D-Bus
//! Bluetooth manager (pairing agent, trust list, reconnects) comes in milestone 4.

pub mod api;
pub mod app;
pub mod audio;
pub mod bluetooth;
pub mod config;
pub mod devices;
pub mod errors;
pub mod monitoring;
pub mod outputs;
pub mod shared;
