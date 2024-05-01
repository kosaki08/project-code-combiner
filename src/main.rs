use clipboard::{ClipboardContext, ClipboardProvider};
use ignore::Walk;
use regex::Regex;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const PCC_IGNORE_FILE: &str = ".pcc_ignore";

fn run(project_dir: &Path, args: &[String]) -> io::Result<()> {
    let ignore_patterns = get_ignore_patterns(project_dir, args)?;
    let combined_source_code = walk_and_combine(project_dir, &ignore_patterns)?;

    // クリップボードにコピーするかどうかを判断
    if args.contains(&String::from("--clipboard")) {
        copy_to_clipboard(combined_source_code);
    } else {
        let output_path = get_output_path(args);
        save_to_file(combined_source_code, &output_path);
    }

    Ok(())
}

fn get_ignore_patterns(project_dir: &Path, args: &[String]) -> io::Result<String> {
    let ignore_file_path = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--ignore_file_path=").map(expand_tilde));

    if let Some(path) = ignore_file_path {
        return fs::read_to_string(path);
    }

    let default_ignore_path = project_dir.join(PCC_IGNORE_FILE);
    if default_ignore_path.exists() {
        return fs::read_to_string(default_ignore_path);
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

fn get_output_path(args: &[String]) -> PathBuf {
    args.iter()
        .find_map(|arg| arg.strip_prefix("--output_path=").map(PathBuf::from))
        .unwrap_or_else(|| {
            let current_dir = env::current_dir().expect("Failed to get current directory");
            current_dir.join("combined_code.txt")
        })
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(home_dir) = path.strip_prefix("~").and_then(|_| {
        env::var("HOME")
            .ok()
            .or_else(|| env::var("USERPROFILE").ok())
    }) {
        let path = Path::new(path);
        if let Ok(stripped_path) = path.strip_prefix("~/") {
            let mut expanded_path = PathBuf::from(home_dir);
            expanded_path.push(stripped_path);
            expanded_path
        } else {
            PathBuf::from(path)
        }
    } else {
        PathBuf::from(path)
    }
}

fn main() {
    // コマンドライン引数からプロジェクトディレクトリを取得
    let project_directory = match env::args().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => {
            eprintln!("Usage: {} <project_directory>", env::args().next().unwrap());
            std::process::exit(1);
        }
    };

    let args: Vec<String> = env::args().collect();
    match run(&project_directory, &args) {
        Ok(_) => println!("Source code combined successfully."),
        Err(err) => eprintln!("Error: {}", err),
    }
}
