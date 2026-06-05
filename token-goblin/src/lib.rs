//! Main `token-goblin` crate. Re-exports will live here later.

pub use token_goblin_macro::munch;

#[munch]
fn foo(v: TokenStream) -> TokenStream {
    v
}

foo!();
