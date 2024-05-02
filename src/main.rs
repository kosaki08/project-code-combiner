use clipboard::{ClipboardContext, ClipboardProvider};
use ignore::Walk;
use regex::Regex;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Default {
    action: Option<String>,
    output_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Ignore {
    patterns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Config {
    default: Default,
    ignore: Ignore,
}

fn print_help() {
    println!("Usage: project_code_combinator [OPTIONS] <PROJECT_DIRECTORY>");
    println!();
    println!("Options:");
    println!("  --clipboard                 Copy the combined code to the clipboard");
    println!("  --save                      Save the combined code to a file");
    println!("  --output_path=<PATH>        Specify the output file path");
    println!("  --ignore_file_path=<PATH>   Specify the ignore file path in .gitignore format");
    println!("  --help                      Show this help message");
    println!("  --version                   Show version information");
}

fn print_version() {
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");
    println!("Project Code Combinator v{}", version);
}

fn run(project_dir: &Path, args: &[String]) -> io::Result<()> {
    let config = load_config()?;
    let ignore_patterns = get_ignore_patterns(&config.ignore.patterns)?;
    let combined_source_code = walk_and_combine(project_dir, &ignore_patterns)?;

    if let Some(action) = get_action(args, &config) {
        match action.as_str() {
            "copy" => {
                copy_to_clipboard(combined_source_code);
            }
            "save" => {
                let output_path = get_output_path(args, &config.default.output_path);
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
    if args.contains(&String::from("--clipboard")) {
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

    if !config_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Config file not found",
        ));
    }

    let config_str = fs::read_to_string(config_path)?;
    let config: Result<Config, _> = toml::from_str(&config_str);

    config.map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn get_ignore_patterns(ignore_patters_config: &Option<Vec<String>>) -> io::Result<String> {
    if let Some(patterns) = ignore_patters_config {
        if !patterns.is_empty() {
            return Ok(patterns.join("\n"));
        }
    }

    Ok(String::new())
}

fn walk_and_combine(project_dir: &Path, ignore_patterns: &str) -> io::Result<String> {
    let mut combined_source_code = String::new();

    for result in Walk::new(project_dir).filter_map(|r| r.ok()) {
        let path = result.path();
        if path.is_file() && !is_ignored(path, project_dir, ignore_patterns) {
            let file_content = fs::read_to_string(path)?;
            let relative_path = path.strip_prefix(project_dir).unwrap();
            combined_source_code.push_str(&format!(
                "{}\n{}\n{}\n",
                "*".repeat(30),
                relative_path.display(),
                "*".repeat(30)
            ));
            combined_source_code.push_str(&file_content);
            combined_source_code.push('\n');
        }
    }

    Ok(combined_source_code)
}

fn write_combined_code(output_file: &Path, combined_source_code: &str) -> io::Result<()> {
    let mut output_file = fs::File::create(output_file)?;
    output_file.write_all(combined_source_code.as_bytes())?;

    Ok(())
}

fn is_ignored(file_path: &Path, project_dir: &Path, ignore_patterns: &str) -> bool {
    //.ignoreファイルで指定されているパターンに一致するかどうかを判断
    let relative_path = file_path.strip_prefix(project_dir).unwrap();
    let relative_path_str = relative_path.to_str().unwrap();

    ignore_patterns
        .lines()
        .filter(|line| !line.trim().is_empty())
        .any(|pattern| {
            // 正規表現を使用してパターンを解析
            let regex_pattern = convert_ignore_pattern_to_regex(pattern);
            let regex = Regex::new(&regex_pattern).unwrap();
            regex.is_match(relative_path_str)
        })
}

fn convert_ignore_pattern_to_regex(pattern: &str) -> String {
    let mut regex_pattern = pattern
        .replace(".", r"\.")
        .replace("*", ".*")
        .replace("/", r"\/")
        .replace("?", ".");

    // ディレクトリを無視するパターンの処理
    if regex_pattern.ends_with("/") {
        regex_pattern.push_str(".*");
    }

    // パターンがスラッシュ（`/`）で始まっていない場合、先頭に`.*`を追加
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

fn get_output_path(args: &[String], default_output_path: &Option<String>) -> PathBuf {
    if let Some(path) = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--output_path="))
    {
        expand_tilde(path)
    } else if let Some(path) = default_output_path {
        expand_tilde(path)
    } else {
        let current_dir = env::current_dir().expect("Failed to get current directory");
        current_dir.join("combined_code.txt")
    }
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

    // コマンドライン引数から対象のディレクトリを取得
    let project_directory = match env::args().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => {
            eprintln!("Error: Project directory not specified.");
            print_help();
            std::process::exit(1);
        }
    };

    match run(&project_directory, &args) {
        Ok(_) => println!("Source code combined successfully."),
        Err(err) => eprintln!("Error: {}", err),
    }
}
