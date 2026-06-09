The mix of `crabtime` and `inline-proc`

- [x] Uses dylib for libraries (like in inline-proc).
- [ ] Use wasm for libraries
- [x] Macro definition is build to: `(dylib + macro_rules! declare($tts) => $lib::proxy!("path_to_dlyb", $tts))`
- [x] Macro caller (processing `$lib::proxy!("path_to_dlyb", $tts)`) is loading dylib and redirect $tts to proxy.
- [ ] Allow extending interface (like in crabtime), e.g. input: (`TokenStream`, String, Vec, or pattern), 

- [ ] Allow output to be created streamingly, like `println!`
- [x] entry point is always `fn entry (TokenStream) -> TokenStream` RUST_ABI.
- [ ] if declaration use some customization macro should generate `shim` that will generate valid `entry` function with redirected input, output.

- [ ] Use attributes, and derive through proxy macro.
- [ ] support `entry(TokenStream, TokenStream) -> TokenStream` for attributes and derive.
- [ ] give interface `$crate::derive(..)`

- [ ] `Reflect!<Type>` - allows collecting derive macro, and extend it in future.
- [x] Should use cargo build-cache and
- [ ] can work with `cache-proc-macros`. For cargo it should already work, since we only touch OUT_DIR and generated code is fully depend on input (only ENV - CARGO_*, and extern `mod X` can be questionable)
- [x] (cache related bug): Currently we store only "latest" version of macro expansion. But r-a can expect old expansion to still be available - which lead to wrong source hash.
- [ ] Remove #[allow(unused)]
- [ ] Support IDE: macro declaration should generate `rust-analyzer` shim for better type information.
- [ ] Support workspace dependencies.
- [ ] map cargo errors to span information.
- [ ] Support module resolution ?
- [ ] Add from source_text macro (without spans, but that allow saving original text)