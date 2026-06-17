#[token_goblin::munch]
fn test_root(_: TokenStream) -> TokenStream {
    TokenStream::new()
}

mod nested {
    #[token_goblin::munch]
    fn test_nested(_: TokenStream) -> TokenStream {
        TokenStream::new()
    }
}

#[test]
fn integration_smoke() {
    test_root!();
    module_path_fixture::lib_root!();
}
