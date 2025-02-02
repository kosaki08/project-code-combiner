mod config;
use clipboard::{ClipboardContext, ClipboardProvider};
use config::Config;
use ignore::Walk;
use regex::Regex;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
    let args: Vec<String> = env::args().collect();

    if args.contains(&String::from("--help")) {
        print_help();
        std::process::exit(0);
    }

    if args.contains(&String::from("--version")) {
        print_version();
        std::process::exit(0);
    }

    let target_paths: Vec<PathBuf> = env::args().skip(1).map(PathBuf::from).collect();

    if target_paths.is_empty() {
        eprintln!("Error: No target files or directories specified.");
        print_help();
        std::process::exit(1);
    }

    match run(&target_paths, &args) {
        Ok(()) => println!("Project code combined successfully."),
        Err(err) => eprintln!("Error: {}", err),
    }
}

fn print_help() {
    println!("Usage: project_code_combiner [OPTIONS] <PROJECT_DIRECTORY>");
    println!();
    println!("Options:");
    println!("  --copy                      Copy the combined code to the clipboard");
    println!("  --save                      Save the combined code to a file");
    println!("  --output_path=<PATH>        Specify the output file path");
    println!("  --ignore_file_path=<PATH>   Specify the ignore file path in .gitignore format");
    println!("  --ignore=<PATTERN>          Add an additional ignore pattern (can be used multiple times)");
    println!("  --help                      Show this help message");
    println!("  --version                   Show version information");
}

fn print_version() {
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");
    println!("Project Code Combiner v{}", version);
}

fn run(target_paths: &[PathBuf], args: &[String]) -> Result<(), AppError> {
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

fn get_config_settings(
    args: &[String],
    config: &Config,
) -> io::Result<(String, String, String, bool)> {
    let mut ignore_patterns = get_ignore_patterns(&config.default.ignore_patterns)?;
    let additional_ignore_patterns = parse_additional_ignore_patterns(args);

    if !additional_ignore_patterns.is_empty() {
        if !ignore_patterns.is_empty() {
            ignore_patterns.push('\n');
        }
        ignore_patterns.push_str(&additional_ignore_patterns.join("\n"));
    }

    let default_sep = "-".repeat(30);
    let left_sep = args
        .iter()
        .find(|arg| arg.starts_with("--left_sep="))
        .and_then(|arg| arg.strip_prefix("--left_sep="))
        .unwrap_or(&default_sep);
    let right_sep = args
        .iter()
        .find(|arg| arg.starts_with("--right_sep="))
        .and_then(|arg| arg.strip_prefix("--right_sep="))
        .unwrap_or(&default_sep);
    let use_relative_paths = get_use_relative_paths(args, config)?;

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

fn parse_additional_ignore_patterns(args: &[String]) -> Vec<String> {
    args.iter()
        .filter_map(|arg| {
            if arg.starts_with("--ignore=") {
                Some(arg.strip_prefix("--ignore=").unwrap().to_string())
            } else {
                None
            }
        })
        .collect()
}

fn get_action(args: &[String], config: &Config) -> Option<String> {
    if args.contains(&String::from("--copy")) {
        return Some("copy".to_string());
    }

    if args.contains(&String::from("--save")) {
        return Some("save".to_string());
    }

    config.default.action.clone()
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
    args: &[String],
    config: &Config,
    combined_source_code: String,
) -> Result<(), AppError> {
    if let Some(action) = get_action(args, config) {
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

fn get_output_path(args: &[String], config: &Config) -> io::Result<PathBuf> {
    let default_output_path = &config.default.output_path;

    if let Some(path) = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--output_path="))
        .or_else(|| default_output_path.as_deref())
    {
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

fn get_use_relative_paths(args: &[String], config: &Config) -> io::Result<bool> {
    if args.contains(&String::from("--relative")) {
        return Ok(true);
    }

    if args.contains(&String::from("--no-relative")) {
        return Ok(false);
    }

    Ok(config.default.use_relative_paths.unwrap_or(true))
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
