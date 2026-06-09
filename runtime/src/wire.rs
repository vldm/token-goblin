//! Guest-side input parsing for the dylib wire format.

use proc_macro2::{LexError, TokenStream};

/// Parse canonical host input text into a local fallback token stream.
pub fn parse_input(source: &str) -> Result<TokenStream, LexError> {
    source.parse()
}
