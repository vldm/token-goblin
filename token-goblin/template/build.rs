use std::path::Path;
use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let source_hash =
        std::env::var("TOKEN_GOBLIN_SOURCE_HASH").expect("TOKEN_GOBLIN_SOURCE_HASH is not set");

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
    bytes.extend_from_slice(format!("source-hash: {source_hash}\n").as_bytes());
    bytes.push(b'\0');

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    let path = Path::new(&out_dir).join("meta.rs");
    let escaped = escape_for_rust_byte_str(&bytes);

    std::fs::write(
        &path,
        format!("pub static META: &[u8] = b\"{escaped}\";"),
    )
    .expect("failed to write generated metadata");
}

fn escape_for_rust_byte_str(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| match byte {
            b'\\' => "\\\\".to_string(),
            b'"' => "\\\"".to_string(),
            b'\n' => "\\n".to_string(),
            b'\r' => "\\r".to_string(),
            b'\t' => "\\t".to_string(),
            b'\0' => "\\0".to_string(),
            0x20..=0x7e => (byte as char).to_string(),
            _ => format!("\\x{byte:02x}"),
        })
        .collect()
}
