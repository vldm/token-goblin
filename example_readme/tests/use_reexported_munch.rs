#[test]
fn test_use_reexported_munch() {
    use example_readme::super_mega_munch;

    #[super_mega_munch]
    fn add3(mut input: TokenStream) -> TokenStream {
        use std::str::FromStr;
        input.extend(TokenStream::from_str(" + 3"));
        input
    }
    let result = add3!(1 + 2);
    assert_eq!(result, 3 + 3);
}
