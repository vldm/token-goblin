#[allow(unused, reason = "struct used only in attr")]
#[derive(token_goblin::Snif)]
struct Foo {
    x: i32,
}

mod foo {
    pub use stringify;
}
#[test]
fn stringify_snif() {
    // Internal use of macro.
    let result = Foo! {@token_goblin [(stringify)] []};

    assert_eq!(result, "@ token_goblin [] [{ struct Foo { x : i32, } }]");
}

#[test]
fn stringify_snif_extra_args() {
    // Internal use of macro.
    let result = Foo! {@token_goblin [(stringify)] [before struct]};

    assert_eq!(
        result,
        "@ token_goblin [] [before struct{ struct Foo { x : i32, } }]"
    );
}

#[test]
fn allow_using_path() {
    // Internal use of macro.
    let result = Foo! {@token_goblin [(stringify)] []};

    let result2 = Foo! {@token_goblin [(foo::stringify)] []};
    assert_eq!(result, result2);
    assert_eq!(result, "@ token_goblin [] [{ struct Foo { x : i32, } }]");
}

#[test]
#[allow(unused, reason = "struct used only in attr")]
fn snif_trait_and_mod() {
    #[token_goblin::derive_snif]
    trait Bar {}

    #[token_goblin::derive_snif]
    mod x {}

    let result = Bar! {@token_goblin [(stringify)] []};
    assert_eq!(result, "@ token_goblin [] [{ trait Bar {} }]");

    let result = x! {@token_goblin [(stringify)] []};
    assert_eq!(result, "@ token_goblin [] [{ mod x {} }]");
}

#[test]
fn combine_multiple_snifs() {
    // even if snif is created with attribute macro, and another with derive - they should be compatible.
    #[token_goblin::derive_snif]
    #[allow(unused, reason = "struct used only in attr")]
    struct Bar {
        x: f32,
    }

    let result = Bar! {@token_goblin [(Foo) => (stringify)] []};

    assert_eq!(
        result,
        "@ token_goblin [] [{ struct Bar { x : f32, } } { struct Foo { x : i32, } }]"
    );
    // or in reverse order
    let result2 = Foo! {@token_goblin [(Bar) => (stringify)] []};
    assert_eq!(
        result2,
        "@ token_goblin [] [{ struct Foo { x : i32, } } { struct Bar { x : f32, } }]"
    );
}

#[token_goblin::munch]
fn stringify_custom(input: TokenStream) -> TokenStream {
    let result = input.to_string();
    quote! {
        #result
    }
}

#[test]
fn test_snif_macro() {
    let result = stringify_custom!(foo);
    assert_eq!(result, "foo");

    let x = token_goblin::snif!(Foo in stringify_custom!(arguments before struct));

    assert_eq!(x, "arguments before struct { struct Foo { x : i32 , } }");
}

#[test]
fn test_vanish_macro() {
    #[token_goblin::vanish]
    struct Bar {
        x: i32,
    }
    // there is no struct Bar anymore
    // let y = Bar { x: 42 };

    fn foo() {
        // but this emits the struct Bar again
        Bar! {}
        let y = Bar { x: 42 };
        assert_eq!(y.x, 42);
    }
    foo();
}
