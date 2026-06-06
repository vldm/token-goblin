// Prelude
#[allow(unused)]
use token_goblin_runtime::*;

mod generated_meta {
    include!(concat!(env!("OUT_DIR"), "/meta.rs"));
}

mod impls;

#[unsafe(no_mangle)]
pub extern "C" fn meta() -> *const std::ffi::c_char {
    generated_meta::META.as_ptr().cast()
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
