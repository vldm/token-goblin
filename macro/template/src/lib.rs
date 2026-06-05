// Prelude
use token_goblin_runtime::*;

// use module to simplify debugging of invalid codegen
mod generated_rustc_meta {
    include!(concat!(env!("OUT_DIR"), "/rustc_meta.rs"));
}

mod impls;

#[unsafe(no_mangle)]
pub extern "C" fn rustc_version() -> *const std::ffi::c_char {
    generated_rustc_meta::RUSTC_META.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub fn entry(input: TokenStream) -> TokenStream {
    // 1. catch_unwind?
    //
    // 2. match first ident as entry branch
    // let (macro_name, input) = split_first(input);
    // match macro_name {
    //  "attr_like"     => { let (attr, impl,) = input.split(); impls::attr_like(attr.convert()?, impl.convert()?)},
    //  "function_like" => { let (input,) = input.split(); impls::function_like(input.convert()?)},
    //  "custom_types"  => { let (c,) = input.split(); impls::custom_types(c.convert()?)},
    //   v              => error!("Goblin proxy error: unexpected macro name: {v}"),
    // }
    // goblin-stencil: entry
}
