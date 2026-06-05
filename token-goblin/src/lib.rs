//! Main `token-goblin` crate. Re-exports will live here later.

pub use token_goblin_macro::munch;

#[munch(split_cache = true)]
fn foo(v: TokenStream) -> TokenStream {
    v
}

const X: usize = foo!(12);
const _ASSERT: () = assert!(X == 12);
