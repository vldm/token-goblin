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

#[token_goblin::munch]
mod trace_syn {
    #[derive(syn_derive::Parse)]
    struct TraceStmt {
        writer: syn::Expr,
        ident: syn::Ident,
        value: TraceValue,
    }

    #[derive(syn_derive::Parse)]
    enum TraceValue {
        #[parse(peek = syn::Token![=])]
        Some {
            token_eq: syn::Token![=],
            expr: syn::Expr,
        },
        None,
    }
    pub struct TraceInput(Vec<TraceStmt>);
    impl syn::parse::Parse for TraceInput {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            Ok(TraceInput(
                syn::punctuated::Punctuated::<TraceStmt, syn::Token![;]>::parse_terminated(input)?
                    .into_iter()
                    .collect(),
            ))
        }
    }

    pub fn trace(input: TraceInput) -> TokenStream {
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

    let mut result3 = String::new();
    trace_syn::trace! {
        (&mut result3) x = 10;
        (&mut result3) y = 20;
        (&mut result3) z = 30;
    }
    println!("result: {result}");
    println!("result2: {result2}");
    println!("result3: {result3}");
}
