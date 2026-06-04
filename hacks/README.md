This directory is created to be a documentation, showcase and testbed for some ideas that was used as bricks for implementing this crate.

1. `export-path` - shows, that path imported `some.rs` can include other modules. 
2. `dyload` - shows, how we can move proc-macro to dylib and dynamically load it.
2. `proxy-macro` - shows, how we generate inlinable proc-macro.
3. `rust-analyzer-helper` - shows, how we can emit helper modules to keep our ide-friendly.
4. `derive-attr-helper` - shows, how helper can convert function-like macros into attributes or derives.
5. `macro-callback` - shows, how to provide token stream from one macro to another.
6. `reflect` - shows, how we can collect some information, and then use it in other macros.

