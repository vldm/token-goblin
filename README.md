# Token Goblin — munches your tokens, forge out charms

![Token Goblin](assets/token-goblin.png)

`token-goblin` is a proc-macro library for defining inline proc-macro, directly inside your crate, without separate proc-macro target.

It is inspired by crates like `crabtime` and `inline-proc`, but aims to provide a more polished, flexible, and ergonomic API.

## Getting started

Add `token-goblin` to your crate:

```toml
[dependencies]
token-goblin = "0.1.0"
```

Then try:

```rust
#[token_goblin::munch]
fn foo(input: TokenStream) -> TokenStream {
    input
}
```

This generates a new macro, or **charm**, named foo!:
```rust
foo!(bar baz); // will expand to `bar baz`
```

In other words, `#[munch]` turns the function into a new macro.

Note: beacause token-goblin are macros that generate macros, **charm** is used in docs for clarity (and a little bit of lore).

# Usecases

## Simple string based API like in `crabtime`

Some users don't want to mess with `proc-macro` API, they found it foreign and confusing.
`crabtime` showed another way to write macro - a simple string based API, that allows to use `String` and `Vec<String>` dirrectly as input of macro.

Example adopted from `crabtime` docs:
```rust
#[token_goblin::munch]
fn generate_enums(components: CommaSeparated<Token>) {
    let components: Vec<String> = components.into();
    for dim in 1..=components.len() {
        let cons = components[0..dim].join(",");
        output_str! {
            "#[derive(Debug)]
            enum Enum{dim} {{
                {cons}
            }}"
        }
    }
}

generate_enums!["X", "Y", "Z", "W", "V", "U", "T", "S", "R", "Q"];
```

which will expand to:
```rust
enum Enum1 { X }
// ... up to
enum Enum10 { X, Y, Z, W, V, U, T, S, R, Q }
```

Note: while it is inspired by `crabtime`, and `token-goblin` adopted this approach, instead of hardcoding `String`, `Vec<String>` type handling, input is expected to implement `syn::parse::Parse` trait.
So `CommaSeparated<Token>` is just two wrappers in `token-goblin-runtime` crate, that provides required `syn::parse::Parse` implementation.


## Inline proc-macro

String based API is simple, but it's looses span information, and reduces IDE/diagnostics quality.

If you don't want to lose span informations, but it stills annoys you, that to implement a simple
`proc-macro` you need to create a separate crate.
`token-goblin` provides a classic `proc-macro2` API as well:

```rust
#[token_goblin::munch]
fn foo(input: TokenStream) -> TokenStream {
    // ..
}
```
And even better, it's support `syn` based types as input params:
```rust
#[token_goblin::munch]
fn stringify(input: syn::Ident) -> TokenStream {
    let v = input.to_string();
    quote! {
        #v
    }
}
```


<details>
  <summary>Or, you can define multiple `charms` in one module, and extend input param</summary>

    ```rust
    #[token_goblin::munch]
    mod macros {
        struct StructParam {
            // ..
        }
        impl syn::parse::Parse for MyStruct {
            //..
        }
        /// Note: ALL `pub fn`/`pub(crate) fn` are considered as entrypoints.
        /// Note2: No need to write `#[token_goblin::munch]` before each `pub fn`, it's already implied.
        pub fn generate_enums(components: CommaSeparated<Token>) -> TokenStream {
            // ..
        }
        pub fn generate_structs(param: StructParam) -> TokenStream {
            // ..
        }
    }

    macros::generate_enums!["X", "Y", "Z", "W", "V", "U", "T", "S", "R", "Q"];
    macros::generate_structs!{Foo};
    ```


</details>


## TTs muncher replacement

[TTs muncher](https://lukaswirth.dev/tlborm/decl-macros/patterns/tt-muncher.html) is a technique of writing recursive macro-by-examples macros, to parse complex input.

Example from link above (slightly modified):
```rust
macro_rules! trace {
    () => {};

    (trace $name:ident; $($tail:tt)*) => {{
        println!("{} = {:?}", stringify!($name), $name);
        trace!($($tail)*);
    }};

    (trace $name:ident = $value:expr; $($tail:tt)*) => {{
        let $name = $value;
        println!("{} = {:?}", stringify!($name), $name);
        trace!($($tail)*);
    }};
}
```

<details>
  <summary>Expand to see details</summary>

    It expects input in format:

    ```rust
    let a = 10;
    trace! {
        trace x = 2 + 3;
        trace y = x * 10;
        trace x;
        trace y;
    }
    ```
    expands to something like:

    ```rust
    {
        let x = 2 + 3;
        println!("x = {:?}", x);
        {
            let y = x * 10;
            println!("y = {:?}", y);
            {
                println!("x = {:?}", x);
                {
                    println!("y = {:?}", y);
                }
            }
        }
    }
    ```

    and produces output into console:
    ```
    x = 5
    y = 50
    x = 5
    y = 50
    ```

</details>

This macro can be rewritten as:

```rust
#[token_goblin::munch]
fn trace(input: TokenStream) -> TokenStream {
    while
}
```


# Questions

## Why it's named Token Goblin?

During thinkering about name, the ChatGPT 5.5 suggested this variant among others:

![Token Goblin](assets/token-goblin-origin.png)

Which i found ridiculous, especially after i saw [OpenAI post how their fighting "goblin" overuse by ChatGPT](https://openai.com/index/where-the-goblins-came-from/).

Also the idea of "some magical entity that eats tokens" looks like a good metaphor for macros.

## Why entrypoint macros named `munch` and `spit`?

1. Because `munch` and `spit` fit well in "goblin" lore.
2. I think that `#[munch] fn` would be a good replacement for existing [TTs muncher](https://lukaswirth.dev/tlborm/decl-macros/patterns/tt-muncher.html) - technique of writing recursive declarative macros, to parse complex input.

## Compare with other solutions

### crabtime
- Recompile each macro on call site
- Don't work with build-dir cache 

### inline-proc
- No simple one function macro. 

## Testing

Most of tests are implemented as regular integration tests, or doctests dirrectly in macro library.
Fixtures represents tests that need to be run with different environment (currently only toolchain, or cargo config).

Fixtures can be run with:
```bash
cargo test -p token-goblin --test fixtures
```



## Offline build

Note: `token-goblin-runtime` is hardcoded dependency of generated crates, and might be not downloaded using `cargo fetch` or `cargo vendor`, in order to build offline, add `token-goblin-runtime` to `[dev-dependencies]` in your `Cargo.toml`.

# Ceveats:
- only `proc-macro2::fallback` is used (no `proc-macro` api is available) in generated crates (which introduce some limitations)
- mixed_site - is not supported by `proc_macro2::fallback`