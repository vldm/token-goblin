#![allow(unused)]
use proc_macro::TokenStream;
#[macro_use]
mod errors;

type Result<T, E = syn::Error> = std::result::Result<T, E>;

mod dylib;
mod macro_impl;
mod metadata;
mod path;
mod rustc_meta;
mod template;
use errors::MapCompileError;

/// Set to 'true' to enable debug prints.
#[allow(unexpected_cfgs, reason = "custom made config")]
pub(crate) const DEBUG: bool = true || cfg!(token_goblin_debug);

// ===============================
// Macros entry points
// ===============================

///
/// This is an internal macro, used to proxy macro expansion calls to the real code in dylib.
///
#[proc_macro]
pub fn proxy(input: TokenStream) -> TokenStream {
    macro_impl::proxy_impl(input.into())
        .map_compile_error()
        .into()
}

///
///  Munch your declaration and produce a new macro.
/// ```
/// #[token_goblin::munch]
/// mod my_module {
///   // entry fn
/// }
/// ```
/// or for function:
/// ```
/// #[token_goblin::munch]
/// fn my_function(_: TokenStream) -> TokenStream {
///   //..
///  # todo!()
/// }
/// ```
/// `munch` macro will expand to one or more macro definitions:
/// ```
/// macro_rules! my_function {
///   ($($args:tt)*) => {
///     //..
///   };
/// }
/// ```
#[proc_macro_attribute]
pub fn munch(attr: TokenStream, item: TokenStream) -> TokenStream {
    macro_impl::munch_impl(attr.into(), item.into())
        .map_compile_error()
        .into()
}

#[proc_macro_derive(Snif)]
pub fn snif(input: TokenStream) -> TokenStream {
    input
}
