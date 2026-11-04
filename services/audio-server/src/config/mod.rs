//! Configuration: TOML file → validated [`Config`].

mod loader;

pub use loader::load;

use crate::errors::ConfigError;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub server: ServerConfig,
    pub audio: AudioConfig,
    pub bluetooth: BluetoothConfig,
    pub output: OutputConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "charlotte-audio-server".into(),
            host: "0.0.0.0".into(),
            port: 8080,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub format: String,
    pub buffer_size: u32,
    pub input_latency_ms: u32,
    pub max_inputs: usize,
    pub master_volume: f32,
    pub limiter_enabled: bool,
    pub limiter_threshold: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
            format: "f32".into(),
            buffer_size: 512,
            input_latency_ms: 60,
            max_inputs: 16,
            master_volume: 0.8,
            limiter_enabled: true,
            limiter_threshold: 0.95,
        }
    }
}

impl AudioConfig {
    /// Number of interleaved samples buffered per input before it is mixed.
    pub fn prefill_samples(&self) -> usize {
        (self.sample_rate as usize * self.input_latency_ms as usize / 1000) * self.channels as usize
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BluetoothConfig {
    pub enabled: bool,
    /// Name other devices see when they scan (the BlueZ adapter alias).
    pub name: String,
    /// BlueZ adapter to use (`hciN`).
    pub adapter: String,
    pub auto_accept_trusted_devices: bool,
    pub auto_connect_sources: bool,
}

impl Default for BluetoothConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            name: "Charlotte".into(),
            adapter: "hci0".into(),
            auto_accept_trusted_devices: true,
            auto_connect_sources: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OutputConfig {
    pub device: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            device: "default".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingConfig {
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
        }
    }
}

impl Config {
    /// Checks every value and returns all problems at once.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let a = &self.audio;
        let mut errs = Vec::new();

        if a.sample_rate != 48_000 {
            errs.push(format!(
                "audio.sample_rate = {} is not supported yet (only 48000)",
                a.sample_rate
            ));
        }
        if a.channels != 2 {
            errs.push(format!(
                "audio.channels = {} is not supported yet (only 2)",
                a.channels
            ));
        }
        if a.format != "f32" {
            errs.push(format!(
                "audio.format = \"{}\" is not supported yet (only \"f32\")",
                a.format
            ));
        }
        if !(64..=8192).contains(&a.buffer_size) || !a.buffer_size.is_power_of_two() {
            errs.push(format!(
                "audio.buffer_size = {} must be a power of two between 64 and 8192",
                a.buffer_size
            ));
        }
        if !(5..=1000).contains(&a.input_latency_ms) {
            errs.push(format!(
                "audio.input_latency_ms = {} must be between 5 and 1000",
                a.input_latency_ms
            ));
        }
        if !(1..=64).contains(&a.max_inputs) {
            errs.push(format!(
                "audio.max_inputs = {} must be between 1 and 64",
                a.max_inputs
            ));
        }
        if !(0.0..=1.0).contains(&a.master_volume) {
            errs.push(format!(
                "audio.master_volume = {} must be between 0.0 and 1.0",
                a.master_volume
            ));
        }
        if !(a.limiter_threshold > 0.0 && a.limiter_threshold <= 1.0) {
            errs.push(format!(
                "audio.limiter_threshold = {} must be in (0.0, 1.0]",
                a.limiter_threshold
            ));
        }
        let bt = &self.bluetooth;
        if bt.name.trim().is_empty() || bt.name.len() > 248 {
            errs.push("bluetooth.name must be 1 to 248 bytes long".into());
        }
        if !bt.adapter.starts_with("hci") || bt.adapter[3..].parse::<u16>().is_err() {
            errs.push(format!("bluetooth.adapter = \"{}\" must look like \"hci0\"", bt.adapter));
        }
        if self.output.device.trim().is_empty() {
            errs.push("output.device must not be empty (use \"default\")".into());
        }
        if !["trace", "debug", "info", "warn", "error"].contains(&self.logging.level.as_str()) {
            errs.push(format!(
                "logging.level = \"{}\" must be one of trace, debug, info, warn, error",
                self.logging.level
            ));
        }

        if errs.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Invalid(errs.join("; ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        Config::default().validate().unwrap();
    }

    #[test]
    fn shipped_default_file_is_valid() {
        let text = include_str!("../../config/default.toml");
        let cfg: Config = toml::from_str(text).unwrap();
        cfg.validate().unwrap();
    }

    #[test]
    fn reports_all_errors() {
        let mut cfg = Config::default();
        cfg.audio.sample_rate = 44_100;
        cfg.audio.buffer_size = 500;
        let err = cfg.validate().unwrap_err().to_string();
        assert!(err.contains("sample_rate"));
        assert!(err.contains("buffer_size"));
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(toml::from_str::<Config>("[audio]\nsampel_rate = 48000\n").is_err());
    }

    #[test]
    fn prefill_is_interleaved_samples() {
        let a = AudioConfig::default(); // 60 ms @ 48 kHz stereo
        assert_eq!(a.prefill_samples(), 2880 * 2);
    }
}
