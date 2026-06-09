// Prelude
#[allow(unused)]
use token_goblin_runtime::*;


mod impls;

static META: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rustc_meta.out"));

#[unsafe(no_mangle)]
pub extern "C" fn meta() -> *const std::ffi::c_char {
    META.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub fn entry(input: &str) -> String {
    let input = parse_input(input).expect("invalid serialized input");

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
    let output = {
        // goblin-stencil: entry
    };
    output.to_string()
}
