use std::path::{Path, PathBuf};

use proc_macro2::Span;

use crate::Result;

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
pub fn build_dir(project_dir: &PathBuf, per_project_cache: bool) -> Result<PathBuf> {
    if per_project_cache {
        return Ok(project_dir.join("build_cache"));
    }
    let build_cache = PathBuf::from(crate::OUT_DIR).join("build_cache");
    let build_cache = PathBuf::from(&build_cache);
    Ok(build_cache)
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
