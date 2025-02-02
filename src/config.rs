use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Default {
    pub action: Option<String>,
    pub output_path: Option<String>,
    pub output_file_name: Option<String>,
    pub ignore_patterns: Option<Vec<String>>,
    pub use_relative_paths: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub default: Default,
}

impl Config {
    pub fn load() -> io::Result<Self> {
        let home_dir = env::var("HOME").unwrap_or_else(|_| env::var("USERPROFILE").unwrap());
        let config_path = PathBuf::from(home_dir).join(".pcc_config.toml");

        let config_str = match fs::read_to_string(&config_path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Config file not found at: {}", config_path.display()),
                ));
            }
            Err(err) => return Err(err),
        };

        toml::from_str(&config_str).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }
}
