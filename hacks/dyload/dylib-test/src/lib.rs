// prelude
#[allow(clippy::wildcard_imports, reason = "prelude")]
use token_goblin_runtime::*;

#[unsafe(no_mangle)]
pub fn entry(mut input: TokenStream) -> TokenStream {
    input.extend([Ident::new("baz", Span::call_site())]);
    input
}
