#[token_goblin::munch]
fn bin_root(_: TokenStream) -> TokenStream {
    TokenStream::new()
}

mod nested {
    #[token_goblin::munch]
    fn bin_nested(_: TokenStream) -> TokenStream {
        TokenStream::new()
    }
}

mod shared_name {
    #[token_goblin::munch]
    fn bin_shared(_: TokenStream) -> TokenStream {
        TokenStream::new()
    }
}

mod shared_mod;

fn main() {}
