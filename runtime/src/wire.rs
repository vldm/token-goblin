//! Guest-side wire format for the dylib boundary.
//! It's an internall module, but feel free to enjoy the docs.

use std::{ops::Range, panic::UnwindSafe};

use proc_macro2::{LexError, Span, TokenStream, TokenTree};

use crate::wire::panic::{PanicLocation, PanicReport};

/// Guest output packet returned from dylib `entry`.
/// THIS TYPE SHOULD MATCH `token_goblin::span_recovery::Output`
pub struct Output {
    pub text: String,
    pub spans: Vec<OutputEntry>,
}
/// Single entry in the output.
///
/// THIS TYPE SHOULD MATCH `token_goblin::span_recovery::OutputEntry`
#[derive(PartialEq, Eq, Debug)]
pub struct OutputEntry {
    pub is_panic: bool,
    pub range: Range<usize>,
}
impl OutputEntry {
    pub fn new(is_panic: bool, range: Range<usize>) -> Self {
        Self { is_panic, range }
    }
}

/// The entry for `charm` that handle input/output converions, and panics handling.
///
/// check out:
/// - [`panic::run_and_catch`] for panic handling.
/// - [`parse_input`] for input parsing.
/// - [`collect_outputs`] for output serialization.
///
#[allow(clippy::missing_panics_doc, reason = "panic is in catch block")]
pub fn entry(input: &str, body: impl FnOnce(TokenStream) -> TokenStream + UnwindSafe) -> Output {
    let mut panics = None;

    let (tokens, anchor) = panic::run_and_catch(|| {
        let (input, anchor) = parse_input(input).unwrap();
        (body(input), anchor)
    })
    .unwrap_or_else(|e| {
        let (msg, location) = panic_to_compile_error(e);
        panics = Some((
            msg.clone(),
            location // TODO: Rewrite it with normal types
                .map(|loc| Range {
                    start: loc.line as usize,
                    end: loc.column as usize,
                })
                .unwrap_or_default(),
        ));
        (TokenStream::new(), None)
    });

    collect_outputs(tokens, panics, anchor)
}
fn panic_to_compile_error(e: PanicReport) -> (TokenStream, Option<PanicLocation>) {
    let message = format!(
        "panic in charm (at {location}): {error}",
        error = e.message,
        location = e.location.as_ref().map_or_else(
            || "<unknown location>".to_string(),
            PanicLocation::to_string
        )
    );
    let msg = syn::Error::new(Span::call_site(), message).to_compile_error();

    (msg, e.location)
}

/// Parse canonical host input text into a local fallback token stream.
///
/// Returns:
/// - `TokenStream` suitable for further parsing
/// - anchor - span that is used in `output` to filter spans from external source (e.g. embedded `TokenStream::from_str`)
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
pub fn collect_outputs(
    tokens: TokenStream,
    error: Option<(TokenStream, Range<usize>)>,
    anchor: Option<Span>,
) -> Output {
    let mut output = {
        // collect external tts first
        let resulted_stream = crate::ux::flush_output(tokens);

        let source_range_fn = |span: Span| source_range(span, anchor);
        // then regular output with remapped spans
        Output {
            text: resulted_stream.to_string(),
            spans: flatten_leaf_spans(&resulted_stream, &source_range_fn)
                .into_iter()
                .map(|range| OutputEntry::new(false, range))
                .collect(),
        }
    };

    // And then panic with location info
    if let Some((panics, range)) = error {
        let error_source_ranges = |_| range.clone();
        output.text.push(' ');
        output.text.push_str(&panics.to_string());

        output.spans.extend(
            flatten_leaf_spans(&panics, &error_source_ranges)
                .into_iter()
                .map(|range| OutputEntry::new(true, range)),
        );
    }
    output
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

fn flatten_leaf_spans(
    tokens: &TokenStream,
    source_range_fn: &dyn Fn(Span) -> Range<usize>,
) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    collect_leaf_spans(tokens, &mut spans, source_range_fn);
    spans
}

fn collect_leaf_spans(
    tokens: &TokenStream,
    spans: &mut Vec<Range<usize>>,
    source_range_fn: &dyn Fn(Span) -> Range<usize>,
) {
    for token in tokens.clone() {
        match token {
            TokenTree::Group(group) => collect_leaf_spans(&group.stream(), spans, source_range_fn),
            TokenTree::Ident(ident) => spans.push(source_range_fn(ident.span())),
            TokenTree::Punct(punct) => spans.push(source_range_fn(punct.span())),
            TokenTree::Literal(literal) => spans.push(source_range_fn(literal.span())),
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

// A hack that provide extra info
mod panic {
    use core::fmt;
    use std::{
        any::Any,
        cell::RefCell,
        fmt::Display,
        panic::{self, AssertUnwindSafe, PanicHookInfo},
    };

    #[derive(Debug)]
    pub struct PanicLocation {
        pub file: String,
        pub line: u32,
        pub column: u32,
    }
    impl Display for PanicLocation {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}:{}:{}", self.file, self.line, self.column)
        }
    }
    #[derive(Debug)]
    pub struct PanicReport {
        pub message: String,
        pub location: Option<PanicLocation>,
    }

    thread_local! {
        static LAST_PANIC: RefCell<Option<PanicReport>> = const {RefCell::new(None)};
    }

    fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
        if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic payload>".to_string()
        }
    }

    /// Install a panic hook for processing panics, return old one.
    fn install_panic_hook() -> Box<dyn Fn(&PanicHookInfo<'_>) + 'static + Sync + Send> {
        let default_hook = panic::take_hook();

        panic::set_hook(Box::new(move |info: &PanicHookInfo<'_>| {
            let message = panic_payload_to_string(info.payload());

            let location = info.location().map(|loc| PanicLocation {
                file: loc.file().to_string(),
                line: loc.line(),
                column: loc.column(),
            });

            LAST_PANIC.with(|slot| {
                *slot.borrow_mut() = Some(PanicReport { message, location });
            });
        }));
        default_hook
    }

    pub fn run_and_catch<F, R>(f: F) -> Result<R, PanicReport>
    where
        F: FnOnce() -> R,
    {
        LAST_PANIC.with(|slot| {
            *slot.borrow_mut() = None;
        });

        let old_hook = install_panic_hook();

        let res = match panic::catch_unwind(AssertUnwindSafe(f)) {
            Ok(value) => Ok(value),

            Err(payload) => {
                let fallback_message = panic_payload_to_string(payload.as_ref());

                let report = LAST_PANIC.with(|slot| slot.borrow_mut().take());

                Err(report.unwrap_or(PanicReport {
                    message: fallback_message,
                    location: None,
                }))
            }
        };

        panic::set_hook(old_hook);
        res
    }
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use std::str::FromStr as _;

    use proc_macro2::{Literal, TokenTree};
    use syn::Ident;

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
        let out = collect_outputs(generated, None, input.1);
        assert_eq!(out.spans, [OutputEntry::new(false, 0..0)]);
    }

    #[test]
    fn output_preserves_input_relative_spans() {
        let input = parse_input("12").expect("valid token stream");
        let out = collect_outputs(input.0.clone(), None, input.1);
        assert_eq!(out.text, "12");
        assert_eq!(out.spans, [OutputEntry::new(false, 0..2)]);
    }

    #[test]
    fn panic_capture_hook_captures_panic() {
        let report = panic::run_and_catch(|| {
            panic!("test panic");
        })
        .unwrap_err();
        assert_eq!(report.message, "test panic");
        assert!(report.location.is_some());
    }

    #[test]
    fn test_entry_full_flow() {
        let input = "12";
        let body = |mut input: TokenStream| {
            input.extend([Ident::new("foo", Span::call_site())]);
            input
        };
        let output = entry(input, body);
        assert_eq!(output.text, "12 foo");
        assert_eq!(
            output.spans,
            [OutputEntry::new(false, 0..2), OutputEntry::new(false, 0..0)]
        );
    }

    #[test]
    fn test_checks_that_entry_cleanup() {
        let input = "12";
        let body = |mut input: TokenStream| {
            input.extend([Ident::new("foo", Span::call_site())]);
            input
        };
        let output = entry(input, body);
        assert_eq!(output.text, "12 foo");
        // second call should produce same output
        let output = entry(input, body);
        assert_eq!(output.text, "12 foo");
        assert_eq!(
            output.spans,
            [OutputEntry::new(false, 0..2), OutputEntry::new(false, 0..0)]
        );
    }

    #[test]
    #[should_panic(expected = "second panic")]
    fn test_check_panic_hook_cleanup() {
        let input = "12";
        let body = |_: TokenStream| {
            panic!("test panic");
        };
        let output = entry(input, body);
        assert!(output.text.contains("test panic"));
        panic!("second panic");
    }
}
