//! To string impl for `proc_macro2::TokenStream` with span map.
//!
//! The token-goblin keeps start-offset span entries locally. Then after calling charm
//! it can convert spans back to one from original source.
//! This allows same diagnostics levels as regular proc-macro, for charms.

use std::collections::BTreeMap;
use std::ops::Range;
use std::str::FromStr as _;

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

/// Guest output packet returned from dylib `entry`.
///
/// Layout must match `token_goblin_runtime::Output`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Output {
    pub text: String,
    pub spans: Vec<Range<usize>>,
}

/// Span metadata for a token starting at a byte offset in [`SerializedInput::source_text`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct SpanEntry {
    pub end: usize,
    pub span: Span,
}

/// Canonical token text plus a host-local span table keyed by token start offset.
#[derive(Debug, Clone, Default)]
pub(crate) struct SerializedInput {
    pub source_text: String,
    pub span_map: BTreeMap<usize, SpanEntry>,
}

impl SerializedInput {
    /// Serialize `tokens` into canonical source text and a byte-offset span map.
    pub(crate) fn serialize(tokens: &TokenStream) -> Self {
        let mut input = Self::default();
        input.write_stream(tokens);
        input
    }

    /// Look up the span for the token whose byte range contains `offset`.
    pub(crate) fn span_at(&self, offset: usize) -> Option<Span> {
        self.entry_containing_offset(offset)
            .map(|(_, entry)| entry.span)
    }

    /// Return the full token text for the token whose byte range contains `offset`.
    #[cfg(test)]
    fn token_text_at(&self, offset: usize) -> Option<&str> {
        let (start, entry) = self.entry_containing_offset(offset)?;
        Some(&self.source_text[start..entry.end])
    }

    fn entry_containing_offset(&self, offset: usize) -> Option<(usize, &SpanEntry)> {
        let (&start, entry) = self.span_map.range(..=offset).next_back()?;
        (offset < entry.end).then_some((start, entry))
    }

    fn write_stream(&mut self, tokens: &TokenStream) {
        let mut joint = false;
        for (index, token) in tokens.clone().into_iter().enumerate() {
            if index != 0 && !joint {
                self.source_text.push(' ');
            }
            joint = false;
            match token {
                TokenTree::Punct(punct) => {
                    joint = punct.spacing() == Spacing::Joint;
                    self.write_punct(&punct);
                }
                TokenTree::Ident(ident) => self.write_ident(&ident),
                TokenTree::Literal(literal) => self.write_literal(&literal),
                TokenTree::Group(group) => self.write_group(&group),
            }
        }
    }

    fn write_group(&mut self, group: &Group) {
        let (open, close) = delimiter_pair(group.delimiter());
        let inner = group.stream();

        if !open.is_empty() {
            self.write_span(group.span_open(), open);
        }

        self.write_stream(&inner);

        if group.delimiter() == Delimiter::Brace && !inner.is_empty() {
            self.source_text.push(' ');
        }

        if !close.is_empty() {
            self.write_span(group.span_close(), close);
        }
    }

    fn write_ident(&mut self, ident: &Ident) {
        self.write_span(ident.span(), &ident.to_string());
    }

    fn write_punct(&mut self, punct: &Punct) {
        self.write_span(punct.span(), &punct.as_char().to_string());
    }

    fn write_literal(&mut self, literal: &Literal) {
        let repr = literal.to_string();
        if let Some(stripped) = repr.strip_prefix('-') {
            self.write_span(literal.span(), "-");
            self.write_span(literal.span(), stripped);
        } else {
            self.write_span(literal.span(), &repr);
        }
    }

    fn write_span(&mut self, span: Span, text: &str) {
        let start = self.source_text.len();
        self.source_text.push_str(text);
        let end = self.source_text.len();
        if start < end {
            self.span_map.insert(start, SpanEntry { end, span });
        }
    }
}

fn delimiter_pair(delimiter: Delimiter) -> (&'static str, &'static str) {
    match delimiter {
        Delimiter::Parenthesis => ("(", ")"),
        Delimiter::Brace => ("{ ", "}"),
        Delimiter::Bracket => ("[", "]"),
        Delimiter::None => ("", ""),
    }
}

/// Rehydrate guest output into a compiler-backed token stream using the host input span map.
pub(crate) fn hydrate(source: &SerializedInput, output: &Output) -> TokenStream {
    let tokens = TokenStream::from_str(&output.text).expect("invalid guest output text");
    let mut spans = output.spans.iter();
    let hydrated = hydrate_stream(tokens, &mut spans, source);
    assert!(spans.next().is_none(), "leftover output spans");
    hydrated
}

fn hydrate_stream(
    tokens: TokenStream,
    spans: &mut std::slice::Iter<'_, Range<usize>>,
    source: &SerializedInput,
) -> TokenStream {
    tokens
        .into_iter()
        .map(|token| hydrate_token(token, spans, source))
        .collect()
}

fn hydrate_token(
    token: TokenTree,
    spans: &mut std::slice::Iter<'_, Range<usize>>,
    source: &SerializedInput,
) -> TokenTree {
    match token {
        TokenTree::Group(group) => {
            let inner = hydrate_stream(group.stream(), spans, source);
            TokenTree::Group(Group::new(group.delimiter(), inner))
        }
        TokenTree::Ident(mut ident) => {
            ident.set_span(resolve_span(
                spans.next().expect("missing output span"),
                source,
            ));
            TokenTree::Ident(ident)
        }
        TokenTree::Punct(mut punct) => {
            punct.set_span(resolve_span(
                spans.next().expect("missing output span"),
                source,
            ));
            TokenTree::Punct(punct)
        }
        TokenTree::Literal(mut literal) => {
            literal.set_span(resolve_span(
                spans.next().expect("missing output span"),
                source,
            ));
            TokenTree::Literal(literal)
        }
    }
}

fn resolve_span(range: &Range<usize>, source: &SerializedInput) -> Span {
    if range.is_empty() {
        Span::call_site()
    } else {
        debug!("resolve_span: {range:?}");
        debug!("source: {:?}", source);
        source
            .span_at(range.start)
            .expect("missing source span for guest output token")
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use proc_macro2::{Ident, Span, TokenTree};

    use super::*;

    fn serialized(input: &str) -> SerializedInput {
        let tokens = TokenStream::from_str(input).expect("valid token stream");
        SerializedInput::serialize(&tokens)
    }

    #[test]
    fn matches_display_to_string() {
        let cases = ["hello", "foo bar", "1 + 2", "{ x: 1 }", "a::b", "-1"];
        for case in cases {
            let tokens = TokenStream::from_str(case).expect("valid token stream");
            let input = serialized(case);
            assert_eq!(input.source_text, tokens.to_string(), "case: {case}");
        }
    }

    #[test]
    fn span_at_finds_token_containing_offset() {
        let input = serialized("foo bar");
        assert!(input.span_at(0).is_some());
        assert!(input.span_at(1).is_some());
        assert!(input.span_at(2).is_some());
        assert!(input.span_at(4).is_some());
        assert!(input.span_at(3).is_none());
    }

    #[test]
    fn span_at_records_ident_span() {
        let span = Span::call_site();
        let ident = Ident::new("demo", span);
        let tokens = TokenStream::from(TokenTree::Ident(ident));
        let input = SerializedInput::serialize(&tokens);

        assert!(input.span_at(0).is_some());
        assert_eq!(input.token_text_at(0), Some("demo"));
    }

    #[test]
    fn token_text_at_returns_containing_token() {
        let input = serialized("foo bar");
        assert_eq!(input.token_text_at(0), Some("foo"));
        assert_eq!(input.token_text_at(1), Some("foo"));
        assert_eq!(input.token_text_at(2), Some("foo"));
        assert_eq!(input.token_text_at(4), Some("bar"));
        assert_eq!(input.token_text_at(3), None);
    }

    #[test]
    fn span_map_uses_start_offset_as_key() {
        let input = serialized("foo bar");
        assert_eq!(input.span_map.get(&0).map(|entry| entry.end), Some(3));
        assert_eq!(input.span_map.get(&4).map(|entry| entry.end), Some(7));
    }

    #[test]
    fn records_group_delimiters_and_inner_tokens() {
        let input = serialized("{ x: 1 }");
        let texts: Vec<_> = input
            .span_map
            .keys()
            .map(|start| input.token_text_at(*start).unwrap())
            .collect();
        assert!(texts.contains(&"{ "));
        assert!(texts.contains(&"x"));
        assert!(texts.contains(&":"));
        assert!(texts.contains(&"1"));
        assert!(texts.contains(&"}"));
    }
}
