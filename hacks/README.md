This directory is created to be a documentation, showcase and testbed for some ideas that was used as bricks for implementing this crate.
## `export-path`
Shows, that path imported `some.rs` can include other modules. 

## `dyload`
Shows, how we can move proc-macro to dylib and dynamically load it.

## RUSTC version
Since we using `dyload` proc-macro we need to compile it with the same rust version as the crate that uses it.
That's why we:
1. force same `rustc` to be called for building proc-macro dylib and token-goblin itself.
2. additionally check result of `rustc -vV` to be the same.


## Source hash control
To ensure that macro declaration 

## `proxy-macro`
Shows, how we generate inlinable proc-macro.

## `rust-analyzer-helper`
Shows, how we can emit helper modules to keep our ide-friendly.

## `derive-attr-helper`
Shows, how helper can convert function-like macros into attributes or derives.

## `macro-callback`
Shows, how to provide token stream from one macro to another.

## `reflect`
Shows, how we can collect some information, and then use it in other macros.
