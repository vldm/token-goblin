macro_rules! trace_recur {
    () => {};

    (($var:expr) $name:ident; $($tail:tt)*) => {{
        writeln!($var, "{} = {:?}", stringify!($name), $name).ok();
        trace_recur!($($tail)*);
    }};

    (($var:expr) $name:ident = $value:expr; $($tail:tt)*) => {{
        let $name = $value;
        writeln!($var, "{} = {:?}", stringify!($name), $name).ok();
        trace_recur!($($tail)*);
    }};
}

#[token_goblin::munch]
fn trace_cycle(input: TokenStream) {
    let mut iter = input.into_iter().peekable();

    macro_rules! fail {
        ($msg:literal) => {
            let msg = $msg;
            output! {
                core::compile_error!(#msg);
            }
            return;
        };
    }
    #[allow(clippy::never_loop)]
    while iter.peek().is_some() {
        let Some(TokenTree::Group(g)) = iter.next() else {
            panic!("Expected group");
        };
        let Some(TokenTree::Ident(ident)) = iter.next() else {
            fail!("Expected ident");
        };
        let mut expr = (&mut iter)
            .take_while(|token| !matches!(token, TokenTree::Punct(p) if p.as_char() == ';'))
            .collect::<Vec<_>>();

        // panic!("foo");
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
        fail!("Expected end of input");
    }
}

fn main() {
    use std::fmt::Write;
    let mut result = String::new();
    trace_recur!(
        (&mut result) x = 10;
        (&mut result) y = 20;
        (&mut result) z = 30;
    );

    let mut result2 = String::new();
    trace_cycle! {
        (&mut result2) x = 10;
        (&mut result2) y = 20;
        (&mut result2) z = 30;
    }
    println!("result: {}", result);
    println!("result2: {}", result2);
}
