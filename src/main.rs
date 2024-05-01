use ignore::Walk;
use regex::Regex;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn combine_source_code(project_dir: &Path, output_file: &Path) -> io::Result<()> {
    // .gitignoreファイルを読み込み、無視するパターンを取得
    let gitignore_path = project_dir.join(".pc_ignore");
    let gitignore_patterns = fs::read_to_string(gitignore_path)?;

    // 結合したソースコードを格納する変数
    let mut combined_source_code = String::new();

    // プロジェクトのディレクトリを再帰的に走査
    for result in Walk::new(project_dir) {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    // .gitignoreで指定されているパターンに一致しないファイルを処理
                    if !is_ignored(path, project_dir, &gitignore_patterns) {
                        // ファイルの内容を読み込んで結合
                        let file_content = fs::read_to_string(path)?;
                        combined_source_code.push_str(&file_content);
                        combined_source_code.push('\n');
                    }
                }
            }
            Err(err) => println!("Error: {}", err),
        }
    }

    // 結合したソースコードを新しいファイルに書き込む
    let mut output_file = fs::File::create(output_file)?;
    output_file.write_all(combined_source_code.as_bytes())?;

    Ok(())
}

fn is_ignored(file_path: &Path, project_dir: &Path, gitignore_patterns: &str) -> bool {
    // .gitignoreで指定されているパターンに一致するかどうかを判断
    let relative_path = file_path.strip_prefix(project_dir).unwrap();
    let relative_path_str = relative_path.to_str().unwrap();

    gitignore_patterns
        .lines()
        .filter(|line| !line.trim().is_empty())
        .any(|pattern| {
            // 正規表現を使用してパターンを解析
            let regex_pattern = convert_gitignore_pattern_to_regex(pattern);
            let regex = Regex::new(&regex_pattern).unwrap();
            regex.is_match(relative_path_str)
        })
}

fn convert_gitignore_pattern_to_regex(pattern: &str) -> String {
    // .gitignoreのパターンを正規表現に変換
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

fn main() {
    // コマンドライン引数からプロジェクトディレクトリを取得
    let project_directory = match env::args().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => {
            eprintln!("Usage: {} <project_directory>", env::args().next().unwrap());
            std::process::exit(1);
        }
    };
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let output_file = current_dir.join("combined_code.txt");
    combine_source_code(&project_directory, &output_file).expect("Failed to combine source code");
}
