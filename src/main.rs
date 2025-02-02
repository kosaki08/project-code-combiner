mod config;
use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use config::Config;
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
    let config = load_config().map_err(|e| AppError::ConfigError(e.to_string()))?;
    let (ignore_patterns, left_sep, right_sep, use_relative_paths) =
        get_config_settings(args, &config)?;

    let combined_source_code = process_files(
        target_paths,
        &ignore_patterns,
        &left_sep,
        &right_sep,
        use_relative_paths,
    )?;

    execute_action(args, &config, combined_source_code)
}

fn load_config() -> io::Result<Config> {
    Config::load()
}

fn get_config_settings(args: &Args, config: &Config) -> io::Result<(String, String, String, bool)> {
    let mut ignore_patterns = get_ignore_patterns(&config.default.ignore_patterns)?;
    let additional_ignore_patterns = &args.ignore_patterns;

    if !additional_ignore_patterns.is_empty() {
        if !ignore_patterns.is_empty() {
            ignore_patterns.push('\n');
        }
        ignore_patterns.push_str(&additional_ignore_patterns.join("\n"));
    }

    let default_sep = "-".repeat(30);
    let left_sep = &default_sep;
    let right_sep = &default_sep;
    let use_relative_paths = args.relative;

    Ok((
        ignore_patterns,
        left_sep.to_string(),
        right_sep.to_string(),
        use_relative_paths,
    ))
}

fn get_ignore_patterns(ignore_patterns_config: &Option<Vec<String>>) -> io::Result<String> {
    if let Some(patterns) = ignore_patterns_config {
        if !patterns.is_empty() {
            return Ok(patterns.join("\n"));
        }
    }

    Ok(String::new())
}

fn process_files(
    target_paths: &[PathBuf],
    ignore_patterns: &str,
    left_sep: &str,
    right_sep: &str,
    use_relative_paths: bool,
) -> Result<String, AppError> {
    let mut combined_source_code = String::new();

    for target_path in target_paths {
        if target_path.is_file() {
            let file_source_code = process_single_file(
                target_path,
                ignore_patterns,
                left_sep,
                right_sep,
                use_relative_paths,
            )?;
            combined_source_code.push_str(&file_source_code);
        } else if target_path.is_dir() {
            for entry in Walk::new(target_path).filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() && !is_ignored(path, target_path, ignore_patterns) {
                    let file_source_code = process_single_file(
                        path,
                        ignore_patterns,
                        left_sep,
                        right_sep,
                        use_relative_paths,
                    )?;
                    combined_source_code.push_str(&file_source_code);
                }
            }
        } else {
            eprintln!("Warning: Skipping invalid path: {}", target_path.display());
        }
    }

    Ok(combined_source_code)
}

fn process_single_file(
    file_path: &Path,
    ignore_patterns: &str,
    left_sep: &str,
    right_sep: &str,
    use_relative_paths: bool,
) -> Result<String, AppError> {
    if is_ignored(file_path, file_path.parent().unwrap(), ignore_patterns) {
        return Ok(String::new());
    }

    let file_content = fs::read_to_string(file_path)?;
    let formatted_content = if use_relative_paths {
        let relative_path = file_path.strip_prefix(file_path.parent().unwrap()).unwrap();
        format_file_content(relative_path, &file_content, left_sep, right_sep)
    } else {
        format_file_content(file_path, &file_content, left_sep, right_sep)
    };
    Ok(formatted_content)
}

fn format_file_content(
    file_path: &Path,
    file_content: &str,
    left_sep: &str,
    right_sep: &str,
) -> String {
    format!(
        "{}\n{}\n{}\n{}\n",
        left_sep,
        file_path.display(),
        right_sep,
        file_content
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
    let mut ctx: ClipboardContext = ClipboardProvider::new().map_err(|err| {
        AppError::ClipboardError(format!("Failed to create clipboard context: {}", err))
    })?;

    ctx.set_contents(combined_code)
        .map_err(|err| AppError::ClipboardError(format!("Failed to copy to clipboard: {}", err)))?;

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

fn is_ignored(file_path: &Path, project_dir: &Path, ignore_patterns: &str) -> bool {
    let relative_path = match file_path.strip_prefix(project_dir) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let relative_path_str = relative_path.to_str().unwrap();

    ignore_patterns
        .lines()
        .filter(|line| !line.trim().is_empty())
        .any(|pattern| {
            let regex_pattern = convert_ignore_pattern_to_regex(pattern);
            let regex = Regex::new(&regex_pattern).unwrap();
            regex.is_match(relative_path_str)
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
