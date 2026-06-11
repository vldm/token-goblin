#[token_goblin::munch]
pub fn add(mut input: TokenStream) -> TokenStream {
    use std::str::FromStr;
    input.extend(TokenStream::from_str(" + 3"));
    input
}

#[test]
fn test_add() {
    let result = add!(1 + 2);
    assert_eq!(result, 3 + 3);
}
