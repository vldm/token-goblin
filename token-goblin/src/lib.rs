// #![allow(unused)]
use proc_macro::TokenStream;
#[macro_use]
mod errors;
#[macro_use]
mod timings;

type Result<T, E = syn::Error> = std::result::Result<T, E>;

mod dylib;
mod ide_support;
mod macro_impl;
mod metadata;
mod path;
mod rust_mod_fs;
mod rustc_meta;
mod span_recovery;
mod syn_items;
mod template;

use errors::MapCompileError;

/// Set to 'true' to enable debug prints.
#[allow(unexpected_cfgs, reason = "custom made config")]
pub(crate) const DEBUG: bool = cfg!(token_goblin_debug);

/// Set to 'true' to enable printing of timings.
/// (Also requires `DEBUG` to be enabled, see above)
#[allow(unexpected_cfgs, reason = "custom made config")]
pub(crate) const PRINT_TIMINGS: bool = cfg!(token_goblin_print_timings);

/// Set to 'true' to enable debug prints of environment variables.
/// (Also requires `DEBUG` to be enabled)
///
/// Internal only feature, not exposed to the user.
pub(crate) const DEBUG_ENV: bool = true;

/// Internal feature that prevent cache checking for dylib.
pub(crate) const NO_CACHE: bool = false;

// ===============================
// Macros entry points
// ===============================

///
/// This is an internal macro, used to proxy macro expansion calls to the real code in dylib.
///
#[proc_macro]
pub fn proxy(input: TokenStream) -> TokenStream {
    timed!("proxy", {
        macro_impl::proxy_impl(input.into())
            .map_compile_error()
            .into()
    })
}

///
///  Munch your declaration and produce a new macro.
/// ```
/// #[token_goblin::munch]
/// mod my_module {
///   // entry fn should be public
///   pub fn inner_function(_: TokenStream) -> TokenStream {
///     //..
///     # todo!()
///   }
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
    let out: TokenStream = timed!("munch", {
        macro_impl::munch_impl(attr.into(), item.into())
            .map_compile_error()
            .into()
    });
    out
}

#[proc_macro_derive(Snif, attributes(snif))]
pub fn snif(input: TokenStream) -> TokenStream {
    timed!("snif", {
        // TODO
        input
    })
}
///
/// Adaptor to function-like macro, that allows using them as derive macro.
///
/// ```
/// #[derive(token_goblin::Spit)]
/// #[charm(path_to_macro)]
/// struct MyStruct {
///   field: i32,
/// }
/// ```
/// will expand to:
/// ```
#[proc_macro_derive(Spit, attributes(charm))]
pub fn spit_derive(input: TokenStream) -> TokenStream {
    timed!("spit_derive", {
        macro_impl::spit_derive_impl(input.into())
            .map_compile_error()
            .into()
    })
}

#[proc_macro_attribute]
pub fn spit(attr: TokenStream, item: TokenStream) -> TokenStream {
    timed!("spit", {
        macro_impl::spit_impl(attr.into(), item.into())
            .map_compile_error()
            .into()
    })
}
