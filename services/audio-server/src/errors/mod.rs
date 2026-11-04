//! Error types shared by the audio-server modules.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid TOML in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("all {0} mixer input slots are in use")]
    NoFreeSlot(usize),
    #[error("mixer command queue is full, try again")]
    QueueFull,
    #[error("unknown mixer slot {0}")]
    UnknownSlot(usize),
}
