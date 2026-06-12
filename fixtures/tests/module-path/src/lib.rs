#[token_goblin::munch]
pub fn lib_root(_: TokenStream) -> TokenStream {
    TokenStream::new()
}

mod nested {
    #[token_goblin::munch]
    pub fn lib_nested(_: TokenStream) -> TokenStream {
        TokenStream::new()
    }
}

mod shared_name {
    #[token_goblin::munch]
    pub fn lib_shared(_: TokenStream) -> TokenStream {
        TokenStream::new()
    }
}

mod shared_mod;
