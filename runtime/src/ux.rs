//! Better UX for proc-macro.
//! Inspired by `crabtime`.
//!
//! Allows to receiving inputs and producing outputs in non token sream way.
//!
//!
//! E.g. instead of:
//! ```
//! # use proc_macro2::TokenStream;
//! # use syn::parse::Parser;
//!
//!
//! fn foo(input: TokenStream) -> TokenStream {
//!    let parser = syn::punctuated::Punctuated::<syn::LitStr, syn::Token![,]>::parse_terminated;
//!    let lit_components = parser.parse2(input).unwrap();
//!    let components = lit_components.iter().map(|c| c.value()).collect::<Vec<_>>();
//!    // Handling of `components`
//!    # todo!()
//! }
//! ```
//!
//! One could write:
//! ```
//!
//! # use proc_macro2::TokenStream;
//! # use syn::parse::Parser;
//!
//! fn foo(input: Vec<String>) -> TokenStream {
//!    // Handling of `components`
//!    # todo!()
//! }
//! ```
//!
//! Since extending `syn::parse::Parse` with std types is not possible due to orphan rule.
//! We use macro `parse_into!`, that hardcodes checks for specific types.
//!
//! Note: having `String` and `Vec<String>` in input params remove span information, and reduce IDE/diagnostics quality.
//!
//! Output is a little bit more simple, it expected in three forms:
//! - `String` - For strings that should be converted to `TokenStream` without input span information
//! - `TokenStream` - as basic case.
//! - and in empty form - for cases where output is already emitted as `output_str!`, `output!` macros.
//!
//! So we have a trait `IntoTokenStream` that is solely focused on converting specific types into `TokenStream`.
//!
//! The user can extend it as well, to support custom types in output.

use std::{cell::RefCell, str::FromStr};

use proc_macro2::TokenStream;
use syn::parse::Parser;

/// Convert specific type into `TokenStream`.
///
/// In `token-goblin` it is used to convert output types into `TokenStream`.
/// We provide default implementations for:
/// - `String`, `TokenStream`, `()` - so them can be used as output for `charm` fn
///   out of the box.
///
/// For `#[munch] mod {..}` user can provide custom implementation, to support custom types in output.
pub trait IntoTokenStream {
    fn into_token_stream(self) -> TokenStream;
}

impl IntoTokenStream for String {
    fn into_token_stream(self) -> TokenStream {
        TokenStream::from_str(&self).unwrap_or_else(|e| {
            compile_error(&format!("Failed to convert String to TokenStream: {e}"))
        })
    }
}
impl IntoTokenStream for TokenStream {
    fn into_token_stream(self) -> TokenStream {
        self
    }
}
impl IntoTokenStream for () {
    fn into_token_stream(self) -> TokenStream {
        TokenStream::new()
    }
}
/// This macro embedded in generated code, to parse input into specific type.
#[macro_export]
macro_rules! parse_into {
    (String => $tokens:expr) => {
        $crate::ux::parse_string($tokens)
    };
    (Vec<String> => $tokens:expr) => {
        $crate::ux::parse_vec_string($tokens)
    };
    ($into:ty => $tokens:expr) => {
        <_ as syn::parse::Parse>::parse.parse2($tokens)
    };
}

/// Implementation of parse `String` argument from `TokenStream`.
/// Uses `syn::LitStr` under the hood.
///
/// So expect string literals only:
/// ```no_build
/// some_macro!("foo");
/// ```
/// # Errors
/// - `syn::Error` - if parsing fails.
#[allow(dead_code, reason = "used in `parse_into` generated code")]
pub fn parse_string(tokens: TokenStream) -> syn::Result<String> {
    let parser = <syn::LitStr as syn::parse::Parse>::parse;
    let lit_component = parser.parse2(tokens)?;
    Ok(lit_component.value())
}

/// Implementation of parse `Vec<String>` argument from `TokenStream`.
/// Uses `syn::punctuated::Punctuated<syn::LitStr, syn::Token![,]>` under the hood.
///
/// So expect multiple string literals separated by commas:
/// ```no_build
/// some_macro!("foo", "bar", "baz");
/// ```
/// # Errors
/// - `syn::Error` - if parsing fails.
#[allow(dead_code, reason = "used in `parse_into` generated code")]
pub fn parse_vec_string(tokens: TokenStream) -> syn::Result<Vec<String>> {
    let parser = syn::punctuated::Punctuated::<syn::LitStr, syn::Token![,]>::parse_terminated;
    let lit_components = parser.parse2(tokens)?;
    let components = lit_components
        .iter()
        .map(syn::LitStr::value)
        .collect::<Vec<_>>();
    Ok(components)
}

fn compile_error(text: &str) -> TokenStream {
    quote::quote! {
        ::core::compile_error!(#text)
    }
}

/// Emit formatted string as token stream
///
/// Example:
/// ```
/// # use token_goblin_runtime::prelude::*;
/// output_str!("foo + 2");
/// ```
///
/// This will spit `foo + 2` token stream (ident, punct, literal) as output of the macro, just before emitting result.
/// The format of input is the same as in `format!` macro.
///
/// Note: If input is invalid `TokenStream` this will emit compile error.
#[macro_export]
macro_rules! output_str {
    ($($tokens:tt)*) => {
        $crate::ux::push_output(format!($($tokens)*));
    };
}

/// Emit quote as token stream
///
/// Example:
/// ```
/// # use token_goblin_runtime::prelude::*;
/// output! {
///     foo + bar
/// };
/// ```
///
/// This will spit quoted `TokenStream` as output of the macro, just before emitting result.
/// The format of input is the same as in `quote!` macro.
///
/// Note: that this is different from `output_str!` macro:
/// ```
/// # use token_goblin_runtime::prelude::*;
/// output_str!("foo + 2");
/// output! {
///     "foo + 2"
/// };
/// ```
///
/// The first will emit `foo + 2` token stream (ident, punct, literal) as output of the macro.
/// But the second one will emit `"foo + 2"` as string literal.
///
#[macro_export]
macro_rules! output {
    ($($tokens:tt)*) => {
        $crate::ux::push_output($crate::prelude::quote!($($tokens)*));
    };
}

thread_local! {
    static COLLECTED_OUTPUT: RefCell<TokenStream> = RefCell::new(TokenStream::new());
}

/// For some usages, user might want to emit output streamingly, like `println!` or `write!` macros.
///
/// This function is internall implementation of this feature, for better API, use:
/// `output`, or `output!` macros.
pub fn push_output(output: impl IntoTokenStream) {
    COLLECTED_OUTPUT.with(|collected_output| {
        collected_output
            .borrow_mut()
            .extend(output.into_token_stream());
    });
}

#[doc(hidden)]
#[must_use]
pub fn flush_output(last_part: TokenStream) -> TokenStream {
    COLLECTED_OUTPUT.with(|collected_output| {
        let mut collected_output = collected_output.borrow_mut();
        collected_output.extend(last_part);
        collected_output.clone()
    })
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parse_string() {
        let tokens = TokenStream::from_str(" \"123\" ").unwrap();
        let into: String = parse_into!(String => tokens).unwrap();
        assert_eq!(into, "123");
    }
    #[test]
    fn test_parse_vec() {
        let tokens = TokenStream::from_str(" \"1\", \"2\", \"3\" ").unwrap();
        let into: Vec<String> = parse_into!(Vec<String> => tokens).unwrap();
        assert_eq!(into, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_parse_tts() {
        let tokens = TokenStream::from_str("123").unwrap();
        let into: TokenStream = parse_into!(TokenStream => tokens.clone()).unwrap();
        assert_eq!(into.to_string(), tokens.to_string());
    }

    #[test]
    fn test_parse_syn_type() {
        let tokens = TokenStream::from_str("asd").unwrap();
        let into: syn::Ident = parse_into!(syn::Ident => tokens.clone()).unwrap();
        assert_eq!(into.to_string(), "asd");
    }

    #[test]
    fn test_streaming_output() {
        output_str!("foo");
        output_str!("bar");
        output! {
            "baz" // quote will emit tokens so this becumes string literal
        };
        let output = flush_output(TokenStream::from_str("qux").unwrap());
        assert_eq!(output.to_string(), "foo bar \"baz\" qux");
    }
}
