macro_rules! eval {
    ($($expr:tt)*) => {
        {
            #[token_goblin::munch]
            fn eval_inner(_: TokenStream) -> TokenStream {
                use std::str::FromStr;
                let x = $($expr)*;
                quote!{ #x }
            }
            eval_inner!($($expr)*)
        }
    };
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn main() {
    let x = eval!((std::f32::consts::PI.sqrt() * 10.0).round() as usize);
    let y = (std::f32::consts::PI.sqrt() * 10.0).round() as usize;
    assert_eq!(x, y);
    println!("x: {x}");
}
