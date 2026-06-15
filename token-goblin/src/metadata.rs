//! Load metadata information from `Cargo.toml` package.
//! It is primarly used to get dependencies information, but might be extended in future.
//!
//! 1. Parsing is started from `CARGO_MANIFEST_PATH`.
//! 2. if some dependency is require workspace version,
//!    try to find parent workspace `Cargo.toml` and fill information from it.
//!

pub mod targets;

use std::path::{Component, Path, PathBuf};

use proc_macro2::Span;

use crate::{
    Result, macro_impl,
    path::{manifest_path, search_for_parent_manifest},
};
type TomlTable = toml::map::Map<String, toml::Value>;

/// Normalize a Cargo target/package name to rustc crate name form (`-` → `_`).
pub fn cargo_crate_name(name: &str) -> String {
    name.replace('-', "_")
}

// Whether value set, or uses workspace version
#[derive(Debug)]
pub enum ValueOrWorkspace {
    Value {
        value: toml::Value,
        /// Save `Cargo.toml` location in case if `path` version is used.
        rel_path: PathBuf,
    },
    #[allow(unused)]
    Workspace { extra: TomlTable },
}
impl ValueOrWorkspace {
    fn from_parts(value: toml::Value, rel_path: PathBuf) -> Result<Self> {
        Ok(if is_workspace(&value) {
            // strange if workspace dep have a path?
            let extra = value
                .as_table()
                .ok_or_else(|| error!(Span::call_site() => "Failed to convert to table"))?
                .clone();
            Self::Workspace { extra }
        } else {
            Self::Value { value, rel_path }
        })
    }
}

/// Returns true if dependency has `workspace` field.
pub fn is_workspace(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(table) => table.contains_key("workspace"),
        _ => false,
    }
}
#[derive(Debug)]
pub struct Metadata {
    pub dependencies: Vec<Dependency>,
}
impl Metadata {
    fn from_manifest(
        manifest_path: &Path,
        extra_metadata: &macro_impl::ExtraMetadata,
    ) -> Result<Self> {
        let manifest: toml::Value = read_toml_file(manifest_path)?;

        let whitelisted_deps = extra_metadata.dependencies.clone();
        let is_strict = extra_metadata.strict_dependencies;
        let strict_whitelist = |(name, _): &(&String, &toml::Value)| {
            if is_strict {
                whitelisted_deps.contains(*name)
            } else {
                true
            }
        };

        let mut dependencies = match manifest.get("dev-dependencies").and_then(|v| v.as_table()) {
            Some(dependencies) => dependencies
                .iter()
                .filter(|(name, _)| *name != "token-goblin-runtime")
                .filter(strict_whitelist)
                .map(|(name, value)| {
                    Dependency::new(name.clone(), manifest_path.to_path_buf(), value.clone())
                })
                .collect::<Result<Vec<Dependency>>>()?,
            None => Vec::new(),
        };

        let remove_optional = |value: &mut toml::Value| {
            if let toml::Value::Table(table) = value {
                table.remove("optional");
            }
        };
        // Add dependencies from `dependencies` section.
        // ensure to remove `optional` field from dependencies.
        dependencies.extend(
            match manifest.get("dependencies").and_then(|v| v.as_table()) {
                Some(dependencies) => dependencies
                    .iter()
                    .filter(|(name, _)| whitelisted_deps.contains(*name))
                    .map(|(name, value)| {
                        let mut new_value = value.clone();
                        remove_optional(&mut new_value);
                        Dependency::new(name.clone(), manifest_path.to_path_buf(), new_value)
                    })
                    .collect::<Result<Vec<Dependency>>>()?,
                None => Vec::new(),
            },
        );

        if extra_metadata.skip_duplicate {
            // the sort is stable, mean dev-dependencies will be before dependencies.
            dependencies.sort_by_cached_key(|dependency| dependency.name.clone());
            dependencies.dedup_by(|a, b| a.name == b.name);
        }

        // Add remaining dependencies to the candidates from workspace resolution.
        let mut remaining_deps = whitelisted_deps;
        for dependency in &dependencies {
            remaining_deps.remove(dependency.name.as_str());
        }
        for name in remaining_deps {
            dependencies.push(Dependency::new_from_workspace(name.clone()));
        }
        Ok(Metadata { dependencies })
    }
    fn try_resolve_workspace_dependency(
        &mut self,
        workspace_path: &Path,
        workspace_dependencies: &toml::map::Map<String, toml::Value>,
    ) -> Result<()> {
        for dependency in &mut self.dependencies {
            if !dependency.is_workspace() {
                continue;
            }
            let Some(workspace_dependency) = workspace_dependencies.get(dependency.name.as_str())
            else {
                bail!(Span::call_site() => "Workspace dependency not found");
            };
            // TODO: add extra knowledge from workspace dependency.
            dependency.value = ValueOrWorkspace::from_parts(
                workspace_dependency.clone(),
                workspace_path.to_path_buf(),
            )?;
        }
        Ok(())
    }

    fn workspace_dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies
            .iter()
            .filter(|dependency| dependency.is_workspace())
    }
    fn has_workspace_dependency(&self) -> bool {
        // use count to enforce evaluation of iterator
        self.workspace_dependencies().count() > 0
    }
}
#[derive(Debug)]
pub struct Dependency {
    pub name: String,
    pub value: ValueOrWorkspace,
}

impl Dependency {
    pub fn new(name: String, rel_path: PathBuf, value: toml::Value) -> Result<Self> {
        Ok(Self {
            name,
            value: ValueOrWorkspace::from_parts(value, rel_path)?,
        })
    }
    // No dependency in package manifest, but we expect it to be in workspace.
    pub fn new_from_workspace(name: String) -> Self {
        Self {
            name,
            value: ValueOrWorkspace::Workspace {
                extra: TomlTable::new(),
            },
        }
    }
    fn is_workspace(&self) -> bool {
        match &self.value {
            ValueOrWorkspace::Value { .. } => false,
            ValueOrWorkspace::Workspace { extra: _ } => true,
        }
    }
}

/// Load `dev-dependencies` section of `Cargo.toml`.
/// Used on expansion of macro definition (aka `munch` macro)
pub fn load_dependencies(extra_metadata: &macro_impl::ExtraMetadata) -> Result<Metadata> {
    let manifest_path = manifest_path()?;
    let mut metadata = Metadata::from_manifest(&manifest_path, extra_metadata)?;

    if !metadata.has_workspace_dependency() {
        return Ok(metadata);
    }
    // resolve workspace dependencies
    let Some((workspace_manifest, workspace_path)) = find_workspace_manifest(&manifest_path)?
    else {
        return Err(error!(
            Span::call_site() =>
            "Dependency uses workspace inheritance, but no containing workspace manifest was found"
        ));
    };

    let Some(workspace_table) = workspace_manifest
        .get("workspace")
        .and_then(|v| v.as_table())
    else {
        return Err(error!(Span::call_site() => "Workspace table not found"));
    };

    let workspace_dependencies = workspace_table
        .get("dependencies")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    metadata.try_resolve_workspace_dependency(&workspace_path, &workspace_dependencies)?;

    if let Some(dependency) = metadata.workspace_dependencies().next() {
        return Err(
            error!(Span::call_site() => "Dependency has workspace field, but is not resolved: {}", dependency.name),
        );
    }
    Ok(metadata)
}

/// Find the nearest workspace manifest that contains the crate at `manifest_path`.
pub fn find_workspace_manifest(manifest_path: &Path) -> Result<Option<(toml::Value, PathBuf)>> {
    if !manifest_path.is_file() {
        return Ok(None);
    }

    let crate_root = manifest_path
        .parent()
        .ok_or_else(|| error!(Span::call_site() => "Manifest path has no parent"))?
        .to_path_buf();

    search_for_parent_manifest(&crate_root, |candidate_manifest| {
        extract_workspace_manifest(&crate_root, candidate_manifest)
    })
}

fn extract_workspace_manifest(
    crate_root: &Path,
    manifest_path: &Path,
) -> Result<Option<(toml::Value, PathBuf)>> {
    let manifest = read_toml_file(manifest_path)?;
    let Some(workspace_table) = manifest.get("workspace").and_then(|v| v.as_table()) else {
        return Ok(None);
    };
    let workspace_root = manifest_path
        .parent()
        .ok_or_else(|| error!(Span::call_site() => "Manifest path has no parent"))?;
    if !is_crate_in_workspace(crate_root, workspace_root, &manifest, workspace_table)? {
        return Ok(None);
    }
    Ok(Some((manifest, manifest_path.to_path_buf())))
}

/// Return workspace root directory for a crate manifest, falling back to the crate root itself.
pub fn workspace_root_for_manifest(manifest_path: &Path) -> Result<PathBuf> {
    let crate_root = manifest_path
        .parent()
        .ok_or_else(|| error!(Span::call_site() => "Manifest path has no parent"))?
        .to_path_buf();

    Ok(find_workspace_manifest(manifest_path)?
        .and_then(|(_, workspace_manifest)| workspace_manifest.parent().map(Path::to_path_buf))
        .unwrap_or(crate_root))
}

fn is_crate_in_workspace(
    crate_root: &Path,
    workspace_root: &Path,
    workspace_manifest: &toml::Value,
    workspace_table: &TomlTable,
) -> Result<bool> {
    let crate_root = canonicalize_lossy(crate_root);
    let workspace_root = canonicalize_lossy(workspace_root);

    let relative_path = crate_root.strip_prefix(&workspace_root).map_err(
        |_| error!(Span::call_site() => "Failed to compute crate path relative to workspace"),
    )?;
    let relative_path = normalize_relative_path(relative_path);

    let exclude = string_array(workspace_table.get("exclude"));
    if is_path_matched_by_any_pattern(&relative_path, &exclude) {
        return Ok(false);
    }

    if relative_path.is_empty() {
        return Ok(workspace_manifest.get("package").is_some());
    }

    let members = string_array(workspace_table.get("members"));
    if members.is_empty() {
        // Default workspace members behavior: auto-discover all packages except excluded.
        return Ok(true);
    }

    Ok(is_path_matched_by_any_pattern(&relative_path, &members))
}

fn string_array(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn is_path_matched_by_any_pattern(relative_path: &str, patterns: &[String]) -> bool {
    for prefix in path_prefixes(relative_path) {
        if patterns
            .iter()
            .any(|pattern| wildcard_match(pattern, &prefix))
        {
            return true;
        }
    }
    false
}

fn path_prefixes(relative_path: &str) -> Vec<String> {
    if relative_path.is_empty() {
        return vec![String::new()];
    }

    let segments: Vec<&str> = relative_path.split('/').collect();
    (0..segments.len())
        .map(|end| segments[..=end].join("/"))
        .collect()
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    fn match_at(pattern: &[u8], text: &[u8]) -> bool {
        match (pattern.first(), text.first()) {
            (None, None) => true,
            (Some(b'*'), None) => match_at(&pattern[1..], text),
            (Some(b'*'), Some(_)) => match_at(pattern, &text[1..]) || match_at(&pattern[1..], text),
            (Some(b'?'), Some(_)) => match_at(&pattern[1..], &text[1..]),
            (Some(p), Some(t)) if p == t => match_at(&pattern[1..], &text[1..]),
            _ => false,
        }
    }

    match_at(pattern.as_bytes(), text.as_bytes())
}

pub(crate) fn read_toml_file(path: &Path) -> Result<toml::Value> {
    let val = std::fs::read_to_string(path)
        .map_err(|e| error!(Span::call_site() => "Failed to read TOML file: {e}"))?;
    toml::from_str(&val).map_err(|e| error!(Span::call_site() => "Failed to parse TOML file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("token_goblin_metadata_{name}_{nanos}"));
        fs::create_dir_all(&dir).expect("create temp workspace dir");
        dir
    }

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, contents).expect("write file");
    }

    #[test]
    fn own_manifest_with_workspace_is_accepted() {
        let root = temp_workspace("own_workspace");
        write(
            &root.join("Cargo.toml"),
            r#"
[workspace]

[package]
name = "member"
version = "0.1.0"
edition = "2024"
"#,
        );

        let found = find_workspace_manifest(&root.join("Cargo.toml"))
            .expect("lookup should succeed")
            .expect("workspace should be found");
        assert_eq!(found.1, root.join("Cargo.toml"));
    }

    #[test]
    fn no_workspace_returns_none() {
        let root = temp_workspace("no_workspace");
        write(
            &root.join("Cargo.toml"),
            r#"
[package]
name = "solo"
version = "0.1.0"
edition = "2024"
"#,
        );

        assert!(
            find_workspace_manifest(&root.join("Cargo.toml"))
                .expect("lookup should succeed")
                .is_none()
        );
    }

    #[test]
    fn parent_workspace_accepts_explicit_member() {
        let root = temp_workspace("explicit_member");
        write(
            &root.join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/member"]

[package]
name = "root"
version = "0.1.0"
edition = "2024"
"#,
        );
        write(
            &root.join("crates/member/Cargo.toml"),
            r#"
[package]
name = "member"
version = "0.1.0"
edition = "2024"
"#,
        );

        let member_root = root.join("crates/member");
        let found = find_workspace_manifest(&member_root.join("Cargo.toml"))
            .expect("lookup should succeed")
            .expect("member should belong to workspace");
        assert_eq!(found.1, root.join("Cargo.toml"));
    }

    #[test]
    fn parent_workspace_rejects_non_member() {
        let root = temp_workspace("non_member");
        write(
            &root.join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/member"]

[package]
name = "root"
version = "0.1.0"
edition = "2024"
"#,
        );
        write(
            &root.join("other/Cargo.toml"),
            r#"
[package]
name = "other"
version = "0.1.0"
edition = "2024"
"#,
        );

        assert!(
            find_workspace_manifest(&root.join("other/Cargo.toml"))
                .expect("lookup should succeed")
                .is_none()
        );
    }

    #[test]
    fn parent_workspace_rejects_excluded_wildcard_path() {
        let root = temp_workspace("excluded");
        write(
            &root.join("Cargo.toml"),
            r#"
[workspace]
members = ["*"]
exclude = ["fixtures/*"]

[package]
name = "root"
version = "0.1.0"
edition = "2024"
"#,
        );
        write(
            &root.join("fixtures/tests/smoke/Cargo.toml"),
            r#"
[package]
name = "smoke"
version = "0.1.0"
edition = "2024"
"#,
        );

        assert!(
            find_workspace_manifest(&root.join("fixtures/tests/smoke/Cargo.toml"))
                .expect("lookup should succeed")
                .is_none()
        );
    }

    #[test]
    fn workspace_root_for_manifest_falls_back_to_crate_root() {
        let root = temp_workspace("fallback");
        write(
            &root.join("Cargo.toml"),
            r#"
[package]
name = "solo"
version = "0.1.0"
edition = "2024"
"#,
        );

        assert_eq!(
            workspace_root_for_manifest(&root.join("Cargo.toml")).expect("lookup should succeed"),
            root
        );
    }
}
