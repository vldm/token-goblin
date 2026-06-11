#[token_goblin::munch]
fn add(input: TokenStream) -> TokenStream {
    input.ex 
    input
}

#[test]
fn test_add() {
    let result = add!(1 + 2);
    assert_eq!(result, 3);
}
