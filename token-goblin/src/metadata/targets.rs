//! Cargo target discovery and active-target resolution for module paths.

use std::{
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
};

use proc_macro2::Span;
use toml::Value;

use super::{cargo_crate_name, read_toml_file};
use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Lib,
    Bin,
    Test,
    Example,
    Bench,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRoot {
    pub kind: TargetKind,
    pub crate_name: String,
    pub root_file: PathBuf,
    pub module_dir: PathBuf,
}

impl TargetRoot {
    pub fn discover(manifest_path: &Path, crate_root: &Path) -> Result<Vec<Self>> {
        let manifest = read_toml_file(manifest_path)?;
        let package_name = manifest
            .get("package")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| error!(Span::call_site() => "package.name missing from manifest"))?;
        let default_crate_name = cargo_crate_name(package_name);

        let mut targets = Vec::new();

        if let Some(lib) = manifest.get("lib").and_then(|v| v.as_table()) {
            let name = table_string(lib.get("name")).map_or_else(
                || default_crate_name.clone(),
                |name| cargo_crate_name(&name),
            );
            let root_file =
                table_path(lib.get("path")).unwrap_or_else(|| PathBuf::from("src/lib.rs"));
            targets.push(Self::new(TargetKind::Lib, name, &root_file));
        } else if crate_root.join("src/lib.rs").is_file() {
            targets.push(Self::new(
                TargetKind::Lib,
                default_crate_name.clone(),
                "src/lib.rs".as_ref(),
            ));
        }

        if manifest_bool(&manifest, "autobins", true) {
            if manifest.get("bin").is_none() && crate_root.join("src/main.rs").is_file() {
                targets.push(Self::new(
                    TargetKind::Bin,
                    default_crate_name.clone(),
                    "src/main.rs".as_ref(),
                ));
            }
            collect_auto_roots(crate_root, "src/bin", TargetKind::Bin, &mut targets)?;
        }

        collect_explicit_targets(
            &manifest,
            "bin",
            TargetKind::Bin,
            &mut targets,
            Some("src/main.rs"),
        )?;

        if manifest_bool(&manifest, "autotests", true) {
            collect_auto_roots(crate_root, "tests", TargetKind::Test, &mut targets)?;
        }
        collect_explicit_targets(&manifest, "test", TargetKind::Test, &mut targets, None)?;

        if manifest_bool(&manifest, "autoexamples", true) {
            collect_auto_roots(crate_root, "examples", TargetKind::Example, &mut targets)?;
        }
        collect_explicit_targets(
            &manifest,
            "example",
            TargetKind::Example,
            &mut targets,
            None,
        )?;

        if manifest_bool(&manifest, "autobenches", true) {
            collect_auto_roots(crate_root, "benches", TargetKind::Bench, &mut targets)?;
        }
        collect_explicit_targets(&manifest, "bench", TargetKind::Bench, &mut targets, None)?;

        dedupe_targets(&mut targets);
        Ok(targets)
    }

    pub fn new(kind: TargetKind, crate_name: String, root_file: &Path) -> Self {
        let module_dir = module_dir_for_root(root_file);
        Self {
            kind,
            crate_name,
            root_file: normalize_path(root_file),
            module_dir,
        }
    }

    pub fn select_for_file<'a>(targets: &'a [Self], module_file: &Path) -> Result<&'a Self> {
        let module_file = normalize_path(module_file);
        let env = TargetEnv::from_process();

        let mut candidates: Vec<&Self> = targets
            .iter()
            .filter(|target| target.contains_file(&module_file, targets))
            .collect();

        if let Some(crate_name) = &env.crate_name {
            candidates.retain(|target| &target.crate_name == crate_name);
        }

        if let Some(bin_name) = &env.bin_name {
            candidates.retain(|target| {
                matches!(target.kind, TargetKind::Bin | TargetKind::Example)
                    && target.crate_name == *bin_name
            });
        } else {
            candidates
                .retain(|target| !matches!(target.kind, TargetKind::Bin | TargetKind::Example));
        }

        if candidates.len() == 1 {
            return Ok(candidates[0]);
        }

        let root_matches: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|target| target.root_file == module_file)
            .collect();
        if root_matches.len() == 1 {
            return Ok(root_matches[0]);
        }

        if candidates.is_empty() {
            bail!(
                Span::call_site() =>
                "SpanLocation: no Cargo target owns `{}`",
                module_file.display()
            );
        }

        let summary = candidates
            .iter()
            .map(|target| {
                format!(
                    "{:?} `{}` root=`{}`",
                    target.kind,
                    target.crate_name,
                    target.root_file.display()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            Span::call_site() =>
            "SpanLocation: ambiguous Cargo target for `{}`: {summary}",
            module_file.display()
        )
    }

    fn contains_file(&self, module_file: &Path, all_targets: &[Self]) -> bool {
        if self.root_file == module_file {
            return true;
        }
        if all_targets
            .iter()
            .any(|target| target.root_file == module_file)
        {
            return false;
        }
        module_file.starts_with(&self.module_dir)
    }

    pub fn file_module_path(&self, module_file: &Path) -> syn::Path {
        if self.root_file == module_file {
            return empty_path();
        }
        let relative = module_file
            .strip_prefix(&self.module_dir)
            .unwrap_or(module_file);
        module_path_from_relative(relative)
    }
}

#[derive(Debug, Default)]
struct TargetEnv {
    crate_name: Option<String>,
    bin_name: Option<String>,
}

impl TargetEnv {
    fn from_process() -> Self {
        Self {
            crate_name: std::env::var("CARGO_CRATE_NAME")
                .ok()
                .map(|name| cargo_crate_name(&name)),
            bin_name: std::env::var("CARGO_BIN_NAME").ok(),
        }
    }
}

fn collect_explicit_targets(
    manifest: &Value,
    table_key: &str,
    kind: TargetKind,
    targets: &mut Vec<TargetRoot>,
    default: Option<&'static str>,
) -> Result<()> {
    let Some(entries) = manifest.get(table_key).and_then(Value::as_array) else {
        return Ok(());
    };

    for entry in entries {
        let Some(table) = entry.as_table() else {
            continue;
        };
        let name = table_string(table.get("name"))
            .map(|name| cargo_crate_name(&name))
            .unwrap_or_default();
        let root_file = table_path(table.get("path"))
            .or_else(|| default.map(PathBuf::from))
            .ok_or_else(|| {
                error!(
                    Span::call_site() =>
                    "[[{table_key}]] entry missing path"
                )
            })?;
        targets.push(TargetRoot::new(kind, name, &root_file));
    }
    Ok(())
}

fn collect_auto_roots(
    crate_root: &Path,
    dir_name: &str,
    kind: TargetKind,
    targets: &mut Vec<TargetRoot>,
) -> Result<()> {
    let dir = crate_root.join(dir_name);
    if !dir.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(&dir).map_err(
        |e| error!(Span::call_site() => "SpanLocation: failed to read `{}`: {e}", dir.display()),
    )?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            let rel = path.strip_prefix(crate_root).unwrap_or(&path);
            let name = path
                .file_stem()
                .and_then(OsStr::to_str)
                .map(cargo_crate_name)
                .unwrap_or_default();
            targets.push(TargetRoot::new(kind, name, &normalize_path(rel)));
            continue;
        }

        if path.is_dir() {
            let main_rs = path.join("main.rs");
            if main_rs.is_file() {
                let rel = main_rs.strip_prefix(crate_root).unwrap_or(&main_rs);
                let name = path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .map(cargo_crate_name)
                    .unwrap_or_default();
                targets.push(TargetRoot::new(kind, name, &normalize_path(rel)));
            }
        }
    }
    Ok(())
}

fn dedupe_targets(targets: &mut Vec<TargetRoot>) {
    targets.sort_by(|left, right| left.root_file.cmp(&right.root_file));
    targets.dedup_by(|left, right| left.root_file == right.root_file);
}

fn module_dir_for_root(root_file: &Path) -> PathBuf {
    root_file
        .parent()
        .map_or_else(|| PathBuf::from("."), normalize_path)
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_os_string()),
            Component::ParentDir => Some(OsStr::new("..").to_os_string()),
            Component::CurDir | Component::Prefix(_) | Component::RootDir => None,
        })
        .collect()
}

fn manifest_bool(manifest: &toml::Value, key: &str, default: bool) -> bool {
    manifest
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn table_string(value: Option<&toml::Value>) -> Option<String> {
    value.and_then(|v| v.as_str()).map(str::to_string)
}

fn table_path(value: Option<&toml::Value>) -> Option<PathBuf> {
    value
        .and_then(|v| v.as_str())
        .map(|path| normalize_path(Path::new(path)))
}

pub(crate) fn module_path_from_relative(relative: &Path) -> syn::Path {
    let mut segments = Vec::new();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            continue;
        };
        let name = name.to_string_lossy();
        if name == "mod.rs" {
            break;
        }
        if let Some(stem) = name.strip_suffix(".rs") {
            segments.push(cargo_crate_name(stem));
            break;
        }
        segments.push(cargo_crate_name(&name));
    }
    path_from_segment_strs(&segments)
}

fn path_from_segment_strs(segments: &[String]) -> syn::Path {
    let mut result = syn::punctuated::Punctuated::<syn::PathSegment, syn::Token![::]>::new();
    for segment in segments {
        let ident = if let Some(raw) = segment.strip_prefix("r#") {
            syn::Ident::new_raw(raw, Span::call_site())
        } else {
            syn::Ident::new(segment, Span::call_site())
        };
        result.push(syn::PathSegment {
            ident,
            arguments: syn::PathArguments::None,
        });
    }
    syn::Path {
        leading_colon: None,
        segments: result,
    }
}

fn empty_path() -> syn::Path {
    syn::Path {
        leading_colon: None,
        segments: syn::punctuated::Punctuated::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_display(path: &syn::Path) -> String {
        path.segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }

    #[test]
    fn module_path_from_relative_handles_nested_files() {
        let path = module_path_from_relative(Path::new("foo/bar.rs"));
        assert_eq!(path_display(&path), "foo::bar");

        let path = module_path_from_relative(Path::new("foo/mod.rs"));
        assert_eq!(path_display(&path), "foo");
    }

    #[test]
    fn target_root_file_module_path_is_empty() {
        let target = TargetRoot::new(TargetKind::Lib, "demo".to_string(), "src/lib.rs".as_ref());
        assert!(
            target
                .file_module_path(Path::new("src/lib.rs"))
                .segments
                .is_empty()
        );
        assert_eq!(
            path_display(&target.file_module_path(Path::new("src/nested/mod.rs"))),
            "nested"
        );
    }

    #[test]
    fn discover_fixture_targets() {
        let fixture_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/tests/module-path");
        let manifest = fixture_root.join("Cargo.toml");
        if !manifest.is_file() {
            return;
        }

        let targets = TargetRoot::discover(&manifest, &fixture_root).expect("discover targets");
        let roots: Vec<_> = targets
            .iter()
            .map(|target| target.root_file.clone())
            .collect();
        assert!(roots.contains(&PathBuf::from("src/lib.rs")));
        assert!(roots.contains(&PathBuf::from("src/main.rs")));
        assert!(roots.contains(&PathBuf::from("tests/integration.rs")));
        assert!(roots.contains(&PathBuf::from("examples/demo.rs")));
        assert!(roots.contains(&PathBuf::from("benches/bench.rs")));
    }
}
