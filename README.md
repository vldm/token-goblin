# Token Goblin — munches your tokens, spits out macros

`token-goblin` is a proc-macro library for defining proc-macro-like transformations inline, directly inside your crate, without separate proc-macro crate.

It is inspired by crates like `crabtime` and `inline-proc`, but aims to provide a more polished, flexible, and ergonomic API.

## Getting started

Add `token-goblin` to your crate:

```toml
[dependencies]
token-goblin = "0.1.0"
```
Then teach the goblin a new **charm**:

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
In other words, `#[munch]` turns the function foo into a charm that munches input tokens and spits out new tokens.

Note: beacause token-goblin are macros that generate macros, **charm** is used in docs for clarity.

# Inline proc-macro

The mix of `crabtime` and `inline-proc`

1. Uses dylib (or possible wasm) for libraries (like in inline-proc).
- Macro definition is build to: `(dylib + macro_rules! declare($tts) => $lib::proxy!("path_to_dlyb", $tts))`
- Macro caller (processing `$lib::proxy!("path_to_dlyb", $tts)`) is loading dylib and redirect $tts to proxy.
2. Allow extending interface (like in crabtime), e.g. input: (`TokenStream`, String, Vec<String>, or pattern), 
output: `TokenStream`, `String`, print directly to stdout.
- entry point is always `fn entry (TokenStream) -> TokenStream` RUST_ABI.
- if declaration use some customization macro should generate `shim` that will generate valid `entry` function with redirected input, output.

3. Use attributes, and derive through proxy macro.
- support `entry(TokenStream, TokenStream) -> TokenStream` for attributes and derive.
- give interface `$crate::derive(..)`

4. `Reflect!<Type>` - allows collecting derive macro, and extend it in future.
5. Should use cargo build-cache and can work with `cache-proc-macros`.
6. macro declaration should generate `rust-analyzer` shim for better type information.
7. Support workspace dependencies.
8. Support IDE
9. map cargo errors to span information.


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