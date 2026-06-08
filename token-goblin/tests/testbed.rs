use token_goblin::*;

#[munch]
fn testbed(input: TokenStream) -> TokenStream {
    input
}

macro_rules! testbed2 {
    (inner) => {
        testbed2!(some_inner);
    };
    ($foo:ident) => {
        #[allow(unused_macros, reason = "inner idents cannot be accessed outer")]
        #[munch]
        fn $foo(input: TokenStream) -> TokenStream {
            input
        }
    };
}

testbed2!(foo);

testbed2!(inner);
testbed2!(inner);

mod bar {
    use super::*;
    #[munch]
    fn bar(input: TokenStream) -> TokenStream {
        input
    }

    testbed2!(inner);

    #[test]
    fn test_full() {
        let x = bar!(testbed!(foo!(12)));
        assert_eq!(x, 12);
    }
}

#[test]
fn test_short() {
    let x = foo!(12);
    assert_eq!(x, 12);
}
