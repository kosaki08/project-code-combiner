use clipboard::{ClipboardContext, ClipboardProvider};
use ignore::Walk;
use regex::Regex;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Default {
    action: Option<String>,
    output_path: Option<String>,
    output_file_name: Option<String>,
    ignore_patterns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Config {
    default: Default,
}

fn print_help() {
    println!("Usage: project_code_combiner [OPTIONS] <PROJECT_DIRECTORY>");
    println!();
    println!("Options:");
    println!("  --copy                      Copy the combined code to the clipboard");
    println!("  --save                      Save the combined code to a file");
    println!("  --output_path=<PATH>        Specify the output file path");
    println!("  --ignore_file_path=<PATH>   Specify the ignore file path in .gitignore format");
    println!("  --help                      Show this help message");
    println!("  --version                   Show version information");
}

fn print_version() {
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");
    println!("Project Code Combiner v{}", version);
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

fn process_file(
    file_path: &Path,
    ignore_patterns: &str,
    left_sep: &str,
    right_sep: &str,
) -> io::Result<String> {
    if is_ignored(file_path, file_path.parent().unwrap(), ignore_patterns) {
        return Ok(String::new());
    }

    let file_content = fs::read_to_string(file_path)?;
    let formatted_content = format_file_content(file_path, &file_content, left_sep, right_sep);
    Ok(formatted_content)
}

fn run(target_paths: &[PathBuf], args: Vec<String>) -> io::Result<()> {
    let config = load_config()?;
    let ignore_patterns = get_ignore_patterns(&config.default.ignore_patterns)?;
    let default_sep = "-".repeat(30);
    let left_sep = args
        .iter()
        .find(|arg| arg.starts_with("--left_sep="))
        .and_then(|arg| arg.strip_prefix("--left_sep="))
        .unwrap_or(default_sep.as_str());

    let right_sep = args
        .iter()
        .find(|arg| arg.starts_with("--right_sep="))
        .and_then(|arg| arg.strip_prefix("--right_sep="))
        .unwrap_or(&default_sep.as_str());

    let mut combined_source_code = String::new();

    for target_path in target_paths {
        if target_path.is_file() {
            let file_source_code =
                process_file(target_path, &ignore_patterns, &left_sep, &right_sep)?;
            combined_source_code.push_str(&file_source_code);
        } else if target_path.is_dir() {
            let dir_source_code =
                walk_and_combine(target_path, &ignore_patterns, &left_sep, &right_sep)?;
            combined_source_code.push_str(&dir_source_code);
        } else {
            eprintln!("Warning: Skipping invalid path: {}", target_path.display());
        }
    }

    if let Some(action) = get_action(&args, &config) {
        match action.as_str() {
            "copy" => {
                copy_to_clipboard(combined_source_code);
            }
            "save" => {
                let output_path = get_output_path(&args, &config);
                save_to_file(combined_source_code, &output_path);
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

    Ok(())
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

fn load_config() -> io::Result<Config> {
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

    let config: Result<Config, _> = toml::from_str(&config_str);
    config.map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn get_ignore_patterns(ignore_patterns_config: &Option<Vec<String>>) -> io::Result<String> {
    if let Some(patterns) = ignore_patterns_config {
        if !patterns.is_empty() {
            return Ok(patterns.join("\n"));
        }
    }

    Ok(String::new())
}

fn walk_and_combine(
    project_dir: &Path,
    ignore_patterns: &str,
    left_sep: &str,
    right_sep: &str,
) -> io::Result<String> {
    let mut combined_source_code = String::new();

    for result in Walk::new(project_dir).filter_map(|r| r.ok()) {
        let path = result.path();
        if path.is_file() && !is_ignored(path, project_dir, ignore_patterns) {
            let file_content = fs::read_to_string(path)?;
            let relative_path = path.strip_prefix(project_dir).unwrap();
            let formatted_content =
                format_file_content(relative_path, &file_content, left_sep, right_sep);
            combined_source_code.push_str(&formatted_content);
        }
    }

    Ok(combined_source_code)
}

fn write_combined_code(output_file_path: &Path, combined_source_code: &str) -> io::Result<()> {
    fs::write(output_file_path, combined_source_code)
}

fn is_ignored(file_path: &Path, project_dir: &Path, ignore_patterns: &str) -> bool {
    let relative_path = file_path.strip_prefix(project_dir).unwrap();
    let relative_path_str = relative_path.to_str().unwrap();

    // Determine if it is included in the ignore_patterns specified in the configuration file
    ignore_patterns
        .lines()
        .filter(|line| !line.trim().is_empty())
        .any(|pattern| {
            // Analyze patterns using regular expressions
            let regex_pattern = convert_ignore_pattern_to_regex(pattern);
            let regex = Regex::new(&regex_pattern).unwrap();
            regex.is_match(relative_path_str)
        })
}

fn convert_ignore_pattern_to_regex(pattern: &str) -> String {
    let escaped_pattern = regex::escape(pattern);
    let mut regex_pattern = escaped_pattern.replace("\\*", ".*").replace("\\?", ".");

    // Handling of patterns that ignore directories
    if regex_pattern.ends_with("/") {
        regex_pattern.push_str(".*");
    }

    // If the pattern does not begin with a slash (`/`), prefix it with `. *` at the beginning
    if !regex_pattern.starts_with("/") {
        regex_pattern.insert_str(0, ".*");
    }

    format!("{}", regex_pattern)
}

fn copy_to_clipboard(combined_code: String) {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    ctx.set_contents(combined_code)
        .expect("Failed to copy to clipboard");
    println!("Combined code copied to clipboard.");
}

fn save_to_file(combined_code: String, output_path: &Path) {
    write_combined_code(output_path, &combined_code)
        .expect("Failed to write combined code to file");
    println!("Combined code saved to file: {}", output_path.display());
}

fn get_output_path(args: &[String], config: &Config) -> PathBuf {
    let default_output_path = &config.default.output_path;

    if let Some(path) = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--output_path="))
        .or_else(|| default_output_path.as_ref().map(|x| x.as_str()))
    {
        return expand_tilde(path);
    }

    let current_dir = env::current_dir().expect("Failed to get current directory");

    if let Some(file_name) = &config.default.output_file_name {
        return current_dir.join(file_name);
    }

    current_dir.join("combined_code.txt")
}

fn expand_tilde(path: &str) -> PathBuf {
    if !path.starts_with('~') {
        return PathBuf::from(path);
    }

    let home_dir = env::var("HOME")
        .ok()
        .or_else(|| env::var("USERPROFILE").ok())
        .expect("Failed to get home directory");

    if let Some(stripped_path) = path.strip_prefix("~/") {
        let mut expanded_path = PathBuf::from(home_dir);
        expanded_path.push(stripped_path);
        return expanded_path;
    }

    PathBuf::from(path)
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

    match run(&target_paths, args) {
        Ok(_) => println!("Project code combined successfully."),
        Err(err) => eprintln!("Error: {}", err),
    }
}
