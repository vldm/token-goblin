//! Materialize generated dylib crates from checked-in templates.
//!
//! Simple line based template engine:
//! - Find `MARKER` in line and replace whole line with

use std::{
    fmt::{self, Debug},
    path::{Path, PathBuf},
};

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{
    Result,
    dylib::GeneratedCrate,
    metadata::{Dependency, Metadata, ValueOrWorkspace},
    path::FsLockGuard,
    syn_items,
};

const MARKER: &str = "goblin-stencil:";
const TOKEN_GOBLIN_LOCK_FILE: &str = "token-goblin.lock";
/// Values substituted into template marker lines.
pub struct TemplateContext {
    pub package_name: String,
    pub package_extra: String,
    pub source_metadata: Metadata,
    // Entries of generated module.
    pub entries: Vec<syn_items::ItemFn>,

    // Content of generated module.
    pub generated_content: TokenStream,

    pub mod_name: Option<(syn::Visibility, syn::Ident)>,
}
impl TemplateContext {
    pub fn name_span(&self) -> proc_macro2::Span {
        if let Some((_, name)) = &self.mod_name {
            name.span()
        } else {
            self.entries[0].sig.ident.span()
        }
    }
    pub fn entries(&self) -> String {
        let entries = self
            .entries
            .iter()
            .map(|entry| {
                let name = &entry.sig.ident;
                let lit = syn::LitStr::new(name.to_string().as_str(), name.span());
                quote! {
                    #lit => {
                        impls::#name(input)
                    }
                }
            })
            .collect::<Vec<_>>();
        quote! {
           match macro_name {
               #(#entries)*
               _ => panic!("BUG: Unexpected macro name: {macro_name}"),
           }
        }
        .to_string()
    }
    pub fn content(&self) -> String {
        self.generated_content.to_string()
    }
}
// custom because syn::Visibility doesn't implement Debug
impl Debug for TemplateContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TemplateContext")
            .field("package_name", &self.package_name)
            .field("package_extra", &self.package_extra)
            .field("source_metadata", &self.source_metadata)
            .field("entries", &self.entries.len())
            .field("generated_content", &"<skipped>")
            .field(
                "mod_name",
                &self
                    .mod_name
                    .as_ref()
                    .map(|(vis, name)| (vis.to_token_stream(), name.to_string())),
            )
            .finish()?;
        Ok(())
    }
}

/// Root directory of the checked-in crate template.
pub fn template_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/template"))
}

/// Stable hash of all template inputs that affect generated crate contents.
/// Used to ensure that our macro declaration and caller code expect to call the same macro.
pub fn source_hash(context: &TemplateContext) -> Result<String> {
    let dependencies = render_dependencies(&context.source_metadata)?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"package_name\0");
    hasher.update(context.package_name.as_bytes());
    hasher.update(b"\0package_extra\0");
    hasher.update(context.package_extra.as_bytes());
    hasher.update(b"\0dependencies\0");
    hasher.update(dependencies.as_bytes());
    hasher.update(b"\0entry\0");
    hasher.update(context.entries().as_bytes());
    hasher.update(b"\0impls\0");
    hasher.update(context.content().as_bytes());
    Ok(hasher.finalize().to_hex().to_string())
}

/// Render the dylib crate template into `output_dir`.
pub fn render_crate(
    output_dir: &Path,
    context: &TemplateContext,
    per_project_cache: bool,
    include_source_hash: bool,
) -> Result<GeneratedCrate> {
    let source_hash = source_hash(context)?;

    let mut output_dir = output_dir.to_path_buf();
    if include_source_hash {
        output_dir.push(&source_hash);
    }
    debug!("rendering crate into {}", output_dir.display());
    debug!("context: {:?}", context);

    let lock_file = FsLockGuard::new(output_dir.join(TOKEN_GOBLIN_LOCK_FILE))?;
    let template_dir = template_root();
    render_template_tree(&template_dir, &output_dir, context)?;

    Ok(GeneratedCrate::new(
        output_dir,
        per_project_cache,
        context.package_name.clone(),
        source_hash,
        lock_file,
    ))
}

fn render_template_tree(
    template_dir: &Path,
    output_dir: &Path,
    context: &TemplateContext,
) -> Result<()> {
    for entry in std::fs::read_dir(template_dir).map_err(|e| {
        error!(
            "failed to read template dir {}: {e}",
            template_dir.display()
        )
    })? {
        let entry = entry.map_err(|e| {
            error!(
                "failed to read template entry in {}: {e}",
                template_dir.display()
            )
        })?;
        let src = entry.path();
        let rel = entry.file_name();
        let dst = output_dir.join(rel);

        if src.is_dir() {
            std::fs::create_dir_all(&dst)
                .map_err(|e| error!("failed to create {}: {e}", dst.display()))?;
            render_template_tree(&src, &dst, context)?;
            continue;
        }

        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| error!("failed to create {}: {e}", parent.display()))?;
        }

        let rendered = render_file(&src, context)?;
        std::fs::write(&dst, rendered)
            .map_err(|e| error!("failed to write {}: {e}", dst.display()))?;
    }

    Ok(())
}

/// Render `build-dependencies` from project metadata into `[dependencies]` TOML.
fn render_dependencies(metadata: &Metadata) -> Result<String> {
    metadata
        .dependencies
        .iter()
        .map(render_dependency)
        .collect::<Result<Vec<_>>>()
        .map(|lines| lines.join("\n"))
}

fn render_dependency(dep: &Dependency) -> Result<String> {
    match &dep.value {
        ValueOrWorkspace::Value(value) => render_value_dependency(&dep.name, value, &dep.rel_path),
        ValueOrWorkspace::Workspace { .. } => Err(error!(
            "dependency `{}` still uses unresolved workspace inheritance",
            dep.name
        )),
    }
}

fn render_value_dependency(
    name: &str,
    value: &toml::Value,
    manifest_path: &Path,
) -> Result<String> {
    let manifest_dir = manifest_path
        .parent()
        .ok_or_else(|| error!("manifest path has no parent: {}", manifest_path.display()))?;

    let value = rewrite_dependency_paths(value, manifest_dir, name)?;
    Ok(format!(
        "{name} = {{ {} }}",
        toml::to_string(&value).map_err(|e| error!("{e}"))?
    ))
}

// Replace relative paths to absolute paths
fn rewrite_dependency_paths(
    value: &toml::Value,
    manifest_dir: &Path,
    name: &str,
) -> Result<toml::Value> {
    let mut value = value.clone();
    let toml::Value::Table(table) = &mut value else {
        return Ok(value);
    };

    let Some(path) = table.get("path") else {
        return Ok(value);
    };
    let Some(path) = path.as_str() else {
        return Err(error!("dependency `{name}` path must be a string"));
    };

    let absolute = manifest_dir.join(path);
    let absolute = absolute.canonicalize().unwrap_or(absolute);
    table.insert(
        "path".to_string(),
        toml::Value::String(absolute.display().to_string()),
    );
    Ok(value)
}

fn render_file(path: &Path, context: &TemplateContext) -> Result<String> {
    let file = std::fs::read_to_string(path)
        .map_err(|e| error!("failed to read template {}: {e}", path.display()))?;

    let mut out = Vec::new();
    for line in file.lines() {
        let Some(key) = extract_marker(line) else {
            out.push(line.to_string());
            continue;
        };

        match key.as_str() {
            "package.name" => out.push(format!("name = \"{}\"", &context.package_name)),
            "package.extra" => push_fragment(&mut out, &context.package_extra),
            "dependencies" => {
                push_fragment(&mut out, &render_dependencies(&context.source_metadata)?);
            }

            "entries" => push_fragment(&mut out, &context.entries()),
            "content" => push_fragment(&mut out, &context.content()),
            other => {
                return Err(error!(
                    "unknown stencil marker `{other}` in {}",
                    path.display()
                ));
            }
        }
    }

    Ok(out.join("\n"))
}

fn extract_marker(line: &str) -> Option<String> {
    let idx = line.find(MARKER)?;
    let key = line[idx + MARKER.len()..].trim();
    if key.is_empty() {
        return None;
    }
    Some(key.to_string())
}

fn push_fragment(out: &mut Vec<String>, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    out.extend(fragment.lines().map(str::to_string));
}
