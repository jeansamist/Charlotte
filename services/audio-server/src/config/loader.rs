use super::Config;
use crate::errors::ConfigError;
use std::path::{Path, PathBuf};

/// Default locations, in order, used when no `--config` is given.
const SEARCH_PATHS: &[&str] = &[
    "/etc/charlotte/audio-server.toml",
    "config/default.toml",
    "services/audio-server/config/default.toml",
];

/// Loads and validates the configuration.
///
/// With `path = None`, the first existing file of [`SEARCH_PATHS`] is used,
/// or built-in defaults if none exists. Returns the config and the file used.
pub fn load(path: Option<&Path>) -> Result<(Config, Option<PathBuf>), ConfigError> {
    let file = match path {
        Some(p) => Some(p.to_path_buf()),
        None => SEARCH_PATHS.iter().map(PathBuf::from).find(|p| p.is_file()),
    };

    let cfg = match &file {
        Some(p) => {
            let text = std::fs::read_to_string(p).map_err(|source| ConfigError::Read {
                path: p.clone(),
                source,
            })?;
            toml::from_str::<Config>(&text).map_err(|source| ConfigError::Parse {
                path: p.clone(),
                source,
            })?
        }
        None => Config::default(),
    };

    cfg.validate()?;
    Ok((cfg, file))
}
