#[token_goblin::munch(dependencies = ["heck"])]
pub fn stringify_to_snake(input: TokenStream) -> TokenStream {
    use heck::ToSnakeCase;
    let string = input.to_string();
    let result = string.to_snake_case();
    quote! {
         #input
    }
}

#[test]
fn test_dependencies() {}
