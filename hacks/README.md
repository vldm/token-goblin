This directory is created to be a documentation, showcase and testbed for some ideas that was used as bricks for implementing this crate.

The documentation in this folder is internal, to describe how it was implemented.
For usecases checkouts `token-goblin` crate.

# [`export-path`](export-path/README.md)
Shows, that path imported `some.rs` can include other modules. 

# [`dyload`](dyload/README.md)
Shows, how we can move proc-macro to dylib and dynamically load it.

## RUSTC version
Since we using `dyload` we need to compile library with the same rust version as the host that uses it.
That's why we:
1. force same `rustc` to be called for building proc-macro dylib and token-goblin itself, by setting `RUSTC` and `CARGO` env variable, that was used in building `token-goblin` itself.
2. additionally check result of `rustc -vV` to be the same.

## Source hash control
Since process of `charm` expansion and declaration is separated in time, we need to ensure that macro declaration and caller expect same macro.
We add source hash into macro name, and include it into proxy macro call.

## Cross lib boundary
Dyload also limits us in interface that we can provide to the definition crate.
1. `proc-macro` is not available in definition crate (since proc-macro uses tls to store route to host runtime)
2. Sending `proc-macro2::fallback::TokenStream` directly is possible, but it will lose some diagnostics information, related to spans location (also caused by tls usage in proc-macro2::fallback)
3. `token-goblin` uses `String` to pass input and build source map, that later used to recover original spans. (more info in `token-goblin/src/span_recovery.rs`) 

# Visibiltity hack

Historically `macro_rules` have it's own namespace. Therefore exporting them 
require `#[macro_export]` attribute.
To allow users to use `charm` with regular visibility, we use hack, that automatically
adds `#[macro_export]` attribute to the macro declaration, when needed.
Actually any `charm` is expanded into `macro_rules!` with special names, that prevent name collisions, and then exported with needed visibility.
```rust
#[macro_export] // automatically added when needed
macro_rules! foo_<hash> {
    ...
}
pub(..) use foo_<hash> as foo;
```


# Proxy macro
Macro declaration is expanded into `macro_rules!` with fn name.
```rust
#[token_goblin::munch]
fn foo(input: TokenStream) -> TokenStream {
    input
}
```
will expand to:
```rust
macro_rules! foo {..}
```
To allow `foo!` use rust code defined earler, all calls to `foo!` are redirected to `token-goblin::proxy!` macro
which loads needed dylib, and handle input/output conversion.


# `rust-analyzer-helper`
For IDE support, we emit helper modules.
```rust
mod __ide_tg_helper {
    /// prelude
    fn foo(input: TokenStream) -> TokenStream {
        input
    }
}
```

This allows rust-analyzer to suggest correct types, and provide code completion.
For better completion support, body of fn kept unparsed.

So fn like this is parsable:
```rust
fn foo(input: TokenStream) -> TokenStream {
   input.<caret>
}
``` 



## `derive-attr-helper`
Shows, how helper can convert function-like macros into attributes or derives.

## `macro-callback`
Shows, how to provide token stream from one macro to another.

## `reflect`
Shows, how we can collect some information, and then use it in other macros.
