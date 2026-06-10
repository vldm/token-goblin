//! Build and dynamically load generated dylib crates.
//!
//! This module provides the low-level routines used by the `munch` and `proxy`
//! macro phases. Template materialization is handled elsewhere; callers pass in
//! paths to an already-generated crate tree.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use proc_macro2::{Span, TokenStream};

use crate::{Result, path, rustc_meta, span_recovery};

/// Cargo build profile for the generated dylib crate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BuildProfile {
    #[cfg_attr(not(feature = "use_release_profile"), default)]
    Debug,

    #[cfg_attr(feature = "use_release_profile", default)]
    Release,
}
impl FromStr for BuildProfile {
    type Err = syn::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "debug" => Ok(BuildProfile::Debug),
            "release" => Ok(BuildProfile::Release),
            _ => Err(error!(Span::call_site() => "Unknown build profile: {}", s)),
        }
    }
}

impl BuildProfile {
    fn subdir(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    fn cargo_release_flag(self) -> Option<&'static str> {
        match self {
            Self::Debug => None,
            Self::Release => Some("--release"),
        }
    }
}

// pub struct Arguments {
//     pub macro_name: String,
//     pub input: TokenStream,
// }

/// Collected information about generated crate, that is ready to be compiled.
#[derive(Debug)]
pub struct GeneratedCrate {
    /// Root directory of the generated crate
    /// Used to calculate `build-dir`, `target-dir`, and provide path to `Cargo.toml`.
    pub source_dir: PathBuf,
    /// Build cache directory for generated crate artifacts.
    /// Can be shared between multiple macros, or used for per-project cache.
    pub build_dir: PathBuf,
    /// Package name from the generated `Cargo.toml` `[package].name`.
    /// Used to calculate `dylib_path`.
    pub crate_name: String,
    /// Stable hash of template inputs used to build this crate,
    /// encoded in hex without `0x` prefix.
    pub source_hash: String,

    /// Lock file path used to prevent concurrent builds of the same crate.
    pub _lock_file: path::FsLockGuard,
}

impl GeneratedCrate {
    pub fn new(
        source_dir: PathBuf,
        per_project_cache: bool,
        crate_name: impl Into<String>,
        source_hash: impl Into<String>,
        lock_file: path::FsLockGuard,
    ) -> Self {
        let source_hash = source_hash.into();

        let build_dir = path::build_dir(&source_dir, per_project_cache);
        Self {
            source_dir,
            build_dir,
            crate_name: crate_name.into(),
            source_hash,
            _lock_file: lock_file,
        }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.source_dir.join("Cargo.toml")
    }
    pub fn target_dir(&self) -> PathBuf {
        self.source_dir.join("target")
    }

    // Path to versioned dylib in target directory.
    pub fn dylib_path(&self, profile: BuildProfile) -> PathBuf {
        let versioned_crate_name = format!("{}_{}", self.crate_name, self.source_hash);
        dylib_path(&self.target_dir(), profile, &versioned_crate_name)
    }

    // Path to source dylib in target directory.
    pub fn dylib_src_path(&self, profile: BuildProfile) -> PathBuf {
        dylib_path(&self.target_dir(), profile, &self.crate_name)
    }
}

/// Result of a successful dylib compilation.
#[derive(Clone, Debug)]
pub struct DylibBuild {
    pub dylib_path: PathBuf,
}

/// Normalize a Cargo package name to the library artifact stem (`-` → `_`).
pub fn cargo_crate_name(package_name: &str) -> String {
    package_name.replace('-', "_")
}

/// Resolve the dylib filename for a package name at the current platform.
pub fn dylib_filename(package_name: &str) -> String {
    format!(
        "{}{}{}",
        std::env::consts::DLL_PREFIX,
        cargo_crate_name(package_name),
        std::env::consts::DLL_SUFFIX,
    )
}

/// Resolve the expected dylib path under a build cache directory.
pub fn dylib_path(target_dir: &Path, profile: BuildProfile, package_name: &str) -> PathBuf {
    target_dir
        .join(profile.subdir())
        .join(dylib_filename(package_name))
}

fn cargo_command() -> Command {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    Command::new(cargo)
}

/// Check if dylib is exists (and was built with compatible toolchain)
fn check_cached_dylib(generated: &GeneratedCrate, profile: BuildProfile) -> Option<DylibBuild> {
    let dylib_path = generated.dylib_path(profile);
    if crate::NO_CACHE || !dylib_path.is_file() {
        return None;
    }
    if let Err(e) = load_library(&dylib_path) {
        debug!("cached dylib is not valid: {e}");
        return None;
    }
    Some(DylibBuild { dylib_path })
}
// Copy dylib artifact to versioned path
fn copy_dylib_artifact(generated: &GeneratedCrate, profile: BuildProfile) -> Result<PathBuf> {
    let src_path = generated.dylib_src_path(profile);
    let dst_path = generated.dylib_path(profile);
    std::fs::copy(&src_path, &dst_path)
        .map_err(|e| error!(Span::call_site() => "failed to copy dylib: {e}"))?;
    Ok(dst_path)
}

/// Compile a generated crate and return the resolved dylib path.
pub fn compile_crate(generated: &GeneratedCrate, profile: BuildProfile) -> Result<DylibBuild> {
    let manifest_path = generated.manifest_path();
    if !manifest_path.is_file() {
        return Err(error!(
            Span::call_site() =>
            "generated crate manifest not found: {}",
            manifest_path.display()
        ));
    }
    if let Some(dylib) = check_cached_dylib(generated, profile) {
        return Ok(dylib);
    }

    let rustc = env!("TOKEN_GOBLIN_RUSTC");
    let rustc_version = Command::new(rustc).arg("-vV").output().map_err(|e| {
        error!(
            Span::call_site() =>
            "failed to run `rustc -vV` for {}: {e}",
            rustc
        )
    })?;

    let mut cmd = cargo_command();
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--target-dir")
        .arg(generated.target_dir())
        .env("CARGO_BUILD_BUILD_DIR", &generated.build_dir)
        .env("RUSTC", rustc)
        .env(
            "TOKEN_GOBLIN_RUSTC_META",
            String::from_utf8_lossy(&rustc_version.stdout).as_ref(),
        );

    if let Some(flag) = profile.cargo_release_flag() {
        cmd.arg(flag);
    }

    debug!(
        "compiling {} (profile={:?})",
        manifest_path.display(),
        profile
    );

    let output = cmd.output().map_err(|e| {
        error!(
            Span::call_site() =>
            "failed to spawn `cargo build` for {}: {e}",
            manifest_path.display()
        )
    })?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(error!(
            Span::call_site() =>
            "cargo build failed for {} (status={}):\n{stdout}{stderr}",
            manifest_path.display(),
            output.status
        ));
    }

    let dylib_path = generated.dylib_src_path(profile);
    if !dylib_path.is_file() {
        return Err(error!(
            Span::call_site() =>
            "dylib not found after successful build: {}",
            dylib_path.display()
        ));
    }
    let dylib_path = copy_dylib_artifact(generated, profile)?;

    debug!("built {}", dylib_path.display());

    Ok(DylibBuild { dylib_path })
}

type EntryFn = fn(&str) -> span_recovery::Output;
type MetaFn = unsafe extern "C" fn() -> *const c_char;

fn read_dylib_meta(library: &libloading::Library, dylib_path: &Path) -> Result<&'static str> {
    // Safety: we know the type of meta function.
    let meta_fn: libloading::Symbol<MetaFn> = unsafe { library.get(b"meta") }.map_err(|e| {
        error!(
            Span::call_site() =>
            "failed to resolve `meta` symbol in {}: {e}",
            dylib_path.display()
        )
    })?;

    // Safety: return pointer to static string on stable C ABI.
    let ptr = unsafe { meta_fn() };
    if ptr.is_null() {
        return Err(error!(
            Span::call_site() =>
            "`meta` returned null in {}",
            dylib_path.display()
        ));
    }

    // Safety: we know that the pointer is a valid null-terminated C string.
    let meta: &'static CStr = unsafe { CStr::from_ptr(ptr) };
    meta.to_str().map_err(|e| {
        error!(
            Span::call_site() =>
            "`meta` returned invalid UTF-8 in {}: {e}",
            dylib_path.display()
        )
    })
}

/// Load a dylib and return the library handle.
/// Ensure that generated library has compatible rustc metadata.
pub fn load_library(dylib_path: &Path) -> Result<libloading::Library> {
    // Safety: our library doesn't contain any "_start" or "OnLoad" methods.
    let library = unsafe { libloading::Library::new(dylib_path) }.map_err(|e| {
        error!(
            Span::call_site() =>
            "failed to load dylib {}: {e}",
            dylib_path.display()
        )
    })?;
    let lib_meta = read_dylib_meta(&library, dylib_path)?;
    rustc_meta::ensure_compatible(lib_meta)?;

    Ok(library)
}

/// Load a dylib, invoke `entry`, and return the resulting token stream.
#[allow(clippy::needless_pass_by_value, reason = "consume token stream")]
pub fn load_and_run_entry(dylib_path: &Path, input: TokenStream) -> Result<TokenStream> {
    let library = load_library(dylib_path)?;
    let serialized_input = span_recovery::SerializedInput::serialize(&input);

    // Safety: we know the type of entrypoint.
    let entry: libloading::Symbol<EntryFn> = unsafe { library.get(b"entry") }
        .map_err(|e| error!(Span::call_site() => "failed to resolve `entry` symbol: {e}"))?;

    debug!("charm input: {}", serialized_input.source_text);
    let guest = entry(&serialized_input.source_text);
    debug!("charm output: {}", guest.text);

    let res = span_recovery::hydrate(&serialized_input, &guest);

    Ok(res)
}

#[cfg(test)]
mod tests {
    #[test]
    fn runner_rustc_path_is_embedded() {
        let path = env!("TOKEN_GOBLIN_RUSTC");
        assert!(!path.is_empty());
    }
}
