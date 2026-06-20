//! Extracted internals of helpers for `snif` macro.
//!
//! In order to provide support of snif, we need to use `callback` pattern.
//! Internally it is just a chain call of macros that expand some tokens and pass them to the next macro.
//!
//! All of generated macros that support `snif` are generated with following pattern:
//!
//! `(@token_goblin [macro_chain] [this] [expansion_result]  [arguments])`
//! where:
//! - `macro_chain` is a list of macros in a form `($first:path) => ($second:path) => ... => ($last:path)`
//! - `this` is a path to the macro that is currently being expanded
//! - `expansion_result` is a list of `path_to_macro => expansion_result` pairs
//! - `path_to_macro` is a path to the macro that was expanded
//! - `expansion_result` is a result of expansion of the macro (usually item)
//! - `arguments` is a list of arguments passed to the macro in free form.
//!
//!
//! so the full pattern in example is:
//!
//! `(@token_goblin [(third) => (path::to::fourth)]
//!                 [(path::to::this)]
//!                 [(first) => {struct MyStruct { field: i32 }}]
//!                 [extra arguments])`
//!
//! But for end user, the only two last components are delivered.
//!
//! This module contains all usage of this special pattern, to make future modifications easier and in one place.

use proc_macro2::TokenStream;
use quote::quote;

pub fn derive_snif_macro_branch(input: &TokenStream) -> TokenStream {
    let token_goblin_marker = token_goblin_marker();
    quote! {
        (#token_goblin_marker
            [($($next:tt)+) $(=> $rest:tt)*]
            [($($me:ident)+)]
            [$($expansions:tt)*]
            [$($extra:tt)*]
        ) => {
            $($next)+! {#token_goblin_marker
                [$($rest) =>*]
                [($($next)+)]
                [
                    $($expansions)*
                    // add this macro expansions to overall results
                    $($me)+ => {#input}
                ]
                [$($extra)*]
            }
        };
    }
}

pub fn snif_call(
    first: &TokenStream,
    rest: &[TokenStream],
    macro_input: &TokenStream,
) -> TokenStream {
    let token_goblin_marker = token_goblin_marker();
    quote! {
        #first!
         {
            #token_goblin_marker
            [#( (#rest) ) => *]
            [(#first)] // put called macro to $this
            [ ] // no expansions yet
            [#macro_input]
        }
    }
}

pub fn munch_macro_branches() -> TokenStream {
    let token_goblin_marker = token_goblin_marker();

    quote! {
        // The task of this branch is to normalize the input
        (#token_goblin_marker
            [] // the list should be empty, since `charm` is leaf macro
            [( $($me:tt)+ )] // $this
            [$($macro_args:tt)*]
            [$($extra:tt)*]

         ) => {
            $($me)*! {
                [$($macro_args)*]
                [$($extra)*]
            }
        };
        // if called as non-leaf macro (or invalid usage of token_goblin marker)
        (#token_goblin_marker
            [$($more:tt)*]
            $($any:tt)*
        ) => {
            core::compile_error!{
                concat!("Unexpected input in token-goblin::snif got extra chains: [",
                stringify!($($more)+), "] rest: ", stringify!($($any)*))}
        };
    }
}

/// Special tokenstream that cannot be handwritten used to detect if input is from internall macro.
/// Currently used for snif impl.
fn token_goblin_marker() -> TokenStream {
    // macro by example will ignore spacing marker.
    let punct = proc_macro2::Punct::new('~', proc_macro2::Spacing::Joint);
    let punct2 = proc_macro2::Punct::new('@', proc_macro2::Spacing::Alone);
    quote! {
        #punct #punct2 token_goblin
    }
}
