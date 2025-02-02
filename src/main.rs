mod config;
mod dependency_resolver;
mod typescript_resolver;

use crate::dependency_resolver::DependencyResolver;
use crate::typescript_resolver::TypeScriptResolver;
use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use config::Config;
use config::ProcessingOptions;
use ignore::Walk;
use regex::Regex;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Target files or directories to process
    #[arg(required = true)]
    targets: Vec<PathBuf>,

    /// Copy the combined code to clipboard
    #[arg(long)]
    copy: bool,

    /// Save the combined code to file
    #[arg(long)]
    save: bool,

    /// Output file path
    #[arg(long)]
    output_path: Option<String>,

    /// Ignore file path in .gitignore format
    #[arg(long)]
    ignore_file_path: Option<String>,

    /// Additional ignore patterns
    #[arg(long = "ignore", value_name = "PATTERN")]
    ignore_patterns: Vec<String>,

    /// Use relative paths
    #[arg(long, default_value_t = true)]
    relative: bool,

    /// Resolve dependencies
    #[arg(long, default_value_t = false)]
    deps: bool,
}

#[derive(Debug)]
enum AppError {
    IoError(io::Error),
    ConfigError(String),
    ClipboardError(String),
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::IoError(err)
    }
}

impl From<Box<dyn std::error::Error>> for AppError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        AppError::ClipboardError(err.to_string())
    }
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError::ConfigError(err)
    }
}

impl From<&str> for AppError {
    fn from(err: &str) -> Self {
        AppError::ConfigError(err.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::IoError(err) => write!(f, "IO error: {}", err),
            AppError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            AppError::ClipboardError(msg) => write!(f, "Clipboard error: {}", msg),
        }
    }
}

fn main() {
    let args = Args::parse();

    let target_paths = args.targets.clone();
    if target_paths.is_empty() {
        eprintln!("Error: No target files or directories specified.");
        std::process::exit(1);
    }

    match run(&target_paths, &args) {
        Ok(()) => println!("Project code combined successfully."),
        Err(err) => eprintln!("Error: {}", err),
    }
}

fn run(target_paths: &[PathBuf], args: &Args) -> Result<(), AppError> {
    let config = load_config()?;
    let options = ProcessingOptions::new(args, &config)?;

    let combined_source_code = process_files(target_paths, &options)?;

    execute_action(args, &config, combined_source_code)
}

fn load_config() -> io::Result<Config> {
    Config::load()
}

fn process_files(
    target_paths: &[PathBuf],
    options: &ProcessingOptions,
) -> Result<String, AppError> {
    let mut combined_source_code =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project>\n");

    let mut resolver = if options.deps {
        Some(DependencyResolver::new(&env::current_dir()?, true)?)
    } else {
        None
    };

    let mut ts_resolver = if options.deps {
        Some(TypeScriptResolver::new())
    } else {
        None
    };

    for target_path in target_paths {
        if target_path.is_file() {
            let files_to_process = if options.deps
                && TypeScriptResolver::is_supported_file(target_path)
                && resolver.is_some()
                && ts_resolver.is_some()
            {
                let resolved_files = resolver
                    .as_mut()
                    .unwrap()
                    .resolve_deps(target_path, ts_resolver.as_mut().unwrap())?;

                // Filter out ignored files from resolved dependencies
                resolved_files
                    .into_iter()
                    .filter(|path| !is_ignored(path, &options.ignore_patterns))
                    .collect()
            } else {
                vec![target_path.to_path_buf()]
            };

            for file_path in files_to_process {
                let file_source_code = process_single_file(&file_path, options)?;
                combined_source_code.push_str(&file_source_code);
            }
        } else if target_path.is_dir() {
            for entry in Walk::new(target_path).filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() && !is_ignored(path, &options.ignore_patterns) {
                    let file_source_code = process_single_file(path, options)?;
                    combined_source_code.push_str(&file_source_code);
                }
            }
        } else {
            eprintln!("Warning: Skipping invalid path: {}", target_path.display());
        }
    }

    combined_source_code.push_str("</project>\n");
    Ok(combined_source_code)
}

fn process_single_file(file_path: &Path, options: &ProcessingOptions) -> Result<String, AppError> {
    if is_ignored(file_path, &options.ignore_patterns) {
        return Ok(String::new());
    }

    let file_content = fs::read_to_string(file_path)?;
    let path_to_display = if options.use_relative_paths {
        match file_path.strip_prefix(env::current_dir()?) {
            Ok(relative) => relative.to_path_buf(),
            Err(_) => file_path.to_path_buf(),
        }
    } else {
        file_path.to_path_buf()
    };

    Ok(format_file_content(&path_to_display, &file_content))
}

fn format_file_content(file_path: &Path, file_content: &str) -> String {
    format!(
        "  <file name=\"{}\">\n{}\n  </file>\n",
        file_path.display(),
        file_content
            .lines()
            .map(|line| format!("    {}", line))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn execute_action(
    args: &Args,
    config: &Config,
    combined_source_code: String,
) -> Result<(), AppError> {
    if args.copy {
        copy_to_clipboard(combined_source_code)
    } else if args.save {
        let output_path = get_output_path(args, config)?;
        save_to_file(combined_source_code, &output_path)
    } else if let Some(action) = &config.default.action {
        match action.as_str() {
            "copy" => copy_to_clipboard(combined_source_code),
            "save" => {
                let output_path = get_output_path(args, config)?;
                save_to_file(combined_source_code, &output_path)
            }
            _ => {
                eprintln!("Unknown action: {}", action);
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("No action specified");
        std::process::exit(1);
    }
}

fn get_output_path(args: &Args, config: &Config) -> io::Result<PathBuf> {
    if let Some(path) = &args.output_path {
        return Ok(expand_tilde(path));
    }

    if let Some(path) = &config.default.output_path {
        return Ok(expand_tilde(path));
    }

    let current_dir = env::current_dir()?;

    if let Some(file_name) = &config.default.output_file_name {
        return Ok(current_dir.join(file_name));
    }

    Ok(current_dir.join("combined_code.txt"))
}

fn copy_to_clipboard(combined_code: String) -> Result<(), AppError> {
    let mut ctx: ClipboardContext = ClipboardProvider::new()?;
    ctx.set_contents(combined_code)?;
    println!("Combined code copied to clipboard.");
    Ok(())
}

fn save_to_file(combined_code: String, output_path: &Path) -> Result<(), AppError> {
    write_combined_code(output_path, &combined_code)?;
    println!("Combined code saved to file: {}", output_path.display());
    Ok(())
}

fn write_combined_code(
    output_file_path: &Path,
    combined_source_code: &str,
) -> Result<(), AppError> {
    fs::write(output_file_path, combined_source_code)?;
    Ok(())
}

fn is_ignored(file_path: &Path, ignore_patterns: &str) -> bool {
    let path_str = file_path.to_string_lossy();

    ignore_patterns
        .lines()
        .filter(|line| !line.trim().is_empty())
        .any(|pattern| {
            let regex_pattern = convert_ignore_pattern_to_regex(pattern);
            match Regex::new(&regex_pattern) {
                Ok(regex) => regex.is_match(&path_str),
                Err(_) => false,
            }
        })
}

fn convert_ignore_pattern_to_regex(pattern: &str) -> String {
    let mut regex_pattern = String::new();

    let mut in_bracket = false;
    for c in pattern.chars() {
        match c {
            '*' if !in_bracket => regex_pattern.push_str(".*"),
            '?' if !in_bracket => regex_pattern.push_str("."),
            '[' => {
                in_bracket = true;
                regex_pattern.push(c);
            }
            ']' => {
                in_bracket = false;
                regex_pattern.push(c);
            }
            '!' if in_bracket => regex_pattern.push('^'),
            '/' => regex_pattern.push_str("\\/"),
            '.' => regex_pattern.push_str("\\."),
            _ => regex_pattern.push(c),
        }
    }

    format!("^{}$", regex_pattern)
}

fn expand_tilde(path: &str) -> PathBuf {
    if !path.starts_with('~') {
        return PathBuf::from(path);
    }

    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("Failed to get home directory");

    let stripped_path = path.strip_prefix("~/").unwrap_or(path);
    let mut expanded_path = PathBuf::from(home_dir);
    expanded_path.push(stripped_path);
    expanded_path
}
