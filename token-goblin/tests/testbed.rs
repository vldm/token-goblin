use token_goblin::*;

// generate some macro
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
        #[munch]
        fn $foo(input: TokenStream) -> TokenStream {
            input
        }
    };
}

// Macro can be generated.
generate_macro!(foo);
generate_macro!(baz);

#[allow(
    unused_imports,
    reason = "some_inner ident is not available outside of generate_macro! macro"
)]
mod inner {
    use super::*;
    generate_macro!(inner);
    generate_macro!(pub_inner);
}

mod private {
    use super::{baz, munch, some_macro};
    #[munch]
    fn bar(input: TokenStream) -> TokenStream {
        input
    }

    // Allow to define macro with same name again.
    // but only if it's private
    #[allow(unused_imports)]
    mod scope {
        use super::*;
        generate_macro!(foo);
    }
    // Public macro can also be defined multiple times
    // If their "source spans" are different.
    #[munch]
    pub fn foo(input: TokenStream) -> TokenStream {
        input
    }

    // Cannot define macro with same name publicly available.
    // generate_macro!(pub_inner);

    #[test]
    fn test_full() {
        let x = bar!(some_macro!(baz!(foo!(12))));
        assert_eq!(x, 12);
    }
}

#[test]
fn test_short() {
    let x = foo!(12);
    assert_eq!(x, 12);
}

#[munch]
mod module {

    pub fn module_macro(input: TokenStream) -> TokenStream {
        input
    }
}

#[test]
fn test_module() {
    let x = module::module_macro!(12);
    assert_eq!(x, 12);
}
