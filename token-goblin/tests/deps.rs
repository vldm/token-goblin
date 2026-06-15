#[token_goblin::munch(dependencies = ["heck"], no_ide_helper)]
pub fn stringify_to_snake(input: TokenStream) -> TokenStream {
    use heck::ToSnakeCase;
    let string = input.to_string();
    let result = string.to_snake_case();
    quote! {
         #result
    }
}

#[test]
fn test_dependencies() {
    // amount of space between words is not relevant, since input is formatted as tokenstream
    let result = stringify_to_snake!(Hello World);
    assert_eq!(result, "hello_world");
}
