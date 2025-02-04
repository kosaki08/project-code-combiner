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
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Target files or directories to process
    #[arg(required = false)]
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

    /// Target files to be modified
    #[arg(long = "target")]
    target_files: Vec<PathBuf>,

    /// Reference files for context
    #[arg(long = "reference")]
    reference_files: Vec<PathBuf>,
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

// Add new struct to track processed files and dependencies
struct FileProcessor {
    processed_files: HashSet<PathBuf>,
    dependency_map: HashMap<PathBuf, HashSet<PathBuf>>,
    combined_source_code: String,
}

impl FileProcessor {
    fn new() -> Self {
        Self {
            processed_files: HashSet::new(),
            dependency_map: HashMap::new(),
            combined_source_code: String::from(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project>\n",
            ),
        }
    }

    // Process a single file and its dependencies
    fn process_file_with_deps(
        &mut self,
        file_path: &Path,
        options: &ProcessingOptions,
        deps_resolver: &mut DependencyResolver,
        ts_resolver: &mut TypeScriptResolver,
    ) -> Result<(), AppError> {
        // Skip if already processed
        if self.processed_files.contains(file_path) {
            return Ok(());
        }

        // Process main file
        let file_source_code = process_single_file(file_path, options)?;
        self.combined_source_code.push_str(&file_source_code);
        self.processed_files.insert(file_path.to_path_buf());

        // Process dependencies
        let resolved_files = deps_resolver.resolve_deps(file_path, ts_resolver)?;

        for dep_file in resolved_files {
            if !is_ignored(&dep_file, &options.ignore_patterns) && &dep_file != file_path {
                let all_importers = deps_resolver.get_all_importers(&dep_file);
                self.dependency_map.insert(dep_file, all_importers);
            }
        }

        Ok(())
    }

    // Add dependencies section to output
    fn add_dependencies_section(&mut self, options: &ProcessingOptions) -> Result<(), AppError> {
        if !self.dependency_map.is_empty() {
            self.combined_source_code.push_str("  <dependencies>\n");

            // Sort dependencies to ensure consistent output
            let mut deps: Vec<_> = self.dependency_map.iter().collect();
            deps.sort_by(|a, b| a.0.cmp(b.0));

            for (dep_file, importers) in deps {
                // Skip if already processed in main section
                if !self.processed_files.contains(dep_file) {
                    let mut file_source_code =
                        process_single_file_with_importers(dep_file, options, importers)?;
                    // Add additional indentation for dependencies section
                    file_source_code = file_source_code
                        .lines()
                        .map(|line| format!("  {}", line))
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.combined_source_code.push_str(&file_source_code);
                    self.combined_source_code.push('\n');
                    self.processed_files.insert(dep_file.clone());
                }
            }
            self.combined_source_code.push_str("  </dependencies>\n");
        }

        Ok(())
    }

    // Finalize and return the combined source code
    fn finalize(mut self) -> String {
        self.combined_source_code.push_str("</project>\n");
        self.combined_source_code
    }
}

fn main() {
    let args = Args::parse();

    if args.targets.is_empty() && args.target_files.is_empty() && args.reference_files.is_empty() {
        eprintln!("Error: Either <TARGETS> or --target/--reference must be specified.");
        std::process::exit(1);
    }

    match run(&args.targets, &args) {
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
    let mut processor = FileProcessor::new();

    // Process target files
    if !options.target_files.is_empty() {
        processor.combined_source_code.push_str("  <targets>\n");
        for file_path in &options.target_files {
            let file_source_code = process_single_file(file_path, options)?;
            processor.combined_source_code.push_str(&file_source_code);
            processor.processed_files.insert(file_path.clone());
        }
        processor.combined_source_code.push_str("  </targets>\n");
    }

    // Process reference files
    if !options.reference_files.is_empty() {
        processor.combined_source_code.push_str("  <references>\n");
        for file_path in &options.reference_files {
            let file_source_code = process_single_file(file_path, options)?;
            processor.combined_source_code.push_str(&file_source_code);
            processor.processed_files.insert(file_path.clone());
        }
        processor.combined_source_code.push_str("  </references>\n");
    }

    // Initialize resolvers
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

    // Process main files and their dependencies
    for target_path in target_paths {
        if target_path.is_file() {
            if options.target_files.contains(target_path)
                || options.reference_files.contains(target_path)
            {
                continue;
            }

            if options.deps
                && TypeScriptResolver::is_supported_file(target_path)
                && resolver.is_some()
                && ts_resolver.is_some()
            {
                processor.process_file_with_deps(
                    target_path,
                    options,
                    resolver.as_mut().unwrap(),
                    ts_resolver.as_mut().unwrap(),
                )?;
            } else {
                let file_source_code = process_single_file(target_path, options)?;
                processor.combined_source_code.push_str(&file_source_code);
                processor.processed_files.insert(target_path.to_path_buf());
            }
        } else if target_path.is_dir() {
            for entry in Walk::new(target_path).filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file()
                    && !is_ignored(path, &options.ignore_patterns)
                    && !options.target_files.contains(&path.to_path_buf())
                    && !options.reference_files.contains(&path.to_path_buf())
                    && !processor.processed_files.contains(path)
                {
                    if options.deps
                        && TypeScriptResolver::is_supported_file(path)
                        && resolver.is_some()
                        && ts_resolver.is_some()
                    {
                        processor.process_file_with_deps(
                            path,
                            options,
                            resolver.as_mut().unwrap(),
                            ts_resolver.as_mut().unwrap(),
                        )?;
                    } else {
                        let file_source_code = process_single_file(path, options)?;
                        processor.combined_source_code.push_str(&file_source_code);
                        processor.processed_files.insert(path.to_path_buf());
                    }
                }
            }
        }
    }

    // Add dependencies section
    processor.add_dependencies_section(options)?;

    Ok(processor.finalize())
}

fn process_single_file_with_importers(
    file_path: &Path,
    options: &ProcessingOptions,
    importers: &HashSet<PathBuf>,
) -> Result<String, AppError> {
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

    let mut output = format!("  <file name=\"{}\">\n", path_to_display.display());

    // Add importers section
    if !importers.is_empty() {
        output.push_str("    <imported_by>\n");
        for importer in importers {
            output.push_str(&format!(
                "      <importer>{}</importer>\n",
                importer.display()
            ));
        }
        output.push_str("    </imported_by>\n");
    }

    // Add file content
    output.push_str(
        &file_content
            .lines()
            .map(|line| format!("    {}", line))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    output.push_str("\n  </file>\n");

    Ok(output)
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
