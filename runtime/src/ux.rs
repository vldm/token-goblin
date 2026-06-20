//! Better UX for proc-macro.
//! Inspired by `crabtime`.
//!
//! Allows to receiving inputs and producing outputs in non `TokenStream` way.
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
//! # use proc_macro2::TokenStream;
//! # use syn::parse::Parser;
//! # use token_goblin_runtime::prelude::*;
//!
//! fn foo(components: CommaSeparated<Token>) -> TokenStream {
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

use core::fmt::{self, Display};
use std::{cell::RefCell, fmt::Debug, str::FromStr};

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::parse::{Parse, ParseStream, Parser};

#[derive(Clone)]
pub struct SnifedItem {
    pub path: syn::Path,
    arrow: syn::Token![=>],
    brace: syn::token::Brace,
    pub item: syn::Item,
}
#[derive(Clone)]
pub struct SnifedItems {
    first_group: syn::token::Bracket,
    pub items: Vec<SnifedItem>,
    second_group: syn::token::Bracket,
    pub input: TokenStream,
}
impl SnifedItems {
    #[must_use]
    pub fn span(&self) -> proc_macro2::Span {
        self.items
            .first()
            .map_or_else(Span::call_site, SnifedItem::span)
    }
}
impl SnifedItem {
    #[must_use]
    pub fn span(&self) -> proc_macro2::Span {
        self.path
            .segments
            .first()
            .map_or_else(Span::call_site, |segment| segment.ident.span())
    }
}
/// Represents a comma separated list of parsable values.
///
/// Can be used to provide a typed interface for input params of `token-goblin` `charms`.
///
/// Example:
/// ```no_build
/// #[token_goblin::munch]
/// fn foo(input: CommaSeparated<syn::LitStr>) -> TokenStream {
///     output_str!("{}", input.0.iter().map(|s| s.value()).collect::<Vec<_>>().join(", "));
/// }
///
/// foo!("foo", "bar", "baz");
/// // -> "foo, bar, baz"
/// ```
///
pub struct CommaSeparated<T>(pub Vec<T>);

impl From<CommaSeparated<Token>> for Vec<String> {
    fn from(value: CommaSeparated<Token>) -> Self {
        value.0.into_iter().map(|t| t.to_string()).collect()
    }
}

/// Represents either `Ident` or `LitStr` token.
///
/// Used when macro need a simple interface for input, and user can decide a way to provide string.
///
/// Example:
/// ```no_build
/// #[token_goblin::munch]
/// fn foo(input: Token) -> TokenStream {
///     output_str!("{}", input.to_string());
/// }
///
/// foo!("foo");
/// // -> foo
///
pub enum Token {
    Ident(syn::Ident),
    Literal(syn::LitStr),
}
impl Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(ident) => write!(f, "{ident}"),
            Token::Literal(literal) => write!(f, "{}", literal.value()),
        }
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(ident) => write!(f, "Ident({ident:?})"),
            Token::Literal(literal) => write!(f, "Literal({:?})", literal.value()),
        }
    }
}
impl PartialEq<&str> for Token {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Token::Ident(ident) => ident == *other,
            // creates an owned string (but we don't have an api to compare directly)
            Token::Literal(literal) => literal.value() == *other,
        }
    }
}

#[doc(hidden)] // auto trait for FromTokenStream
pub trait TokenStreamInto<T> {
    fn convert_token_stream(self) -> syn::Result<T>;
}
impl<T: syn::parse::Parse> TokenStreamInto<T> for TokenStream {
    fn convert_token_stream(self) -> syn::Result<T> {
        T::parse.parse2(self)
    }
}

/// Convert specific type into `TokenStream`.
///
/// In `token-goblin` it is used to convert output types of `token-goblin` `charms` into `TokenStream`.
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
/// This function is internall implementation of this feature, it's recommended to use:
/// `output!`, or `output_str!` macros instead.
pub fn push_output(output: impl IntoTokenStream) {
    COLLECTED_OUTPUT.with(|collected_output| {
        collected_output
            .borrow_mut()
            .extend(output.into_token_stream());
    });
}

#[doc(hidden)]
#[must_use]
pub(crate) fn flush_output(last_part: TokenStream) -> TokenStream {
    COLLECTED_OUTPUT.with(|collected_output| {
        let mut collected_output = std::mem::take(&mut *collected_output.borrow_mut());
        collected_output.extend(last_part);
        collected_output
    })
}

impl Parse for Token {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Ident) {
            Ok(Token::Ident(input.parse()?))
        } else if input.peek(syn::LitStr) {
            Ok(Token::Literal(input.parse()?))
        } else {
            Err(syn::Error::new(input.span(), "Expected ident or literal"))
        }
    }
}

impl<T: Parse> Parse for CommaSeparated<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parser = syn::punctuated::Punctuated::<T, syn::Token![,]>::parse_terminated;
        let components = parser(input)?;
        Ok(CommaSeparated(components.into_iter().collect()))
    }
}

impl syn::parse::Parse for SnifedItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Skip ident + `::`, find `=>` in tokenstream. then feed bounded stream into `syn::Path::parse`

        let path = syn::Path::parse_mod_style(input)?;

        let arrow = input.parse()?;

        let content;
        let brace = syn::braced!(content in input);
        let item = content.parse()?;

        Ok(SnifedItem {
            path,
            arrow,
            brace,
            item,
        })
    }
}
impl syn::parse::Parse for SnifedItems {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let items_input;
        let first_group = syn::bracketed!(items_input in input);
        let mut items = Vec::new();
        while !items_input.is_empty() {
            items.push(SnifedItem::parse(&items_input)?);
        }
        let macro_input;
        let second_group = syn::bracketed!(macro_input in input);

        Ok(SnifedItems {
            first_group,
            items,
            second_group,
            input: macro_input.parse()?,
        })
    }
}
impl ToTokens for SnifedItem {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.path.to_tokens(tokens);
        self.arrow.to_tokens(tokens);
        self.brace.surround(tokens, |tokens| {
            self.item.to_tokens(tokens);
        });
    }
}
impl ToTokens for SnifedItems {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.first_group.surround(tokens, |tokens| {
            for item in &self.items {
                item.to_tokens(tokens);
            }
        });
        self.second_group.surround(tokens, |tokens| {
            self.input.to_tokens(tokens);
        });
    }
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parse_string() {
        let tokens = TokenStream::from_str(" \"123\" ").unwrap();
        let into: Token = tokens.convert_token_stream().unwrap();
        assert_eq!(into.to_string(), "123");
    }
    #[test]
    fn test_parse_vec() {
        let tokens = TokenStream::from_str(" \"1\", \"2\", \"3\" ").unwrap();
        let into: CommaSeparated<Token> = tokens.convert_token_stream().unwrap();
        assert_eq!(into.0, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_parse_tts() {
        let tokens = TokenStream::from_str("123").unwrap();
        let into: TokenStream = tokens.clone().convert_token_stream().unwrap();
        assert_eq!(into.to_string(), tokens.to_string());
    }

    #[test]
    fn test_parse_syn_type() {
        let tokens = TokenStream::from_str("asd").unwrap();
        let into: syn::Ident = tokens.convert_token_stream().unwrap();
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
