macro_rules! eval {
    ($($expr:tt)*) => {
        {
            #[token_goblin::munch(lazy)]
            fn eval_inner(_: TokenStream) -> TokenStream {
                use std::str::FromStr;
                let x = $($expr)*;
                TokenStream::from_str(&format!("{}", x)).unwrap()
            }
            eval_inner!($($expr)*)
        }
    };
}

fn main() {
    let x = eval!(200);
    let y = eval!(100) + x;
    println!("y: {y}");
}
