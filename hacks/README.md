This directory is documentation, a showcase, and a testbed for some ideas that were used as building blocks for this crate.

The documentation in this folder is internal and describes how the crate was implemented.
For use cases, check the `token-goblin` crate.

# [`export-path`](export-path/README.md)
Shows that a path-imported `some.rs` can include other modules.

# [`dyload`](dyload/README.md)
Shows how we can move a proc-macro to a dylib and dynamically load it.

## RUSTC version
Since we use `dyload`, we need to compile the library with the same Rust version as the host that uses it.
That's why we:
1. force the same `rustc` to be used for building the proc-macro dylib and token-goblin itself by setting the `RUSTC` and `CARGO` environment variables that were used to build `token-goblin` itself.
2. additionally check that the result of `rustc -vV` is the same.

## Source hash control
Since the process of `charm` expansion and declaration is separated in time, we need to ensure that the macro declaration and caller expect the same macro.
We add the source hash to the macro name and include it in the proxy macro call.

## Cross lib boundary
`dyload` also limits the interface that we can provide to the definition crate.
1. `proc-macro` is not available in the definition crate, since proc-macro uses TLS to store the route to the host runtime.
2. Sending `proc-macro2::fallback::TokenStream` directly is possible, but it will lose some diagnostic information related to span locations, also because of TLS usage in `proc-macro2::fallback`.
3. `token-goblin` uses `String` to pass input and build a source map that is later used to recover original spans. More info is in `token-goblin/src/span_recovery.rs`.

# Visibility hack

Historically, `macro_rules` has its own namespace. Therefore, exporting them
requires the `#[macro_export]` attribute.
To allow users to use `charm` with regular visibility, we use a hack that automatically
adds the `#[macro_export]` attribute to the macro declaration when needed.
Each `charm` is expanded into `macro_rules!` with special names that prevent name collisions and is then exported with the needed visibility.
```rust
#[macro_export] // automatically added when needed
macro_rules! foo_<hash> {
    ...
}
pub(..) use foo_<hash> as foo;
```


# Proxy macro
Macro declaration is expanded into `macro_rules!` with the function name.
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
To allow `foo!` to use Rust code defined earlier, all calls to `foo!` are redirected to the `token-goblin::proxy!` macro,
which loads the needed dylib and handles input/output conversion.


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

This allows rust-analyzer to suggest correct types and provide code completion.
For better completion support, the function body is kept unparsed.

So a function like this is parsable:
```rust
fn foo(input: TokenStream) -> TokenStream {
   input.<caret>
}
```

# Recovering span information

TBD


## `derive-attr-helper`
Shows how a helper can convert function-like macros into attributes or derives.

## `macro-callback`

Sometimes you need to pass the output of one macro to another.
Passing it like this:
```rust
macro_rules! foo { ... }

stringify!(foo!(...))
```
will produce the string literal `foo!(...)` instead of first expanding the `foo!` macro.

Instead, one can write a `foo` macro that receives a path to another macro:
```rust 
macro_rules! foo {
    ($path:ident => $($tt:tt)*) => {
        $path!($($tt)*)
    };
}

foo!(stringify => ...);
```

This expands to:
```rust
stringify!(...)
```

This technique allows some level of modularity in macros.

## `reflect`
Shows how we can collect some information and then use it in other macros.
