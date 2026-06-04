use proc_macro::TokenStream;
#[macro_use]
mod errors;

type Result<T, E = errors::Error> = std::result::Result<T, E>;

mod metadata;
mod path;

/// Set to 'true' to enable debug prints.
#[allow(unexpected_cfgs, reason = "custom made config")]
pub(crate) const DEBUG: bool = false || cfg!(crabtime_debug);

pub(crate) const OUT_DIR: &str = env!("OUT_DIR");

// ===============================
// Macros entry points
// ===============================
#[proc_macro]
pub fn proxy(input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn munch(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_derive(Snif)]
pub fn snif(input: TokenStream) -> TokenStream {
    input
}
