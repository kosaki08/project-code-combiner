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
        save_to_file(combined_source_code);
    }

    Ok(())
}

fn get_ignore_patterns(project_dir: &Path, args: &[String]) -> io::Result<String> {
    let ignore_file_path = args.iter().find_map(|arg| {
        if arg.starts_with("--ignore_file_path=") {
            Some(PathBuf::from(
                arg.strip_prefix("--ignore_file_path=").unwrap(),
            ))
        } else {
            None
        }
    });

    let ignore_path = if let Some(path) = ignore_file_path {
        path
    } else if project_dir.join(PCC_IGNORE_FILE).exists() {
        project_dir.join(PCC_IGNORE_FILE)
    } else {
        return Ok(String::new()); // デフォルトの無視ファイルがない場合は空の文字列を返す
    };
    fs::read_to_string(ignore_path)
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

fn save_to_file(combined_code: String) {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let output_file: PathBuf = current_dir.join("combined_code.txt");
    write_combined_code(&output_file, &combined_code)
        .expect("Failed to write combined code to file");
    println!("Combined code saved to file: {}", output_file.display());
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
