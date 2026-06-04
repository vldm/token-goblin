token_goblin

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


## Compare with other solutions

### crabtime
- Recompile each macro on call site
- Need 

### inline-proc
- No simple one function macro. 