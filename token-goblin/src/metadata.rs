//! Load metadata information from `Cargo.toml` package.
//! It is primarly used to get dependencies information, but might be extended in future.
//!
//! 1. Parsing is started from `CARGO_MANIFEST_PATH`.
//! 2. if some dependency is require workspace version,
//!    try to find parent workspace `Cargo.toml` and fill information from it.
//!

use std::path::{Path, PathBuf};

use proc_macro2::Span;

use crate::{
    Result,
    path::{manifest_path, search_for_parent_manifest},
};
type TomlTable = toml::map::Map<String, toml::Value>;

// Whether value set, or uses workspace version
#[derive(Debug)]
pub enum ValueOrWorkspace {
    Value(toml::Value),
    #[allow(unused)]
    Workspace {
        extra: TomlTable,
    },
}
impl ValueOrWorkspace {
    fn from_value(value: toml::Value) -> Result<Self> {
        Ok(if is_workspace(&value) {
            let extra = value
                .as_table()
                .ok_or_else(|| error!(Span::call_site() => "Failed to convert to table"))?
                .clone();
            Self::Workspace { extra }
        } else {
            Self::Value(value)
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
    fn from_manifest(manifest_path: &Path) -> Result<Self> {
        let manifest: toml::Value = read_toml_file(manifest_path)?;

        let dependencies = match manifest.get("dev-dependencies").and_then(|v| v.as_table()) {
            Some(dependencies) => dependencies
                .iter()
                .filter(|(name, _)| *name != "token-goblin-runtime")
                .map(|(name, value)| {
                    Dependency::new(name.clone(), manifest_path.to_path_buf(), value.clone())
                })
                .collect::<Result<Vec<Dependency>>>()?,
            None => Vec::new(),
        };
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
            // TODO: add extra knowledge from workspace dependency.
            dependency.value = ValueOrWorkspace::from_value(workspace_dependency.clone())?;
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
    /// Save `Cargo.toml` location in case if `path` version is used.
    pub rel_path: PathBuf,
    pub value: ValueOrWorkspace,
}

impl Dependency {
    pub fn new(name: String, rel_path: PathBuf, value: toml::Value) -> Result<Self> {
        Ok(Self {
            name,
            rel_path,
            value: ValueOrWorkspace::from_value(value)?,
        })
    }
    fn is_workspace(&self) -> bool {
        match &self.value {
            ValueOrWorkspace::Value(_) => false,
            ValueOrWorkspace::Workspace { extra: _ } => true,
        }
    }
}

/// Load `dev-dependencies` section of `Cargo.toml`.
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
    let manifest: toml::Value = read_toml_file(path)?;

    if manifest.get("workspace").is_some() {
        return Ok(Some(manifest));
    }
    Ok(None)
}

fn read_toml_file(path: &Path) -> Result<toml::Value> {
    let val = std::fs::read_to_string(path)
        .map_err(|e| error!(Span::call_site() => "Failed to read TOML file: {e}"))?;
    toml::from_str(&val).map_err(|e| error!(Span::call_site() => "Failed to parse TOML file: {e}"))
}
