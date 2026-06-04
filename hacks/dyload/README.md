# dyload of proc-macro

This directory shows, how proc-macro logic can be moved to dylib and dynamically loaded.

The real logic of macro expansion is in `dylib-test` crate.
`some-test` define a main that compile `dylib-test` load it and call `entry` function with some specific token stream.

`dylib-test` is linked to `proc-macro`, which protocol is not part of public API.
Therefore both `dylib-test` and `proc-macro` should be compiled with the same rust version.
(Thats why we can use Rust ABI and dylib instead of cdylib as well)

This is okay for our case, since our dylibs is lived during compilation process.


Similar aproaches:
- watt - uses wasm for loading macros.
- inline-proc - uses dylib for loading macros.