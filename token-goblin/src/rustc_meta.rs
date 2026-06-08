//! Runner-side rustc metadata captured at proc-macro crate build time.

use proc_macro2::Span;

/// Full `rustc -vV` output for the compiler that built the proc-macro runner.
pub const RUSTC_META: &str = include_str!(concat!(env!("OUT_DIR"), "/rustc_meta.out"));

/// Compare dylib-exported metadata with the runner metadata.
pub fn ensure_compatible(lib_meta: &str) -> crate::Result<()> {
    // debug!("lib_meta: {lib_meta}");
    // debug!("expected_meta: {RUSTC_META}");
    if lib_meta == RUSTC_META {
        return Ok(());
    }

    Err(error!(
        Span::call_site() =>
        "dylib metadata does not match proc-macro runner; rebuild the crate or use the same toolchain\n\
         expected:\n{RUSTC_META}\
         dylib:\n{lib_meta}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_matching_metadata() {
        ensure_compatible(RUSTC_META).expect("matching metadata should be accepted");
    }

    #[test]
    fn accepts_matching_metadata_after_roundtrip() {
        let mut meta = RUSTC_META.as_bytes().to_vec();
        meta.push(b'\0');
        let cstr = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(&meta) };
        let meta = cstr.to_str().unwrap();
        ensure_compatible(meta).expect("matching metadata should be accepted");
    }

    #[test]
    fn rejects_different_rustc_metadata() {
        let err = ensure_compatible("rustc 0.0.0 (000000000 0000-00-00)\n")
            .expect_err("different rustc metadata should be rejected");
        assert!(err.to_string().contains("does not match"));
    }
}
