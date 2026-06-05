//! Runner-side rustc metadata captured at proc-macro crate build time.

use proc_macro2::Span;

mod runner {
    include!(concat!(env!("OUT_DIR"), "/runner_rustc_meta.rs"));
}

/// Full `rustc -vV` output for the compiler that built the proc-macro runner.
pub const RUSTC_META: &str = runner::RUSTC_META;

fn expected_meta(source_hash: &str) -> String {
    format!("{RUSTC_META}source-hash: {source_hash}\n")
}

/// Compare dylib-exported metadata with the runner metadata and source hash.
pub fn ensure_compatible(lib_meta: &str, source_hash: &str) -> crate::Result<()> {
    let expected = expected_meta(source_hash);
    debug!("lib_meta: {lib_meta}");
    debug!("expected_meta: {expected}");
    if lib_meta == expected {
        return Ok(());
    }

    Err(error!(
        Span::call_site() =>
        "dylib metadata does not match proc-macro runner; rebuild the crate or use the same toolchain/source\n\
         expected:\n{expected}\
         dylib:\n{lib_meta}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn accepts_matching_metadata() {
        let expected = expected_meta(TEST_HASH);
        ensure_compatible(&expected, TEST_HASH).expect("matching metadata should be accepted");
    }

    #[test]
    fn accepts_matching_metadata_after_roundtrip() {
        let expected = expected_meta(TEST_HASH);
        let mut meta = expected.as_bytes().to_vec();
        meta.push(b'\0');
        let cstr = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(&meta) };
        let meta = cstr.to_str().unwrap();
        ensure_compatible(meta, TEST_HASH).expect("matching metadata should be accepted");
    }

    #[test]
    fn rejects_different_rustc_metadata() {
        let err = ensure_compatible(
            "rustc 0.0.0 (000000000 0000-00-00)\nsource-hash: abc\n",
            "abc",
        )
        .expect_err("different rustc metadata should be rejected");
        assert!(err.to_string().contains("does not match"));
    }

    #[test]
    fn rejects_different_source_hash() {
        let expected = expected_meta(TEST_HASH);
        let err = ensure_compatible(&expected, "deadbeef")
            .expect_err("different source hash should be rejected");
        assert!(err.to_string().contains("does not match"));
    }
}
