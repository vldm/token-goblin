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
pub(crate) const DEBUG_ENV: bool = false;

/// Internal feature that prevent cache checking for dylib.
pub(crate) const NO_CACHE: bool = false;

// ===============================
// Macros entry points
// ===============================

///
/// This is an internal macro, used to proxy macro expansion calls to the real code in dylib.
///
#[proc_macro]
#[doc(hidden)]
pub fn proxy(input: TokenStream) -> TokenStream {
    timed!("proxy", {
        macro_impl::proxy_impl(input.into())
            .map_compile_error()
            .into()
    })
}

///
/// Munches your declaration and produces a new macro or `charm`.
///
/// Use it for module:
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
/// `munch` macro will expand to macro definitions like this:
/// ```
/// macro_rules! my_function {
///   ($($args:tt)*) => {
///     //..
///   };
/// }
/// ```
#[proc_macro_attribute]
pub fn munch(attr: TokenStream, item: TokenStream) -> TokenStream {
    let result = timed!("munch", {
        macro_impl::munch_impl(attr.into(), item.into())
            .map_compile_error()
            .into()
    });
    debug!("munch result: {result}");
    result
}

/// Goblin sniffs your item, and stores knowledge about it for future use.
///
/// Then it can be passed to other macros.
///
/// Example:
/// ```
/// #[token_goblin::derive_snif]
/// struct MyStruct {
///   field: i32,
/// }
/// ```
///
/// And use it like this:
/// ```
/// token_goblinsnif!(MyStruct in some_macro!("extra tokens") )
/// ```
///
/// This will feed declaration of `MyStruct` as group of tokens inside `{..}`
/// to `some_macro!`, and pass `"extra tokens"` as arguments before it.
/// ```
/// some_macro!( "extra tokens" { struct MyStruct { field : i32, } })
/// ```
///
#[proc_macro_derive(Snif, attributes(snif))]
pub fn derive_snif_impl(input: TokenStream) -> TokenStream {
    timed!("derive_snif", {
        macro_impl::snif_impl(input.into())
            .map_compile_error()
            .into()
    })
}

/// The version of `#[derive(Snif)]` that can be used as attribute for any item, not only struct/union/enum.
///
/// Use
/// ```
/// #[token_goblin::derive_snif]
/// fn my_function(_: TokenStream) -> TokenStream {
///   //..
///  # todo!()
/// }
/// ```
#[proc_macro_attribute]
pub fn derive_snif(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let resulted_macro = timed!("derive_snif_attr", {
        macro_impl::snif_impl(input.clone()).map_compile_error()
    });
    // Since attribute macro consumes tokens, return original input as well.
    // Note: if macro returns `Err` - original input will persist as well.
    quote::quote! {
        #input
        #resulted_macro
    }
    .into()
}

/// Token goblin snif your item and vanish it.
/// So only knowledge about it is left in token-goblin's memory.
///
/// Useful as alternative to `derive_snif` when you don't need the item itself.
///
/// ```
/// #[token_goblin::vanish]
/// struct MyStruct {
///   field: i32,
/// }
/// // let x = MyStruct { field: 42 }; // will fail to compile, since no MyStruct is available anymore.
/// ```
///
/// But you can generate it at any time, by calling generated `MyStruct!` macro.
/// ```
/// /// #[token_goblin::vanish]
/// # struct MyStruct {
/// #  field: i32,
/// # }
/// MyStruct!{}
/// // will expand to:
/// // struct MyStruct {
/// //  field: 42,
/// // }
/// // and this will succesfully compile
/// let x = MyStruct { field: 42 };
/// ```
///
#[proc_macro_attribute]
pub fn vanish(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let resulted_macro = timed!("vanish", {
        macro_impl::snif_impl(input.clone()).map_compile_error()
    });
    quote::quote! {
        #resulted_macro
    }
    .into()
}

/// Ask token goblin to share knowledge about item in macro.
///
#[proc_macro]
pub fn snif(input: TokenStream) -> TokenStream {
    timed!("snif", {
        macro_impl::snif_expand_impl(input.into())
            .map_compile_error()
            .into()
    })
}

///
/// Adaptor to function-like macro, that allows using them as derive macro.
///
/// ```
/// # macro_rules! path_to_macro {
/// #   ($($tt:tt)*) => { }
/// # }
/// #[derive(token_goblin::Spit)]
/// #[charm(path_to_macro)]
/// struct MyStruct {
///   field: i32,
/// }
/// ```
/// will expand to:
/// ```
/// # macro_rules! path_to_macro {
/// #   ($($tt:tt)*) => { }
/// # }
/// struct MyStruct {
///   field: i32,
/// }
/// path_to_macro!(MyStruct { field: 42 });
/// ```
#[proc_macro_derive(Spit, attributes(charm))]
pub fn derive_spit(input: TokenStream) -> TokenStream {
    timed!("derive_spit", {
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
