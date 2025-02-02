use crate::Args;
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
    pub deps: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub default: Default,
}

#[derive(Debug)]
pub struct ProcessingOptions {
    pub ignore_patterns: String,
    pub use_relative_paths: bool,
    pub deps: bool,
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

fn convert_ignore_patterns(patterns: &[String]) -> String {
    patterns
        .iter()
        .map(|pattern| {
            if pattern.ends_with('/') {
                format!("{}**/*", pattern)
            } else {
                pattern.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl ProcessingOptions {
    pub fn new(args: &Args, config: &Config) -> io::Result<Self> {
        let mut patterns = Vec::new();

        // First, apply patterns from the config file
        if let Some(config_patterns) = &config.default.ignore_patterns {
            patterns.extend(config_patterns.clone());
        }

        // Command line patterns can override config file patterns
        patterns.extend(args.ignore_patterns.clone());

        // Convert patterns to proper ignore format
        let ignore_patterns = convert_ignore_patterns(&patterns);

        Ok(ProcessingOptions {
            ignore_patterns,
            use_relative_paths: args.relative,
            deps: args.deps || config.default.deps.unwrap_or(false),
        })
    }
}
