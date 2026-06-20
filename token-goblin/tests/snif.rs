#![allow(unused, reason = "struct used only in attr tests")]

use token_goblin::snif;

#[derive(token_goblin::Snif)]
struct Foo {
    x: i32,
}

#[token_goblin::munch]
fn stringify_our(input: TokenStream) -> TokenStream {
    let result = input.to_string();
    quote! {
        #result
    }
}

mod foo {
    pub(super) use super::stringify_our as stringify;
}
#[test]
fn stringify_snif() {
    // Internal use of macro.
    // we can use any macro here,
    let result = snif!(Foo in stringify!());

    // But there will be internal input.
    assert_eq!(
        result,
        "~ @ token_goblin [(stringify)] [{ struct Foo { x : i32, } }]"
    );
}

#[test]
fn test_snif_macro() {
    let result = stringify_our!(foo);
    assert_eq!(result, "foo");

    let x = token_goblin::snif!(Foo in stringify_our!());

    assert_eq!(x, "{ struct Foo { x : i32 , } }");
}

#[test]
fn stringify_snif_extra_args() {
    // extra arguments is passed before structs.
    // They should be known for macro implementation and finit.
    let result = snif!(Foo in stringify_our!(before struct) );

    assert_eq!(result, "before struct { struct Foo { x : i32 , } }");
}

#[test]
fn allow_using_path() {
    let result = snif!(Foo in foo::stringify!());
    assert_eq!(result, "{ struct Foo { x : i32 , } }");
}

#[test]
fn snif_trait_and_mod() {
    #[token_goblin::derive_snif]
    trait Bar {}

    #[token_goblin::derive_snif]
    mod x {}

    let result = snif!(Bar in stringify_our!());
    assert_eq!(result, "{ trait Bar { } }");

    let result = snif!(x in stringify_our!());
    assert_eq!(result, "{ mod x { } }");
}

// allow attr on whole test to avoid passing it in `#[derive_snif]` inputs
#[test]
fn combine_multiple_snifs() {
    // even if snif is created with attribute macro, and another with derive - they should be compatible.
    #[token_goblin::derive_snif]
    struct Bar {
        x: f32,
    }

    let result = snif!(Bar, Foo in stringify_our!());

    assert_eq!(
        result,
        "{ struct Bar { x : f32 , } } { struct Foo { x : i32 , } }"
    );
    // or in reverse order
    let result2 = snif!(Foo, Bar in stringify_our!());
    assert_eq!(
        result2,
        "{ struct Foo { x : i32 , } } { struct Bar { x : f32 , } }"
    );
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
