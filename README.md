# Token Goblin — munches your tokens, forges charms

![Token Goblin](assets/token-goblin.png)

`token-goblin` is a proc-macro library for defining inline proc-macros directly inside your crate, without a separate proc-macro target.

It is inspired by crates like `crabtime` and `inline-proc`, but aims to provide a more polished, flexible, and ergonomic API.

## Getting started

Add `token-goblin` to your crate:

```toml
[dependencies]
token-goblin = "0.1.0"
```

Then try:

```rust
#[token_goblin::munch]
fn foo(input: TokenStream) -> TokenStream {
    input
}
```

This generates a new macro, or **charm**, named `foo!`:

```rust
foo!(bar baz); // will expand to `bar baz`
```

In other words, `#[munch]` turns the function into a new macro.

Note: because `token-goblin::munch` macros generate macros, the term **charm** is used for generated macros in the docs for clarity (and a little bit of lore).

# Use Cases

*A well-fed goblin is a productive goblin. Here is what it does once it has chewed through your tokens.*

## Simple string-based API like in `crabtime`

Some users don't want to mess with the `proc-macro` API; they find it foreign and confusing.
`crabtime` showed another way to write macros: a simple string-based API that allows `String` and `Vec<String>` to be used directly as macro input.

Example adapted from the `crabtime` docs:

```rust
#[token_goblin::munch]
fn generate_enums(components: CommaSeparated<Token>) {
    let components: Vec<String> = components.into();
    for dim in 1..=components.len() {
        let cons = components[0..dim].join(",");
        output_str! {
            "#[derive(Debug)]
            enum Enum{dim} {{
                {cons}
            }}"
        }
    }
}

generate_enums!["X", "Y", "Z", "W", "V", "U", "T", "S", "R", "Q"];
```

which will expand to:

```rust
enum Enum1 { X }
// ... up to
enum Enum10 { X, Y, Z, W, V, U, T, S, R, Q }
```

Note: while this is inspired by `crabtime`, `token-goblin` adopts the approach without hardcoding `String` or `Vec<String>` handling. Instead, **input is expected to implement the `syn::parse::Parse` trait**.
So `CommaSeparated<Token>` is just two wrappers in the `token-goblin-runtime` crate that provide the required `syn::parse::Parse` implementation.

## Inline proc-macro

The string-based API is simple, but it loses span information and reduces IDE/diagnostics quality.

If you don't want to lose span information, but are still annoyed that implementing a simple
`proc-macro` requires a separate crate, `token-goblin` provides a classic `proc-macro2` API as well:

```rust
#[token_goblin::munch]
fn foo(input: TokenStream) -> TokenStream {
    // ..
}
```

And even better, it supports `syn`-based types as input parameters:

```rust
#[token_goblin::munch]
fn stringify(input: syn::Ident) -> TokenStream {
    let v = input.to_string();
    quote! {
        #v
    }
}
```

Or, you can define multiple `charms` in one module and extend the input parameters.

<details>
  <summary>Or, you can define multiple `charms` in one module and extend the input parameters</summary>

```rust
#[token_goblin::munch]
mod macros {
    struct StructParam {
        // ..
    }
    impl syn::parse::Parse for MyStruct {
        //..
    }
    /// Note: ALL `pub fn`/`pub(crate) fn` are considered as entrypoints.
    /// Note2: No need to write `#[token_goblin::munch]` before each `pub fn`, it's already implied.
    pub fn generate_enums(components: CommaSeparated<Token>) -> TokenStream {
        // ..
    }
    pub fn generate_structs(param: StructParam) -> TokenStream {
        // ..
    }
}

macros::generate_enums!["X", "Y", "Z", "W", "V", "U", "T", "S", "R", "Q"];
macros::generate_structs!{Foo};
```


</details>

## Probes and Evals

*Sometimes the goblin just sits by the fire and counts things in its head, so you don't have to at runtime.*

Another common use case for macros is to precompute some data.
`crabtime` provides an `eval` macro for this purpose.

But with `token-goblin`, you can implement it yourself:
```rust
macro_rules! eval {
    ($($expr:tt)*) => {
        {
            #[token_goblin::munch(lazy)]
            fn eval_inner(_: TokenStream) -> TokenStream {
                use std::str::FromStr;
                let x = $($expr)*;
                quote!{ #x }
            }
            eval_inner!($($expr)*)
        }
    };
}

fn main() {
    // Example from crabtime docs:
    let x = eval!((std::f32::consts::PI.sqrt() * 10.0).round() as usize);
    println!("x: {x}");
}
// prints:
// x: 18
```

Note: any expression is embedded into the charm as code and cannot use external variables or call functions from your crate.

<details>
<summary>Some cursed examples of using proc-macros</summary>

But you are not limited to simple expressions. In fact, you can do any compile-time execution, like
evaluating bytecode or even downloading something from the internet (using external state in a macro is not recommended though).

e.g. from [example_readme/examples/brainfuck.rs](example_readme/examples/brainfuck.rs)

```rust
#[token_goblin::munch]
mod brainfuck {
    pub fn execute(input: ProgramInput) -> TokenStream {
        // ..
    }

    pub fn request_and_execute(input: ProgramInput) -> TokenStream {
        // Handle program field as URL.
        let url = String::from_utf8(input.program.value()).unwrap();
        let program = reqwest::blocking::get(url).unwrap().text().unwrap();
        execute(ProgramInput {
            program: syn::LitByteStr::new(&program.as_bytes(), Span::call_site()),
            input: input.input,
        })
    }
}
```

```rust
    let result = brainfuck::request_and_execute!(b"https://gist.githubusercontent.com/vldm/f796f0d6235a608c0bed5957d146f8c0/raw/a068d4a8b2764fbc02b909322f31321b1b7eb7fc/reverse.bf", b"\n!dlroW olleH");
    println!("result: {result}");
    // downloads: ">,[>,]<[.<]" program that reverses input
    // prints:
    // result: Hello World!
```

Executing a brainfuck program is purely functional and therefore fits `proc-macro` purposes well, but using system APIs and requesting external data is clearly misuse. The whole crate is an experiment around `proc-macro`, so I think it's fun to
showcase it as well.

Note: while `token-goblin` itself doesn't cache the output of `charms`, Rust itself might cache them, especially when `-Zcache-proc-macros` is enabled.

Note: there is a plan to implement `wasm` as a feature that will enforce sandboxing of `charms`.

</details>


## Reflection?

*In computer science, reflective programming or reflection is the ability of a process to examine, introspect, and modify its own structure and behavior.*

Reflection is a powerful feature that allows code to be generated dynamically without knowing the exact types by observing their structure.
Zig has the `comptime` keyword, which allows code to execute at compile time and observe the structure of the code.
In Rust, we only have derives. They can replace some kinds of reflection, e.g. by providing a way to generate some traits based on `struct` fields. The missing piece is that they are not extendable.
For example, the person who writes `struct Foo` defines the list of derived traits, and this list is not extendable.

So if you want to extend some type with your custom trait, you need to duplicate the `Foo` definition somewhere in some form. Reflection could solve this problem by providing the `shape` of the type and then generating the trait based on it.

`token-goblin` has a similar feature called `Snif`, which allows you to collect information about a type and pass it to another macro.

```rust
#[derive(token_goblin::Snif)]
struct Foo {
    x: i32,
}
#[token_goblin::munch(lazy)]
fn generate_getters(input: SnifedEntries) -> TokenStream {
    let syn::Item::Struct(item) = &input.entries[0].item else {
        return syn::Error::new(input.span(), "Expected struct").to_compile_error();
    };
    let name = &item.ident;
    let (fields, types): (Vec<syn::Ident>, Vec<syn::Type>) = item
        .fields
        .iter()
        .cloned()
        .map(|field| (field.ident.unwrap(), field.ty))
        .unzip();
    quote! {
        impl #name {
            #(
                pub fn #fields(&self) -> &#types {
                    &self.#fields
                }
            )*
        }
    }
}

token_goblin::snif!(Foo in generate_getters!(extra args));
```

`generate_getters!()` will receive input in this format:
`[Foo => { struct Foo { x : i32, } }] [ extra args]`

It can then generate code based on the information about types (in this example, it generates getters for `Foo`).

This example can be found in [example_readme/examples/generate_getters.rs](example_readme/examples/generate_getters.rs).

A more complete example (MultiArrayList-like from Zig) that converts an array of structs into a struct of arrays can be found in [token-goblin/examples/struct_of_arrays.rs](token-goblin/examples/struct_of_arrays.rs).


## Multiple Small Derives

Sometimes in big projects, you need to define multiple small derives, e.g. parsing/emitting/printing functionality is distinct and should be separated. Placing all of them in one "macro" crate might not be the best choice.
Instead, `token-goblin` allows you to split the logic into multiple "macro" crates and use them as dependencies.

```rust
#[derive(token_goblin::Snif)]
struct Foo {
    x: i32,
}

#[token_goblin::munch]
fn generate_parser(input: SnifedEntries) -> TokenStream {
   // ..
}

#[token_goblin::munch]
fn generate_emitter(input: SnifedEntries) -> TokenStream {
}

token_goblin::snif!(Foo in generate_parser!());
token_goblin::snif!(Foo in generate_emitter!());
```

This approach is partially shown in [token-goblin/examples/struct_of_arrays.rs](token-goblin/examples/struct_of_arrays.rs).
I also use it in a real project, where I want to extend my type with additional metadata while keeping derive logic separated. It looks like this:
```rust
#[derive(token_goblin::Snif)]
enum Expr {
    #[snif(mnemonic = "lit")]
    #[snif(arity = 0 -> 1)]
    Lit(syn::Lit),
    #[snif(mnemonic = "add")]
    #[snif(arity = 2 -> 1)]
    Add(Box<Expr>, Box<Expr>),
}
mod printer {
  trait Printer {}
  snif!(Expr in generate_printer!());
}
// ..
```

## Do I need to rewrite declarative macros to the proc-macro API?

While the proc-macro API is more Rust-like and powerful, one might want to rewrite all declarative macros to the proc-macro API.
But working with `TokenStream` introduces some boilerplate, and some macros should be kept declarative.

<details>
<summary>Example of a TTs muncher rewrite</summary>


[TTs muncher](https://lukaswirth.dev/tlborm/decl-macros/patterns/tt-muncher.html) is a technique for writing recursive declarative macros to parse complex input.

If we take the example from the link above (slightly modified):

```rust
macro_rules! trace {
    () => {};

    ($name:ident; $($tail:tt)*) => {{
        println!("{} = {:?}", stringify!($name), $name);
        trace!($($tail)*);
    }};

    ($name:ident = $value:expr; $($tail:tt)*) => {{
        let $name = $value;
        println!("{} = {:?}", stringify!($name), $name);
        trace!($($tail)*);
    }};
}
```

It expects input in this format:

```rust
let a = 10;
trace! {
    x = 2 + 3;
    y = x * 10;
    x;
    y;
}
```
expands to something like:

```rust
{
    let x = 2 + 3;
    println!("x = {:?}", x);
    {
        let y = x * 10;
        println!("y = {:?}", y);
        {
            println!("x = {:?}", x);
            {
                println!("y = {:?}", y);
            }
        }
    }
}
```

and produces this output in the console:
```
x = 5
y = 50
x = 5
y = 50
```


Rewriting it as a proc-macro `TokenStream` API would increase the amount of code and add a lot of boilerplate:

```rust
#[token_goblin::munch]
fn trace_cycle(input: TokenStream) {
    let mut iter = input.into_iter().peekable();

    while iter.peek().is_some() {
        let Some(TokenTree::Group(g)) = iter.next() else {
            panic!("Expected group");
        };
        let Some(TokenTree::Ident(ident)) = iter.next() else {
            panic!("Expected ident");
        };
        let mut expr = (&mut iter)
            .take_while(|token| !matches!(token, TokenTree::Punct(p) if p.as_char() == ';'))
            .collect::<Vec<_>>();

        let let_stmt = if expr.is_empty() {
            quote! {}
        } else {
            quote! {
                let #ident  #(#expr)*;
            }
        };
        let ident_str = ident.to_string();
        output! {
            #let_stmt;
            writeln!(#g, "{} = {:?}", #ident_str, #ident).ok();
        }
    }
    if iter.peek().is_some() {
        panic!("Expected end of input");
    }
}
```

Using `syn` with `syn-derive` might help with the main logic:

```rust
pub fn trace_syn(input: TraceInput) -> TokenStream {
    let mut out = TokenStream::new();

    for TraceStmt {
        writer,
        ident,
        value,
    } in input.0
    {
        let ident_str = ident.to_string();

        let let_stmt = match value {
            TraceValue::Some { expr, .. } => quote! { let #ident = #expr; },
            TraceValue::None => quote! {},
        };

        out.extend(quote! {
            #let_stmt
            writeln!(#writer, "{} = {:?}", #ident_str, #ident).ok();
        });
    }

    out
}
```

It still requires defining `TraceInput` and `TraceStmt` structs, and `syn::parse::Parse` implementation for them.
See [example_readme/examples/ttmunch-replace.rs](example_readme/examples/ttmunch-replace.rs) for more details.

</details>

With `token-goblin`, you don't need to choose, since it allows you to combine both approaches.

For example, you can write a declarative macro as a facade that checks patterns and computes results in the `proc-macro` API.

```rust
#[token_goblin::munch]
pub fn stringify_any(input: TokenStream) -> TokenStream {
    let string = input.to_string();
    quote! {
        #string
    }
}

macro_rules! stringify_ident {
    ($ident:ident) => {
        stringify_any!($ident)
    };
}

fn main() {
    // this will fail at compile time, due to wrong input pattern
    // let result = stringify_ident!("non ident");
    // let result = stringify_ident!(foo asd);
    let result = stringify_ident!(foo);
    println!("result: {result}");
}
```

Uncommenting non-ident expansions will fail at compile time:
![fails](assets/decl-proc-fail.png)

There is still the old but good `proc-macro-rules` crate, which allows you to use declarative macro patterns directly in the proc-macro API.

# Questions

## Why is it named Token Goblin?

While tinkering with the name, ChatGPT 5.5 suggested this variant among others:

![Token Goblin](assets/token-goblin-origin.png)

I found it ridiculous, especially after I saw [OpenAI's post on how they are fighting "goblin" overuse by ChatGPT](https://openai.com/index/where-the-goblins-came-from/).

Also, the idea of "some magical entity that eats tokens" seems like a good metaphor for macros.

## Why entrypoint macros named `munch` and `spit`?

1. Because `munch` and `spit` fit well in "goblin" lore.
2. I think that `#[munch] fn` would be a good replacement for the existing [TTs muncher](https://lukaswirth.dev/tlborm/decl-macros/patterns/tt-muncher.html), a technique for writing recursive declarative macros to parse complex input.

## Why not use `crabtime` or `inline-proc`?

They both look unmaintained.

`inline-proc` uses syn 1.0 and has had no updates for ~5-6 years. It doesn't compile anymore on modern Rust versions.

I have tried to contribute to `crabtime` at https://github.com/wdanilo/crabtime/issues?q=author%3Avldm,
but it looks like the author is not interested in maintaining it anymore. There are still issues related to the build cache.

`token-goblin` combines all the features from both `crabtime` and `inline-proc`, like:

- using dylib to load proc-macro definition
- support for workspace dependencies
- support for attributes and derive macro helpers
- mod and fn entrypoints

It also adds some extra:

- Emit ide helper for Rust-Analyzer completion [ide-helper](token-goblin/src/ide_support.rs)
- Allow span information to be preserved in output [span_recovery](token-goblin/src/span_recovery.rs)
- Convert any panic to compile error [panic](runtime/src/wire.rs#L185)
- Extensible interface for input and output [ux](runtime/src/ux.rs)
- A "reflection"-like macro to store tokens of some items and use them as input to another macro [snif](token-goblin/src/lib.rs#L311).

And more is planned:

- Mapping panics/compile errors to `compile_error!` should show any error at the right source location.
- Support for `wasm` as a feature that will enforce sandboxing of `charms`.

## Testing

Most tests are implemented as regular integration tests or doctests directly in the macro library.
Fixtures represent tests that need to be run with different environments (currently only toolchain or Cargo config).

Fixtures can be run with:

```bash
cargo test -p token-goblin --test fixtures
```

# Usage recommendations

Some hints and recommendations for using `token-goblin` in your projects.

## IDE support

As with offline builds, it is recommended to add `token-goblin-runtime` to `[dev-dependencies]` in your `Cargo.toml`. This will help rust-analyzer find the needed crate and provide important semantic information for your macros.

## Laziness

`token-goblin::munch` provides a `lazy` attribute that allows enforcing laziness of charm compilation.

By default, all charms generated by `token-goblin::munch` are eager. This means that a charm is compiled during expansion of the `#[munch]` attribute, and users of the `charm` only use the compiled dylib.

This setup is faster, since `charm` is compiled only once, and every user (expansion of `charm` itself) skips the compilation step.

But during development, flycheck could call `cargo check` on broken code and spam errors. In VS Code + Lens, this can slow down IDE performance.
Therefore, you can set `lazy` to `true` in `#[token_goblin::munch]` attributes.

```rust
#[token_goblin::munch(lazy)] // or #[token_goblin::munch(lazy = true)]
fn foo(input: TokenStream) -> TokenStream {
    // ..
}
```

With this setup, `#[munch]` will not compile the `charm`; instead, compilation will be triggered during `foo` expansion.
Note: for the same code, compilation is only triggered once, since `token-goblin` caches the compiled dylib.

## Debugging

You can also set the `TOKEN_GOBLIN_PRINT_LEVEL` environment variable to `1-4` to enable debug prints.
```bash
1 - print basic info
2 - print timings
3 - print input and output of internal macros
4 - print environment variables
```

You can also use `println` / `eprintln` / `dbg` and other macros to debug your charms.


## Share cache or not?

By default, all charms generated by `token-goblin::munch` share the same build-cache directory.
Sharing a cache forces Cargo to lock the directory, so one "slow" charm can slow down the whole compilation process.

To avoid this, you can set `split_cache` to `true` in `#[token_goblin::munch]` attributes.

```rust
#[token_goblin::munch(split_cache)] // or #[token_goblin::munch(split_cache = true)]
fn foo(input: TokenStream) -> TokenStream {
    // ..
}
```

This will force the charm to use a separate build-cache directory, so it will not be affected by other charms.

I recommend using `split_cache` only for "big" charms that require a lot of dependencies or take a lot of time to build. This is because a charm with a separate cache can be compiled in parallel with other charms.

# Caveats

*Even a helpful goblin has its quirks. Mind these before you let it loose.*

- only `proc-macro2::fallback` is used (no `proc-macro` API is available) in generated crates, which introduces some limitations.
- `mixed_site` is not supported by `proc_macro2::fallback`.
- we use `dev-dependencies` for `charm` dependencies, which cannot be optional by design of the Cargo resolver, so one small macro may increase compile time by rebuilding all `dev-dependencies`.
- `name` in `#[munch] fn name` should not be proc-macro generated and is expected to have a local source file.
- on macOS, loading `dylibs` (newly generated charms) may take more time than compilation itself (~300ms). This is a [known issue](https://nnethercote.github.io/2025/09/04/faster-rust-builds-on-mac.html) related to XProtect. See the link above for a workaround.
- rust-analyzer will not analyze "optional" dependencies and will emit **"unresolved external crate"** errors on charms.
To disable IDE support for charms, use the `no_ide_helper` attribute: `#[token_goblin::munch(dependencies = [..],no_ide_helper)]`.

## Offline build

Note: `token-goblin-runtime` is a hardcoded dependency of generated crates and might not be downloaded by `cargo fetch` or `cargo vendor`. To build offline, add `token-goblin-runtime` to `[dev-dependencies]` in your `Cargo.toml`.
