use serde_json::Value;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::typescript_resolver::TypeScriptResolver;

#[derive(Debug)]
pub struct DependencyResolver {
    base_path: PathBuf,
    alias_map: Option<HashMap<String, String>>,
    resolved_files: HashSet<PathBuf>,
}

pub trait LanguageResolver: Any {
    fn as_any(&self) -> &dyn Any
    where
        Self: Sized,
    {
        self as &dyn Any
    }

    fn get_imports(&mut self, content: &str) -> Vec<String>;
}

impl DependencyResolver {
    pub fn new(project_root: &Path, load_aliases: bool) -> io::Result<Self> {
        let alias_map = if load_aliases {
            let tsconfig_path = project_root.join("tsconfig.json");
            Some(Self::load_tsconfig_aliases(&tsconfig_path)?)
        } else {
            None
        };

        Ok(Self {
            base_path: project_root.to_path_buf(),
            alias_map,
            resolved_files: HashSet::new(),
        })
    }

    fn load_tsconfig_aliases(tsconfig_path: &Path) -> io::Result<HashMap<String, String>> {
        let mut alias_map = HashMap::new();

        if tsconfig_path.exists() {
            let content = fs::read_to_string(tsconfig_path)?;
            let config: Value = serde_json::from_str(&content)?;

            if let Some(compiler_options) = config.get("compilerOptions") {
                if let Some(paths) = compiler_options.get("paths") {
                    if let Some(paths_obj) = paths.as_object() {
                        for (alias, targets) in paths_obj {
                            if let Some(target) = targets.get(0) {
                                if let Some(target_str) = target.as_str() {
                                    let clean_alias = alias.trim_end_matches("/*");
                                    let clean_target = target_str.trim_end_matches("/*");
                                    alias_map
                                        .insert(clean_alias.to_string(), clean_target.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(alias_map)
    }

    pub fn resolve_deps<T: LanguageResolver>(
        &mut self,
        entry_file: &Path,
        resolver: &mut T,
    ) -> io::Result<Vec<PathBuf>> {
        let mut file_queue = vec![entry_file.to_path_buf()];
        let mut resolved_files = Vec::new();

        while let Some(current_file) = file_queue.pop() {
            // Skip if the path contains "node_modules"
            if should_ignore_file(&current_file) {
                continue;
            }

            if self.resolved_files.contains(&current_file) {
                continue;
            }

            let content = fs::read_to_string(&current_file)?;
            let imports = resolver.get_imports(&content);

            for import_path in imports {
                if let Some(ts_resolver) = resolver.as_any().downcast_ref::<TypeScriptResolver>() {
                    if let Some(resolved_path) =
                        ts_resolver.resolve_import_with_resolver(&import_path, &current_file, self)
                    {
                        // Skip if the resolved path contains "node_modules"
                        if should_ignore_file(&resolved_path) {
                            continue;
                        }

                        if !self.resolved_files.contains(&resolved_path) {
                            file_queue.push(resolved_path);
                        }
                    }
                }
            }

            self.resolved_files.insert(current_file.clone());
            resolved_files.push(current_file);
        }

        Ok(resolved_files)
    }

    pub fn get_alias_map(&self) -> Option<&HashMap<String, String>> {
        self.alias_map.as_ref()
    }

    pub fn get_base_path(&self) -> &Path {
        &self.base_path
    }
}

fn should_ignore_file(path: &Path) -> bool {
    path.to_string_lossy().contains("node_modules")
}
