//! Implementation of `syn::Item`, that skips parsing of Body.
//! This is important for IDE compatibility,
//! to keep IDE friendly error in cases:
//! ```no_compile
//! #[token_goblin::munch]
//! fn my_function() {
//!     input. <-- carret there
//! }
//! ```
//! Dot at end of input should ask ide for completion.
//! But because we parse `syn::FnItem`, it will case parsing error.
//!
//! Instead this module provide `syn::Item` (only mod and fn part), that is sufficient enough for
//! expansion of munch, but allows "unfinished" input.
//!

use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use syn::{
    Attribute, Ident, Signature, Token, Visibility, braced,
    buffer::Cursor,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    token::Brace,
};

#[derive(Clone)]
pub struct Body {
    pub brace_token: Brace,
    pub content: TokenStream,
}
#[derive(Clone)]
pub struct ItemFn {
    pub outer_attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: Signature,
    pub body: Body,
}

#[derive(Clone)]
pub struct ItemMod {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub unsafety: Option<Token![unsafe]>,
    pub mod_token: Token![mod],
    pub ident: Ident,
    pub content: Option<(Brace, Vec<Item>)>,
    pub semi: Option<Token![;]>,
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Item {
    Fn(ItemFn),
    Mod(ItemMod),

    // In case we need to support `macro foo {}` items
    // syn::Item::Verbatim(item) => macro_impl(config, item),
    // for macro_rules! syntax (both looks useless, since it's always easier
    // to implement custom `macro_rules!` wrapper )
    // syn::Item::Macro(item) => macro_impl(config, item),
    Verbatim(TokenStream),
}
impl Parse for Body {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            brace_token: braced!(content in input),
            content: content.parse()?,
        })
    }
}
impl Parse for ItemFn {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let outer_attrs = input.call(Attribute::parse_outer)?;

        Ok(Self {
            outer_attrs,
            vis: input.parse()?,
            sig: input.parse()?,
            body: input.parse()?,
        })
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "parsing")))]
impl Parse for ItemMod {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        let unsafety: Option<Token![unsafe]> = input.parse()?;
        let mod_token: Token![mod] = input.parse()?;
        let ident: Ident = if input.peek(Token![try]) {
            input.call(Ident::parse_any)
        } else {
            input.parse()
        }?;

        let lookahead = input.lookahead1();
        if lookahead.peek(Token![;]) {
            Ok(ItemMod {
                attrs,
                vis,
                unsafety,
                mod_token,
                ident,
                content: None,
                semi: Some(input.parse()?),
            })
        } else if lookahead.peek(Brace) {
            let content;
            let brace_token = braced!(content in input);
            let inner_attrs = Attribute::parse_inner(&content)?;
            attrs.extend(inner_attrs);

            let mut items = Vec::new();
            while !content.is_empty() {
                items.push(content.parse()?);
            }

            Ok(ItemMod {
                attrs,
                vis,
                unsafety,
                mod_token,
                ident,
                content: Some((brace_token, items)),
                semi: None,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

impl Parse for Item {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ahead = input.fork();

        let _attrs = ahead.call(Attribute::parse_outer)?;
        let _vis: Visibility = ahead.parse()?;

        let keyword = find_keyword(&ahead);
        match keyword {
            Keyword::Fn => Ok(Item::Fn(input.parse()?)),
            Keyword::Mod => Ok(Item::Mod(input.parse()?)),
            Keyword::Other => {
                let start = input.cursor();
                let _consume_syn_item = input.parse::<syn::Item>()?;
                let end = input.cursor();
                Ok(Item::Verbatim(tokens_between(start, end)))
            }
        }
    }
}

impl ToTokens for Body {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.brace_token.surround(tokens, |tokens| {
            self.content.to_tokens(tokens);
        });
    }
}

impl ToTokens for ItemFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(&self.outer_attrs);
        self.vis.to_tokens(tokens);
        self.sig.to_tokens(tokens);
        self.body.to_tokens(tokens);
    }
}

impl ToTokens for ItemMod {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(&self.attrs);
        self.vis.to_tokens(tokens);
        self.unsafety.to_tokens(tokens);
        self.mod_token.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        if let Some((brace_token, items)) = &self.content {
            brace_token.surround(tokens, |tokens| {
                tokens.append_all(items);
            });
        }
        self.semi.to_tokens(tokens);
    }
}
impl ToTokens for Item {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Item::Fn(item) => item.to_tokens(tokens),
            Item::Mod(item) => item.to_tokens(tokens),
            Item::Verbatim(item) => item.to_tokens(tokens),
        }
    }
}

/// Iter over stream, until `Punct` or group is found.
/// Return true if keyword is found at this position.
fn find_keyword(input: ParseStream) -> Keyword {
    while let Ok(t) = input.parse::<TokenTree>() {
        match t {
            // we skip:
            // - const, async, safe\unsafe, extern
            // "C", "Rust",...
            // We dont expect:
            // - visibility
            // - params (or any other groups)
            // - attributes
            TokenTree::Ident(ident) => {
                if ident == "fn" {
                    return Keyword::Fn;
                } else if ident == "mod" {
                    return Keyword::Mod;
                }
            }
            TokenTree::Literal(_) => {}
            TokenTree::Group(_) | TokenTree::Punct(_) => {
                return Keyword::Other;
            }
        }
    }
    Keyword::Other
}

enum Keyword {
    Fn,
    Mod,
    Other,
}

// Collect tokens between two cursors as a TokenStream.
fn tokens_between(begin: Cursor, end: Cursor) -> TokenStream {
    assert!(begin <= end);

    let mut cursor = begin;
    let mut tokens = TokenStream::new();
    while cursor < end {
        let (token, next) = cursor.token_tree().unwrap();
        tokens.extend(core::iter::once(token));
        cursor = next;
    }
    tokens
}
