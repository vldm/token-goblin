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

## Inline proc-macro

While `proc-macro` provides an Rust api to write custom macros, it's anoying that for small macros you still need to create
a separate crate.

Example adopted from `crabtime` docs:
```rust
#[token_goblin::munch]
fn generate_enums(components: Vec<String>) -> TokenStream {
    let mut result = vec![];
    for dim in 1..=components.len() {
        let cons = components[0..dim].join(",");
        result.push(format!("#[derive(Debug)] enum Enum{dim} {{ {cons} }}"));
    }
    TokenStream::from_str(&result.join("\n")).unwrap()
}

generate_enums!["X", "Y", "Z", "W", "V", "U", "T", "S", "R", "Q"];
```
which will expand to:
```rust
enum Enum1 { X }
// ... up to
enum Enum10 { X, Y, Z, W, V, U, T, S, R, Q }
```

Note: like `crabtime`, `token-goblin` allows simple "string" based api.

## Eval macro
While token-goblin doesn't provide you `eval!` macro, it's simple to implement it yourself:


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