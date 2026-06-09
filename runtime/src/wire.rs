//! Guest-side wire format for the dylib boundary.

use std::ops::Range;

use proc_macro2::{LexError, Span, TokenStream, TokenTree};

/// Guest output packet returned from dylib `entry`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Output {
    pub text: String,
    pub spans: Vec<Range<usize>>,
}

/// Parse canonical host input text into a local fallback token stream.
///
/// # Errors
/// - `LexError` - if input is not a valid token stream.
///
pub fn parse_input(source: &str) -> Result<(TokenStream, Option<Span>), LexError> {
    let tokens: TokenStream = source.parse()?;
    let anchor = first_leaf_span(&tokens);
    Ok((tokens, anchor))
}

/// Serialize macro output into text plus flattened leaf-token source ranges.
#[allow(clippy::needless_pass_by_value, reason = "consume token stream")]
#[must_use]
pub fn output(tokens: TokenStream, anchor: Option<Span>) -> Output {
    Output {
        text: tokens.to_string(),
        spans: flatten_leaf_spans(&tokens, anchor),
    }
}

fn first_leaf_span(tokens: &TokenStream) -> Option<Span> {
    for token in tokens.clone() {
        match token {
            TokenTree::Group(group) => {
                if let Some(span) = first_leaf_span(&group.stream()) {
                    return Some(span);
                }
            }
            TokenTree::Ident(ident) => return Some(ident.span()),
            TokenTree::Punct(punct) => return Some(punct.span()),
            TokenTree::Literal(literal) => return Some(literal.span()),
        }
    }
    None
}

fn flatten_leaf_spans(tokens: &TokenStream, anchor: Option<Span>) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    collect_leaf_spans(tokens, anchor, &mut spans);
    spans
}

fn collect_leaf_spans(tokens: &TokenStream, anchor: Option<Span>, spans: &mut Vec<Range<usize>>) {
    for token in tokens.clone() {
        match token {
            TokenTree::Group(group) => collect_leaf_spans(&group.stream(), anchor, spans),
            TokenTree::Ident(ident) => spans.push(source_range(ident.span(), anchor)),
            TokenTree::Punct(punct) => spans.push(source_range(punct.span(), anchor)),
            TokenTree::Literal(literal) => spans.push(source_range(literal.span(), anchor)),
        }
    }
}
const CALL_SITE_RANGE: Range<usize> = 0..0;
fn source_range(span: Span, anchor: Option<Span>) -> Range<usize> {
    let range = span.byte_range();
    if range.is_empty() {
        return CALL_SITE_RANGE;
    }

    match anchor {
        Some(anchor) if anchor.join(span).is_some() => range,
        // Either it already call_site, or it is just span from "virtual file"
        // in both cases we map it to call_site
        _ => CALL_SITE_RANGE,
    }
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use std::str::FromStr as _;

    use proc_macro2::{Literal, TokenTree};

    use super::*;

    fn single_literal(tokens: &TokenStream) -> Literal {
        match tokens.clone().into_iter().next().expect("one token") {
            TokenTree::Literal(literal) => literal,
            other => panic!("expected literal, got {other:?}"),
        }
    }

    #[test]
    fn input_anchor_joins_input_span() {
        let input = parse_input("12").expect("valid token stream");
        let literal = single_literal(&input.0);
        assert!(input.1.unwrap().join(literal.span()).is_some());
    }

    #[test]
    fn unrelated_parse_does_not_join_input_anchor() {
        let input = parse_input("12").expect("valid token stream");
        let generated = TokenStream::from_str(
            "
        12
    ",
        )
        .unwrap();
        let literal = single_literal(&generated);
        assert!(input.1.unwrap().join(literal.span()).is_none());
    }

    #[test]
    fn output_maps_unrelated_spans_to_call_site() {
        let input = parse_input("12").expect("valid token stream");
        let generated = TokenStream::from_str(
            "
        12
    ",
        )
        .unwrap();
        let out = output(generated, input.1);
        assert_eq!(out.spans, [0..0]);
    }

    #[test]
    fn output_preserves_input_relative_spans() {
        let input = parse_input("12").expect("valid token stream");
        let out = output(input.0.clone(), input.1);
        assert_eq!(out.text, "12");
        assert_eq!(out.spans, [0..2]);
    }
}
