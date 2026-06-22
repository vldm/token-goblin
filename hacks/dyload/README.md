# dyload of a proc-macro

This directory shows how proc-macro logic can be moved to a dylib and dynamically loaded.

The real logic of macro expansion is in `dylib-test` crate.
`some-test` defines a main that compiles `dylib-test`, loads it, and calls the `entry` function with a specific token stream.

`dylib-test` is linked to `proc-macro`, whose protocol is not part of the public API.
Therefore, both `dylib-test` and `proc-macro` should be compiled with the same Rust version.
(That's why we can use Rust ABI and dylib instead of cdylib as well.)

This is okay for our case, since the dylibs live only during the compilation process.


Similar approaches:
- watt uses wasm for loading macros.
- inline-proc uses dylib for loading macros.