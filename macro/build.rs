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

    let meta = String::from_utf8(output.stdout).expect("rustc -vV stdout is not valid UTF-8");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    let path = Path::new(&out_dir).join("runner_rustc_meta.rs");
    let escaped = escape_for_rust_str(&meta);
    std::fs::write(
        &path,
        format!("pub const RUSTC_META: &str = \"{escaped}\";"),
    )
    .expect("failed to write runner rustc metadata");
}

fn escape_for_rust_str(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\\' => "\\\\".to_string(),
            '"' => "\\\"".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            ch if ch.is_control() => format!("\\x{:02x}", ch as u8),
            ch => ch.to_string(),
        })
        .collect()
}
