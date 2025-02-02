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

fn get_ignore_patterns(ignore_patterns_config: &Option<Vec<String>>) -> io::Result<String> {
    if let Some(patterns) = ignore_patterns_config {
        if !patterns.is_empty() {
            return Ok(patterns.join("\n"));
        }
    }
    Ok(String::new())
}

impl ProcessingOptions {
    pub fn new(args: &Args, config: &Config) -> io::Result<Self> {
        let mut ignore_patterns = get_ignore_patterns(&config.default.ignore_patterns)?;
        let additional_ignore_patterns = &args.ignore_patterns;

        if !additional_ignore_patterns.is_empty() {
            if !ignore_patterns.is_empty() {
                ignore_patterns.push('\n');
            }
            ignore_patterns.push_str(&additional_ignore_patterns.join("\n"));
        }

        Ok(ProcessingOptions {
            ignore_patterns,
            use_relative_paths: args.relative,
            deps: args.deps || config.default.deps.unwrap_or(false),
        })
    }
}
