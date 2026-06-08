use std::path::Path;
use std::process::Command;

// As side-effect of running build.rs cargo set `OUT_DIR` which is also used.
fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    println!("cargo:rustc-env=TOKEN_GOBLIN_RUSTC={rustc}");

    let output = Command::new(&rustc)
        .arg("-vV")
        .output()
        .expect("failed to run `rustc -vV`");
    assert!(
        output.status.success(),
        "`rustc -vV` failed with status {}",
        output.status
    );

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    let path = Path::new(&out_dir).join("rustc_meta.out");
    std::fs::write(&path, &output.stdout).expect("failed to write rustc metadata");
}
