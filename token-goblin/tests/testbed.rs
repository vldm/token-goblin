use token_goblin::*;

#[munch]
fn some_macro(input: TokenStream) -> TokenStream {
    input
}

macro_rules! generate_macro {
    (inner) => {
        generate_macro!(some_inner);
    };

    (pub_inner) => {
        #[munch]
        pub fn pub_inner(input: TokenStream) -> TokenStream {
            input
        }
    };
    ($foo:ident) => {
        #[allow(unused_macros, reason = "inner idents cannot be accessed outer")]
        #[munch]
        fn $foo(input: TokenStream) -> TokenStream {
            input
        }
    };
}

generate_macro!(foo);

generate_macro!(baz);
generate_macro!(inner);
generate_macro!(pub_inner);

mod private {
    use super::*;
    #[munch]
    fn bar(input: TokenStream) -> TokenStream {
        input
    }

    // testbed2!(pub_inner);

    // Cannot define macro with same name publicly available.
    generate_macro!(inner);

    #[test]
    fn test_full() {
        let x = bar!(some_macro!(foo!(12)));
        assert_eq!(x, 12);
    }
}

#[test]
fn test_short() {
    let x = foo!(12);
    assert_eq!(x, 12);
}
