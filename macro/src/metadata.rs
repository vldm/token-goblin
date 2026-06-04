//! Load metadata information from `Cargo.toml` package.
//! It is primarly used to get dependencies information, but might be extended in future.
//!
//! 1. Parsing is started from `CARGO_MANIFEST_PATH`.
//! 2. if some dependency is require workspace version,
//!    try to find parent workspace `Cargo.toml` and fill information from it.
//!

use std::path::{Path, PathBuf};

use proc_macro::Span;

use crate::{
    Result,
    path::{manifest_path, search_for_parent_manifest},
};

pub struct Metadata {
    pub dependencies: Vec<Dependency>,
}
impl Metadata {
    fn from_manifest(manifest_path: &PathBuf) -> Result<Self> {
        let manifest: toml::Value = toml::from_str(&std::fs::read_to_string(manifest_path)?)?;
        let dependencies = manifest
            .get("build-dependencies")
            .and_then(|v| v.as_table())
            .ok_or_else(
                || error!(Span::call_site() => "build-dependencies section is not found"),
            )?;
        let dependencies = dependencies
            .iter()
            .map(|(name, value)| {
                Dependency::new(name.clone(), manifest_path.clone(), value.clone())
            })
            .collect();
        Ok(Metadata { dependencies })
    }
    fn try_resolve_workspace_dependency(
        &mut self,
        workspace_dependencies: &toml::map::Map<String, toml::Value>,
    ) -> Result<()> {
        for dependency in &mut self.dependencies {
            if !dependency.is_workspace() {
                continue;
            }
            let Some(workspace_dependency) = workspace_dependencies.get(dependency.name.as_str())
            else {
                return Err(error!(Span::call_site() => "Workspace dependency not found"));
            };
            dependency.value = workspace_dependency.clone();
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
        self.workspace_dependencies()
            .inspect(|dependency| debug!("Workspace dependency: {}", dependency.name))
            .count()
            > 0
    }
}

pub struct Dependency {
    pub name: String,
    /// Save `Cargo.toml` location in case if `path` version is used.
    pub rel_path: PathBuf,
    pub value: toml::Value,
}

impl Dependency {
    pub fn new(name: String, rel_path: PathBuf, value: toml::Value) -> Self {
        Self {
            name,
            rel_path,
            value,
        }
    }
    /// Returns true if dependency has `workspace` field.
    pub fn is_workspace(&self) -> bool {
        match self.value {
            toml::Value::Table(ref table) => table.contains_key("workspace"),
            _ => false,
        }
    }
}

/// Load `build-dependencies` section of `Cargo.toml`.
/// Used on expansion of macro definition (aka `munch` macro)
pub fn load_dependencies() -> Result<Metadata> {
    let manifest_path = manifest_path()?;
    let mut metadata = Metadata::from_manifest(&manifest_path)?;

    if !metadata.has_workspace_dependency() {
        return Ok(metadata);
    }
    // resolve workspace dependencies
    let workspace_manifest =
        search_for_parent_manifest(&manifest_path, extract_workspace_manifest)?;

    let Some(workspace_table) = workspace_manifest
        .get("workspace")
        .and_then(|v| v.as_table())
    else {
        return Err(error!(Span::call_site() => "Workspace table not found"));
    };
    metadata.try_resolve_workspace_dependency(workspace_table)?;

    if let Some(dependency) = metadata.workspace_dependencies().next() {
        return Err(
            error!(Span::call_site() => "Dependency has workspace field, but is not resolved: {}", dependency.name),
        );
    }
    Ok(metadata)
}
// Try load file
// Return `Some` if `workspace` key exists.
fn extract_workspace_manifest(path: &Path) -> Result<Option<toml::Value>> {
    let manifest: toml::Value = toml::from_str(&std::fs::read_to_string(path)?)?;

    if manifest.get("workspace").is_some() {
        return Ok(Some(manifest));
    }
    Ok(None)
}
