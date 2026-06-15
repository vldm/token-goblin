#[token_goblin::munch]
pub fn add(mut input: TokenStream) -> TokenStream {
    use std::str::FromStr;
    input.extend(TokenStream::from_str(" + 3"));
    input
}

#[token_goblin::munch]
pub fn stringify2(input: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::spanned::Spanned;
    let string = input.to_string();
    quote! {
        pub const FOO: &str = #string;
    }
}

#[derive(token_goblin::Spit)]
#[charm(stringify2)]
pub struct MyStruct {
    pub x: i32,
}

#[test]
fn test_add() {
    let result = add!(1 + 2);
    assert_eq!(result, 3 + 3);
}

#[test]
fn test_spit_derive() {
    use token_goblin::spit;

    assert!(FOO.contains("struct MyStruct"));
    assert!(FOO.contains("pub x : i32"));

    {
        // proc-macro-attribute are destructive unlike derive macros
        #[spit(stringify2)]
        struct MyStruct {}
        assert!(!FOO.contains("pub x : i32"));
    }
}
