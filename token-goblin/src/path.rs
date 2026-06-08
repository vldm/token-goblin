use std::{
    fs::{File, TryLockError},
    path::{Path, PathBuf},
};

use proc_macro2::Span;

use crate::Result;

pub(crate) const OUT_DIR: &str = env!("OUT_DIR");

/// Use `CARGO_MANIFEST_DIR` to get path to crate root.
/// It might be unset if custom build system is used.
pub fn crate_root() -> Result<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| error!(Span::call_site() => "CARGO_MANIFEST_DIR is not set"))?;
    let manifest_dir = PathBuf::from(&manifest_dir);
    Ok(manifest_dir)
}

/// Cache build directory
/// Single common directory for all macros:
/// - This avoid rebuilding of same crates like `syn`, `proc-macro2` for each macro call.
/// - But on the other hand, it prevent parallel build of multiple macros
///
/// Returns:
/// - If `per_project_cache` is `true`, returns `project_dir/build_cache`.
/// - If `per_project_cache` is `false`, returns `OUT_DIR/build_cache`.
pub fn build_dir(project_dir: &Path, per_project_cache: bool) -> PathBuf {
    if per_project_cache {
        return project_dir.join("build_cache");
    }
    PathBuf::from(OUT_DIR).join("build_cache")
}

/// Use `CARGO_MANIFEST_PATH` to get path to Cargo.toml:
/// 1. It might be file in `crate_root()`, or separate file, if custom build system is used.
/// 2. It might be manifest merged with workspace manifest.
/// 3. Or might be missing, if custom build system didn't use Cargo.toml.
pub fn manifest_path() -> Result<PathBuf> {
    let manifest_path = std::env::var("CARGO_MANIFEST_PATH")
        .map_err(|_| error!(Span::call_site() => "CARGO_MANIFEST_PATH is not set"))?;
    let manifest_path = PathBuf::from(&manifest_path);
    Ok(manifest_path)
}

/// Recursively try search parent folders for `Cargo.toml`.
/// Stops when extract function returns `Some` or error
pub fn search_for_parent_manifest<U>(
    path: &Path,
    extract: impl Fn(&Path) -> Result<Option<U>>,
) -> Result<U> {
    let mut prev = path;
    while let Some(path) = prev.parent() {
        prev = path;

        let try_path = path.join("Cargo.toml");
        // If file doesn't exist it is not an error, try parent folder
        if !try_path.exists() {
            continue;
        }
        let result = extract(&try_path)?;
        if let Some(value) = result {
            return Ok(value);
        }
    }

    Err(error!(Span::call_site() => "No parent directory found"))
}

/// Use source span to request path of macro definition.
///
/// Returns:
/// - Path to generated crate
/// - Whether this path is local (not remapped, generated, etc)
///
/// Format of path is:
/// `{OUT_DIR}/generated/{crate_name}_{crate_version}/{path_to_macro_definition}_{fn_name}_{line}_{column}`
pub fn calculate_generated_path(ident: &syn::Ident) -> (PathBuf, bool) {
    let fn_name = ident.to_string();
    let span: proc_macro::Span = ident.span().unwrap();

    let mut stable = true;
    let file = span.local_file().map_or_else(
        || {
            stable = false;
            span.file()
        },
        |v| v.display().to_string(),
    );

    let crate_name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| {
        stable = false;
        "unknown".to_string()
    });
    let crate_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| {
        stable = false;
        "unknown".to_string()
    });
    let file = sanitize_path(&file);
    let line = span.line();
    let column = span.column();
    let path = format!("{OUT_DIR}/generated/{crate_name}_{crate_version}/{file}_{line}_{column}");
    (PathBuf::from(path), stable)
}

fn sanitize_path(path: &str) -> String {
    path.replace(['\\', '/'], "_")
}

#[derive(Debug)]
pub struct FsLockGuard {
    path: PathBuf,
    file: File,
}

impl FsLockGuard {
    pub fn new(path: PathBuf) -> Result<Self> {
        let file = File::create(&path)
            .map_err(|e| error!(Span::call_site() => "Failed to create lock file: {e}"))?;

        let this = Self { path, file };
        this.lock()?;
        Ok(this)
    }
    fn lock(&self) -> Result<()> {
        match self.file.try_lock() {
            Err(TryLockError::WouldBlock) => {
                debug!("Waiting for lock file: {}", self.path.display());
                self.file.lock()
            }
            Err(TryLockError::Error(e)) => Err(e),
            Ok(()) => Ok(()),
        }
        .map_err(|e| error!(Span::call_site() => "Failed to lock lock file: {e}"))
    }
    fn unlock(&self) -> Result<()> {
        self.file
            .unlock()
            .map_err(|e| error!(Span::call_site() => "Failed to unlock lock file: {e}"))
    }
}

impl Drop for FsLockGuard {
    fn drop(&mut self) {
        self.unlock().unwrap();
    }
}
