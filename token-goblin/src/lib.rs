// #![allow(unused)]
//! `token-goblin` keeps a small goblin that munches your tokens and forges them
//! back into macros (here called `charms`). You feed it declarations, it chews,
//! and spits out something useful. Mostly.
//!
//! ## Forge your charm:
//! ```
//! # use token_goblin::munch;
//! #[munch]
//! fn my_charm(input: TokenStream) -> TokenStream {
//!   // ..
//!  # todo!()
//! }
//! ```
//!
//! That can be later used as `macro` in your code:
//! ```
//! # use token_goblin::munch;
//! # macro_rules! my_charm {
//! #   ($($tt:tt)*) => { }
//! # }
//! my_charm!(foo bar);
//! ```
//!
//! ## Use `syn` types as input, when it needed:
//!
//! ```
//! # use token_goblin::munch;
//! # use syn::Ident;
//! #[munch]
//! fn my_charm(input: Ident) -> TokenStream {
//!  quote!{#input}
//! }
//! let foo = 12;
//! let x = my_charm!(foo);
//! assert_eq!(x, 12);
//! ```
//!
//! ## Emit streamingly, like `println!`:
//! ```
//! # use token_goblin::munch;
//! #[munch]
//! fn add_foo_bar(_: TokenStream) {
//!   // ..
//!   output_str!("42");
//!   output! {
//!     + 53
//!   };
//! }
//! let x = add_foo_bar!();
//! assert_eq!(x, 42 + 53);
//! ```
//!
//! **Hygiene from the box, preventing `charm` to access external variables:**
//! ```
//! # use token_goblin::munch;
//! #[munch]
//! fn my_charm(input: TokenStream) -> TokenStream {
//!   // ..
//!  quote!(foo)
//! }
//! let foo = 12;
//! let x = my_charm!(foo); // Failed: cannot find value `foo` in this scope.
//! assert_eq!(x, 12);
//! ```
//!
//! ## Provide a way to extend your types in future:
//! ```
//! # use token_goblin::Snif;
//! #[derive(Snif)]
//! pub struct MyStruct {
//!   field: i32,
//! }
//! # macro_rules! some_macro {
//! #   ($($tt:tt)*) => { }
//! # }
//!
//! // In other crate user can use it like this:
//! token_goblin::snif!(MyStruct in some_macro!());
//! ```
//!
//! For more docs, and examples checkout [README.md](https://github.com/vldm/token-goblin/blob/master/README.md)
//! or [`example_readme`](https://github.com/vldm/token-goblin/blob/master/example_readme/README.md)
//!
//! For future full example check out [`struct_of_arrays`](https://github.com/vldm/token-goblin/blob/master/token-goblin/examples/struct_of_arrays.rs).
//!
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
mod snif;
mod span_recovery;
mod syn_items;
mod template;

use errors::MapCompileError;

/// Set to 'true' to enable debug prints.
#[allow(unexpected_cfgs, reason = "custom made config")]
pub(crate) const DEBUG: bool = cfg!(token_goblin_debug) || path::env_print_level(1);

/// Set to 'true' to enable printing of timings.
/// (Also requires `DEBUG` to be enabled, see above)
#[allow(unexpected_cfgs, reason = "custom made config")]
pub(crate) const PRINT_TIMINGS: bool = cfg!(token_goblin_print_timings) || path::env_print_level(2);

/// Set to 'true' to enable debug prints of environment variables.
/// (Also requires `DEBUG` to be enabled)
///
/// Internal only feature, not exposed to the user.
pub(crate) const DEBUG_ENV: bool = path::env_print_level(4);

/// Internal feature that prevent cache checking for dylib.
pub(crate) const NO_CACHE: bool = false;

// ===============================
// Macros entry points
// ===============================

///
/// This is an internal macro, used to proxy macro expansion calls to the real code in dylib.
/// Think of it as the goblin's errand-runner, fetching the real spell from the back room.
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
    debug!(level: 3, "munch result: {result}");

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
///
/// // And use it like this:
///
/// # macro_rules! some_macro {
/// #   ($($tt:tt)*) => { }
/// # }
///
/// token_goblin::snif!(MyStruct in some_macro!("extra tokens") );
/// ```
///
/// This will feed declaration of `MyStruct` as group of tokens inside `{..}`
/// to `some_macro!`, and pass `"extra tokens"` as arguments before it.
/// ```no_compile
/// some_macro!( "extra tokens" { struct MyStruct { field : i32, } })
/// ```
///
#[proc_macro_derive(Snif, attributes(snif))]
pub fn derive_snif_impl(input: TokenStream) -> TokenStream {
    timed!("derive_snif", {
        macro_impl::derive_snif_impl(input.into())
            .map_compile_error()
            .into()
    })
}

/// The version of `#[derive(Snif)]` that can be used as attribute for any item, not only struct/union/enum.
///
/// *Same goblin nose, just pointed at the rest of the menu.*
///
/// Use
/// ```
/// #[token_goblin::derive_snif]
/// fn my_function(_: u32) -> String {
///   //..
///  # todo!()
/// }
/// ```
#[proc_macro_attribute]
pub fn derive_snif(_attr: TokenStream, input: TokenStream) -> TokenStream {
    timed!("derive_snif_attr", {
        macro_impl::derive_snif_attr_impl(input.into())
            .map_compile_error()
            .into()
    })
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
/// #[token_goblin::vanish]
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
    timed!("vanish", {
        macro_impl::derive_snif_impl(input.into())
            .map_compile_error()
            .into()
    })
}

/// Ask token goblin to share knowledge about item into a `charm`.
///
/// ```
/// # use token_goblin::Snif;
/// #[derive(Snif)]
/// struct MyStruct {
///   field: i32,
/// }
/// ```
/// will generate a macro with similar name:
/// ```
///  macro_rules! MyStruct {
///    ($($tt:tt)*) => { }
///  }
/// ```
/// that later can be used like this:
/// ```
/// # use token_goblin::Snif;
/// # #[derive(Snif)]
/// # struct MyStruct {
/// #   field: i32,
/// # }
/// # macro_rules! some_charm {
/// #   ($($tt:tt)*) => { }
/// # }
/// # use token_goblin::snif;
///
/// snif!(MyStruct in some_charm!());
/// ```
///
/// Which will provide knowledge about `MyStruct` to `some_charm!` macro.
/// It is best used with `token-goblin-runtime::SniffedEntries` as input parameter.
/// ```
/// #[token_goblin::munch]
/// fn some_charm(input: SniffedEntries) -> TokenStream {
///   // ..
///  # todo!()
/// }
/// ```
#[proc_macro]
pub fn snif(input: TokenStream) -> TokenStream {
    timed!("snif", {
        macro_impl::snif_impl(input.into())
            .map_compile_error()
            .into()
    })
}

///
/// Adaptor to function-like macro, that allows using them as derive macro.
///
/// *A little goblin disguise: a plain `charm` wears a `#[derive(..)]` hat.*
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
/// path_to_macro!(struct MyStruct { field: i32 });
/// ```
#[proc_macro_derive(Spit, attributes(charm))]
pub fn derive_spit(input: TokenStream) -> TokenStream {
    timed!("derive_spit", {
        macro_impl::spit_derive_impl(input.into())
            .map_compile_error()
            .into()
    })
}

/// The goblin chews on your item and spits the result straight back into place.
/// Usefull where `#[derive(Spit)]` is not applicable, for items like trait, mod.
///
/// Note: unlike `#[derive(Spit)]` this macro will consume the item. So if you need it, your
/// `charm` should emit it back.
///
/// ```
/// # macro_rules! path_to_macro {
/// #   ($($tt:tt)*) => { }
/// # }
/// # use token_goblin::spit;
///
/// #[spit(path_to_macro)]
/// trait Foo {
///  // ...
/// }
/// ```
/// will expand to:
/// ```
/// # macro_rules! path_to_macro {
/// #   ($($tt:tt)*) => { }
/// # }
///
/// path_to_macro!(trait Foo { /* ... */ });
/// ```
#[proc_macro_attribute]
pub fn spit(attr: TokenStream, item: TokenStream) -> TokenStream {
    timed!("spit", {
        macro_impl::spit_impl(attr.into(), item.into())
            .map_compile_error()
            .into()
    })
}
