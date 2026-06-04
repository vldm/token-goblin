use proc_macro2::TokenStream;
use quote::quote;
use std::env;
use std::error::Error;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
// compile dylib-test crate
fn compile_dylib(manifest_path: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    cmd.arg("--manifest-path");
    cmd.arg(manifest_path);
    cmd.arg("--target-dir");
    cmd.arg(out_dir);
    cmd.arg("--release");
    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("cargo build failed: {status}").into());
    }
    cmd.output()?;
    Ok(())
}

fn run_entry(dylib_path: &Path, token_stream: TokenStream) -> Result<TokenStream, Box<dyn Error>> {
    let dylib = unsafe { libloading::Library::new(dylib_path)? };
    let entry: libloading::Symbol<fn(TokenStream) -> TokenStream> = unsafe { dylib.get(b"entry")? };
    Ok(entry(token_stream))
}

fn main() -> Result<(), Box<dyn Error>> {
    let workspace_path: PathBuf = format!("{}/../", env!("CARGO_MANIFEST_DIR")).into();

    let manifest_path: PathBuf = workspace_path.join("dylib-test/Cargo.toml");
    let out_dir: PathBuf = workspace_path.join("target/");

    compile_dylib(&manifest_path, &out_dir)?;

    let dylib_path: PathBuf = out_dir.join("release").join(format!(
        "{}dylib_test{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX,
    ));
    let token_stream: TokenStream = quote! {
        foo bar
    };
    let result: TokenStream = run_entry(&dylib_path, token_stream)?;
    println!("{result}");
    Ok(())
}
