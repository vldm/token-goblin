//! Simple macro that returns input tokens
#[token_goblin::munch]
pub fn echo(input: TokenStream) -> TokenStream {
    input
}

#[test]
fn smoke_expansion() {
    let value: i32 = echo!(42);
    echo! {let y = 42;}
    assert_eq!(value, 42);
    assert_eq!(y, 42);
}
