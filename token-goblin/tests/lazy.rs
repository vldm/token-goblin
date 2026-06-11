#[token_goblin::munch]
fn stmt(_: TokenStream) -> TokenStream {
    use std::str::FromStr;
    TokenStream::from_str(
        "
        struct Foo {
            x: u32,
        }
    ",
    )
    .unwrap()
}

#[token_goblin::munch(lazy)]
fn stmt_lazy(_: TokenStream) -> TokenStream {
    use std::str::FromStr;
    TokenStream::from_str(
        "
        struct Foo {
            x: u32,
        }
    ",
    )
    .unwrap()
}

#[test]
fn test_stmt() {
    stmt!();
    let y = Foo { x: 12 };
    assert_eq!(y.x, 12);
}

#[test]
fn test_stmt_lazy() {
    stmt_lazy!();
    let y = Foo { x: 12 };
    assert_eq!(y.x, 12);
}
