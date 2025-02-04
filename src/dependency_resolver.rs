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
    dependency_graph: HashMap<PathBuf, HashSet<PathBuf>>,
    processing_stack: Vec<PathBuf>,
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
            match Self::load_tsconfig_aliases(&project_root.join("tsconfig.json")) {
                Ok(aliases) => Some(aliases),
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(Self {
            base_path: project_root.to_path_buf(),
            alias_map,
            resolved_files: HashSet::new(),
            dependency_graph: HashMap::new(),
            processing_stack: Vec::new(),
        })
    }

    fn load_tsconfig_aliases(tsconfig_path: &Path) -> io::Result<HashMap<String, String>> {
        if !tsconfig_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(tsconfig_path)?;

        match serde_json::from_str::<Value>(&content) {
            Ok(config) => {
                let mut alias_map = HashMap::new();

                if let Some(compiler_options) = config.get("compilerOptions") {
                    if let Some(paths) = compiler_options.get("paths") {
                        if let Some(paths_obj) = paths.as_object() {
                            for (alias, targets) in paths_obj {
                                if let Some(target) = targets.get(0) {
                                    if let Some(target_str) = target.as_str() {
                                        let clean_alias = alias.trim_end_matches("/*");
                                        let clean_target = target_str.trim_end_matches("/*");
                                        alias_map.insert(
                                            clean_alias.to_string(),
                                            clean_target.to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(alias_map)
            }
            Err(_) => Ok(HashMap::new()),
        }
    }

    pub fn resolve_deps<T: LanguageResolver>(
        &mut self,
        entry_file: &Path,
        resolver: &mut T,
    ) -> io::Result<Vec<PathBuf>> {
        self.processing_stack.clear();
        self.dependency_graph.clear();
        self.resolved_files.clear();

        self.resolve_deps_recursive(entry_file, resolver)?;

        let mut all_files: HashSet<PathBuf> = HashSet::new();
        let mut stack = vec![entry_file.to_path_buf()];

        while let Some(current) = stack.pop() {
            if all_files.insert(current.clone()) {
                if let Some(deps) = self.dependency_graph.get(&current) {
                    stack.extend(deps.iter().cloned());
                }
            }
        }

        Ok(all_files.into_iter().collect())
    }

    fn resolve_deps_recursive<T: LanguageResolver>(
        &mut self,
        current_file: &Path,
        resolver: &mut T,
    ) -> io::Result<()> {
        if self.processing_stack.contains(&current_file.to_path_buf()) {
            println!(
                "Warning: Circular dependency detected for file: {}",
                current_file.display()
            );
            return Ok(());
        }

        if self.resolved_files.contains(current_file) {
            return Ok(());
        }

        self.processing_stack.push(current_file.to_path_buf());

        let content = fs::read_to_string(current_file)?;
        let imports = resolver.get_imports(&content);

        for import_path in imports {
            if let Some(ts_resolver) = resolver.as_any().downcast_ref::<TypeScriptResolver>() {
                if let Some(resolved_path) =
                    ts_resolver.resolve_import_with_resolver(&import_path, current_file, self)
                {
                    if should_ignore_file(&resolved_path) {
                        continue;
                    }

                    self.dependency_graph
                        .entry(current_file.to_path_buf())
                        .or_default()
                        .insert(resolved_path.clone());

                    self.resolve_deps_recursive(&resolved_path, resolver)?;
                }
            }
        }

        self.processing_stack.pop();
        self.resolved_files.insert(current_file.to_path_buf());
        Ok(())
    }

    pub fn get_all_importers(&self, file: &Path) -> HashSet<PathBuf> {
        let mut all_importers = HashSet::new();
        let mut stack = vec![file.to_path_buf()];
        let mut visited = HashSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            for (importer, deps) in &self.dependency_graph {
                if deps.contains(&current) {
                    all_importers.insert(importer.clone());
                    stack.push(importer.clone());
                }
            }
        }

        all_importers
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
