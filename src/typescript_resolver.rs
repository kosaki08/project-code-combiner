use crate::dependency_resolver::{DependencyResolver, LanguageResolver};
use oxc_resolver::{ResolveOptions, Resolver};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::{Parser, Query, QueryCursor};

pub struct TypeScriptResolver {
    resolver: Resolver,
    import_query: Query,
    parser: Parser,
}

impl TypeScriptResolver {
    pub fn new() -> Self {
        let resolver = Resolver::new(ResolveOptions {
            extensions: vec![
                ".ts".to_string(),
                ".tsx".to_string(),
                ".js".to_string(),
                ".jsx".to_string(),
            ],
            condition_names: vec!["import".to_string(), "require".to_string()],
            description_files: vec!["package.json".to_string()],
            modules: vec!["node_modules".to_string()],
            ..Default::default()
        });

        let mut parser = Parser::new();
        let language = tree_sitter_typescript::language_typescript();
        parser.set_language(language).unwrap();

        let import_query = Query::new(
            language,
            r#"
            (import_statement
                source: (string) @import_path)
            (import_require_clause
                source: (string) @import_path)
            "#,
        )
        .unwrap();

        Self {
            resolver,
            import_query,
            parser,
        }
    }

    pub fn is_supported_file(file_path: &Path) -> bool {
        if let Some(extension) = file_path.extension() {
            matches!(
                extension.to_str(),
                Some("ts") | Some("tsx") | Some("js") | Some("jsx")
            )
        } else {
            false
        }
    }

    fn resolve_with_alias(
        &self,
        import_path: &str,
        alias_map: &HashMap<String, String>,
        _base_path: &Path,
    ) -> Option<String> {
        if import_path.starts_with('~') {
            let without_tilde = import_path.strip_prefix('~').unwrap();
            let path = without_tilde.trim_start_matches('/');
            if !path.contains('.') {
                return Some(format!("{}.ts", path));
            }
            return Some(path.to_string());
        }

        if import_path.starts_with('@') || !import_path.starts_with('.') {
            return Some(import_path.to_string());
        }

        for (alias, target) in alias_map {
            if import_path.starts_with(alias) {
                let resolved = import_path.replacen(alias, target, 1);
                return Some(resolved);
            }
        }

        Some(import_path.to_string())
    }

    pub fn resolve_import_with_resolver(
        &self,
        import_path: &str,
        current_file: &Path,
        dependency_resolver: &DependencyResolver,
    ) -> Option<PathBuf> {
        let resolved_path = self.resolve_with_alias(
            import_path,
            dependency_resolver.get_alias_map(),
            dependency_resolver.get_base_path(),
        )?;

        let result = if resolved_path.starts_with('@') || !resolved_path.starts_with('.') {
            let project_root = if let Some(current_dir) = current_file.parent() {
                let mut dir = current_dir;
                let mut found_root = None;

                while let Some(parent) = dir.parent() {
                    if dir.ends_with("src") {
                        found_root = Some(parent.to_path_buf());
                        break;
                    }
                    dir = parent;
                }

                found_root.unwrap_or_else(|| current_dir.to_path_buf())
            } else {
                dependency_resolver.get_base_path().to_path_buf()
            };

            let src_dir = project_root.join("src");
            let direct_path = src_dir.join(&resolved_path);

            if direct_path.exists() {
                Some(direct_path)
            } else {
                self.resolver
                    .resolve(&src_dir, &resolved_path)
                    .ok()
                    .map(|resolved| {
                        PathBuf::from(resolved.full_path().to_string_lossy().to_string())
                    })
            }
        } else {
            let current_dir = current_file.parent().unwrap_or(Path::new(""));
            self.resolver
                .resolve(current_dir, &resolved_path)
                .ok()
                .map(|resolved| PathBuf::from(resolved.full_path().to_string_lossy().to_string()))
        };

        result
    }
}

impl LanguageResolver for TypeScriptResolver {
    fn get_imports(&mut self, content: &str) -> Vec<String> {
        let tree = self.parser.parse(content, None).unwrap();
        let mut imports = Vec::new();
        let mut cursor = QueryCursor::new();

        for match_ in cursor.matches(&self.import_query, tree.root_node(), content.as_bytes()) {
            for capture in match_.captures {
                let import_path = capture
                    .node
                    .utf8_text(content.as_bytes())
                    .unwrap()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                imports.push(import_path);
            }
        }

        imports
    }
}
