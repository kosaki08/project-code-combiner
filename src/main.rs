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
    use_relative_paths: Option<bool>,
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
    use_relative_paths: bool,
) -> io::Result<String> {
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

fn process_files_common(
    target_path: &Path,
    ignore_patterns: &str,
    left_sep: &str,
    right_sep: &str,
    use_relative_paths: bool,
) -> io::Result<String> {
    let mut combined_source_code = String::new();

    if target_path.is_file() {
        let file_source_code = process_file(
            target_path,
            ignore_patterns,
            left_sep,
            right_sep,
            use_relative_paths,
        )?;
        combined_source_code.push_str(&file_source_code);
    } else if target_path.is_dir() {
        for result in Walk::new(target_path).filter_map(|r| r.ok()) {
            let path = result.path();
            if path.is_file() && !is_ignored(path, target_path, ignore_patterns) {
                if use_relative_paths {
                    path.strip_prefix(target_path).unwrap()
                } else {
                    path
                };
                let file_source_code = process_file(
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

    Ok(combined_source_code)
}

fn process_files(
    target_paths: &[PathBuf],
    ignore_patterns: &str,
    left_sep: &str,
    right_sep: &str,
    use_relative_paths: bool,
) -> io::Result<String> {
    let mut combined_source_code = String::new();

    for target_path in target_paths {
        let dir_source_code = process_files_common(
            target_path,
            ignore_patterns,
            left_sep,
            right_sep,
            use_relative_paths,
        )?;
        combined_source_code.push_str(&dir_source_code);
    }

    Ok(combined_source_code)
}

fn get_config_settings(args: &[String], config: &Config) -> (String, String, String, bool) {
    let ignore_patterns = get_ignore_patterns(&config.default.ignore_patterns).unwrap_or_default();
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
    let use_relative_paths = get_use_relative_paths(args, config);

    (
        ignore_patterns,
        left_sep.to_string(),
        right_sep.to_string(),
        use_relative_paths,
    )
}

fn execute_action(
    args: &[String],
    config: &Config,
    combined_source_code: String,
) -> io::Result<()> {
    if let Some(action) = get_action(args, config) {
        match action.as_str() {
            "copy" => copy_to_clipboard(combined_source_code),
            "save" => {
                let output_path = get_output_path(args, config);
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

fn run(target_paths: &[PathBuf], args: &[String]) -> io::Result<()> {
    let config = load_config()?;
    let (ignore_patterns, left_sep, right_sep, use_relative_paths) =
        get_config_settings(args, &config);

    let combined_source_code = process_files(
        target_paths,
        &ignore_patterns,
        &left_sep,
        &right_sep,
        use_relative_paths,
    )?;

    execute_action(args, &config, combined_source_code)
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

fn write_combined_code(output_file_path: &Path, combined_source_code: &str) -> io::Result<()> {
    fs::write(output_file_path, combined_source_code)
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
    let escaped_pattern = regex::escape(pattern);
    let regex_pattern = format!(
        "^{}$",
        escaped_pattern
            .replace("\\*\\*", ".*")
            .replace("\\*", "[^/]*")
            .replace("\\?", "[^/]")
            .replace("/", "/.*")
    );

    regex_pattern
}

fn copy_to_clipboard(combined_code: String) -> io::Result<()> {
    let mut ctx: ClipboardContext = ClipboardProvider::new().map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to create clipboard context: {}", err),
        )
    })?;

    ctx.set_contents(combined_code).map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to copy to clipboard: {}", err),
        )
    })?;

    println!("Combined code copied to clipboard.");
    Ok(())
}

fn save_to_file(combined_code: String, output_path: &Path) -> io::Result<()> {
    write_combined_code(output_path, &combined_code)?;
    println!("Combined code saved to file: {}", output_path.display());
    Ok(())
}

fn get_output_path(args: &[String], config: &Config) -> PathBuf {
    let default_output_path = &config.default.output_path;

    if let Some(path) = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--output_path="))
        .or_else(|| default_output_path.as_deref())
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
        .or_else(|_| env::var("USERPROFILE"))
        .expect("Failed to get home directory");

    let stripped_path = path.strip_prefix("~/").unwrap_or(path);
    let mut expanded_path = PathBuf::from(home_dir);
    expanded_path.push(stripped_path);
    expanded_path
}

fn get_use_relative_paths(args: &[String], config: &Config) -> bool {
    if args.contains(&String::from("--relative")) {
        return true;
    }

    if args.contains(&String::from("--no-relative")) {
        return false;
    }

    config.default.use_relative_paths.unwrap_or(true)
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
