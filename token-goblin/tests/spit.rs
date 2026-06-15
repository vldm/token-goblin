#[token_goblin::munch]
pub fn add_pub(input: TokenStream) -> TokenStream {
    quote! {
        pub #input
    }
}

#[test]
fn test_spit() {
    #[allow(dead_code)]
    mod x {
        super::add_pub!(
            struct Foo {
                pub x: i32,
                pub y: i32,
            }
        );

        #[token_goblin::spit(super::add_pub)]
        struct Bar {
            pub x: i32,
        }

        struct Baz {
            pub x: i32,
        }
    }

    let result = x::Foo { x: 12, y: 13 };
    assert_eq!(result.x, 12);

    let result = x::Bar { x: 12 };
    assert_eq!(result.x, 12);

    // Baz is private, so it's not visible outside of module `x`
    // let result = x::Baz { x: 12 };
    // assert_eq!(result.x, 12);
}
