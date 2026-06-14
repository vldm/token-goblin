

Done:
- [x] Should use cargo build-cache and
- [x] can work with `cache-proc-macros`. For cargo it should already work, since we only touch OUT_DIR and generated code is fully depend on input (we use env that is used by cargo - ENV - CARGO_*, so if it changes - cargo will triger rebuild) 
- [x] (cache related bug): Currently we store only "latest" version of macro expansion. But r-a can expect old expansion to still be available - which lead to wrong source hash.
- [x] Remove #[allow(unused)]
- [x] Support IDE: macro declaration should generate `rust-analyzer` shim for better type information.
- [x] Support workspace dependencies.
- [x] map cargo errors to span information.
- [x] entry point is always `fn entry (TokenStream) -> TokenStream` RUST_ABI.
- [x] Macro definition is build to: `(dylib + macro_rules! declare($tts) => $lib::proxy!("path_to_dlyb", $tts))`
- [x] Macro caller (processing `$lib::proxy!("path_to_dlyb", $tts)`) is loading dylib and redirect $tts to proxy.
- [x] Uses dylib for libraries (like in inline-proc).
- [x] if declaration use some customization macro should generate `shim` that will generate valid `entry` function with redirected input, output.
- [x] give interface `$crate::derive(..)` = spit
- [x] Use attributes, and derive through proxy macro.
- [x] Extend api: `fn module_path(span: Span) -> String`

UX:

- [ ] Extend spit interface to receive some extra params `#[charm(via = macro)]`, and params like `#[charm(other = ..)]`, for attribute like receive args in format `#[spit(macro(args,..))]`
- [ ] Implement Snif 
- [x] Allow extending interface (like in crabtime), e.g. input: (`TokenStream`, String, Vec, or `syn::Parsable` types), 
- [x] Allow output to be created streamingly, like `println!`


Features:
- [ ] support `entry(TokenStream, TokenStream) -> TokenStream` for attributes and derive.
- [ ] `Reflect!<Type>` - allows collecting derive macro, and extend it in future.
- [ ] Implement better diagnostics, e.g. panic handling, and cargo errors should be converted to spans and passed as compile errors.
- [ ] Support of `mod X` in `#[munch] mod foo { .. }` should import module related to foo, from external file only.
- [ ] Optional dependencies.

Consider this:
- [ ] Use wasm for libraries
- [x] Support module resolution ?
- [ ] Add from source_text macro (without spans, but that allow saving original text)

Nice to have:
- [ ] check compatibility of `cache-proc-macros` and extern `mod X` 