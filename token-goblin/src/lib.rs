//! Main `token-goblin` crate. Re-exports will live here later.

use std::num::Saturating;

pub use token_goblin_macro::munch;

// #[munch(split_cache = true)]
// fn foo(v: TokenStream) -> TokenStream {
//     v
// }

// #[munch(split_cache = true)]
// macro! bar() {}

// const X: usize = foo!(12);
// const _ASSERT: () = assert!(X == 12);

type MyType<T = u32> = Saturating<T>;
const my_type: fn(u32) -> MyType = Saturating;

fn foo() {
    let x = my_type(10);
    println!("x: {}", x);
}
