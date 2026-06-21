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
        "~ @ token_goblin [] [(stringify)] [Foo => { struct Foo { x : i32, } }] []"
    );
}

#[test]
fn test_snif_macro() {
    let result = stringify_our!(foo);
    assert_eq!(result, "foo");

    let x = token_goblin::snif!(Foo in stringify_our!());

    assert_eq!(x, "[Foo => { struct Foo { x : i32 , } }] []");
}

#[test]
fn stringify_snif_extra_args() {
    // extra arguments is passed before structs.
    // They should be known for macro implementation and finit.
    let result = snif!(Foo in stringify_our!(after struct) );

    assert_eq!(
        result,
        "[Foo => { struct Foo { x : i32 , } }] [after struct]"
    );
}

#[test]
fn allow_using_path() {
    let result = snif!(Foo in foo::stringify!());
    assert_eq!(result, "[Foo => { struct Foo { x : i32 , } }] []");
}

#[test]
fn snif_trait_and_mod() {
    #[token_goblin::derive_snif]
    trait Bar {}

    #[token_goblin::derive_snif]
    mod x {}

    let result = snif!(Bar in stringify_our!());
    assert_eq!(result, "[Bar => { trait Bar { } }] []");

    let result = snif!(x in stringify_our!());
    assert_eq!(result, "[x => { mod x { } }] []");
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
        "[Bar => { struct Bar { x : f32 , } } Foo => { struct Foo { x : i32 , } }] []"
    );
    // or in reverse order
    let result2 = snif!(Foo, Bar in stringify_our!());
    assert_eq!(
        result2,
        "[Foo => { struct Foo { x : i32 , } } Bar => { struct Bar { x : f32 , } }] []"
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

#[test]
fn munch_use_snifed_items() {
    #[derive(token_goblin::Snif)]
    struct Bar {
        x: i32,
        y: u32,
    }

    #[token_goblin::munch(lazy)]
    fn stringify_fields(input: SnifedEntries) -> TokenStream {
        let result = input
            .entries
            .iter()
            .filter_map(|item| match &item.item {
                syn::Item::Struct(struct_item) => Some(struct_item),
                _ => None,
            })
            .flat_map(|f| f.fields.iter())
            .map(|f| f.ident.to_token_stream().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        quote! {
            #result
        }
    }

    let result = snif!(Foo in stringify_fields!());
    assert_eq!(result, "x");

    let result = snif!(Bar in stringify_fields!());
    assert_eq!(result, "x, y");
    let result = snif!(Bar, Foo in stringify_fields!());
}
