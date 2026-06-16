// Prelude
#[allow(unused)]
use token_goblin_runtime::prelude::*;

mod impls;

static META: &[u8] = concat!(env!("TOKEN_GOBLIN_RUSTC_META"), "\0").as_bytes();

/// We use this entrypoint to check compatibility of dylib itself.
#[unsafe(no_mangle)]
pub extern "C" fn meta() -> *const std::ffi::c_char {
    META.as_ptr().cast()
}

/// This function is called after checking rustc version and other metadata,
/// so we assume that using Rust ABI is safe.
// TODO: It still might be problem if proc-macro and dylib uses different alocators, but let it keep until first bugreport.
#[unsafe(no_mangle)]
pub fn entry(macro_name: &str, input: &str) -> token_goblin_runtime::Output {
    token_goblin_runtime::entry(input, |input| {
        // goblin-stencil: entries
    })
}
