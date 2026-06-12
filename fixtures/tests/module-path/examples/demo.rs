#[token_goblin::munch]
fn example_root(_: TokenStream) -> TokenStream {
    TokenStream::new()
}

fn main() {
    example_root!();
}
