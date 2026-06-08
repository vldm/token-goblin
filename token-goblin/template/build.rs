use std::path::Path;
use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());

    let output = Command::new(&rustc)
        .arg("-vV")
        .output()
        .expect("failed to run `rustc -vV`");
    assert!(
        output.status.success(),
        "`rustc -vV` failed with status {}",
        output.status
    );

    let mut bytes = output.stdout;
    bytes.push(b'\0');

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    let path = Path::new(&out_dir).join("rustc_meta.out");
    std::fs::write(&path, &bytes).expect("failed to write rustc metadata");
}
