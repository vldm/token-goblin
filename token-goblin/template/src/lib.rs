// Prelude
#[allow(unused)]
use token_goblin_runtime::prelude::*;

mod impls;

static META: &[u8] = concat!(env!("TOKEN_GOBLIN_RUSTC_META"), "\0").as_bytes();

#[unsafe(no_mangle)]
pub extern "C" fn meta() -> *const std::ffi::c_char {
    META.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub fn entry(macro_name: &str, input: &str) -> token_goblin_runtime::Output {
    token_goblin_runtime::entry(input, |input| {
        // goblin-stencil: entries
    })
}
