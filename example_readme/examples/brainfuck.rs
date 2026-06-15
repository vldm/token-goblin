#[token_goblin::munch(dependencies = ["reqwest"])]
mod brainfuck {
    use std::collections::VecDeque;

    pub struct ProgramInput {
        program: syn::LitByteStr,
        // instead of stdin, we use predefined input
        input: syn::LitByteStr,
    }

    impl syn::parse::Parse for ProgramInput {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let program = input.parse()?;
            let _: syn::Token![,] = input.parse()?;
            let input = input.parse()?;
            Ok(ProgramInput { program, input })
        }
    }

    pub fn execute(input: ProgramInput) -> TokenStream {
        let program: Vec<u8> = input.program.value();
        let mut input: VecDeque<u8> = input.input.value().into();
        let mut tape = vec![0u8; 30000];
        let mut ptr = 0;
        let mut i = 0;
        let mut output = String::new();
        while i < program.len() {
            match program[i] {
                b'+' => tape[ptr] += 1,
                b'-' => tape[ptr] -= 1,
                b'>' => ptr += 1,
                b'<' => ptr -= 1,
                b'.' => output.push(tape[ptr] as char),
                b',' => tape[ptr] = input.pop_front().unwrap_or_default(),
                b'[' => {
                    if tape[ptr] == 0 {
                        i = program.iter().position(|&x| x == b']').unwrap();
                    }
                }
                b']' => {
                    if tape[ptr] != 0 {
                        i = program.iter().position(|&x| x == b'[').unwrap();
                    }
                }
                _ => {}
            }
            i += 1;
        }
        quote! {
            #output
        }
    }

    // Parse program as URL, download file from URL as program and execute with input.
    pub fn request_and_execute(input: ProgramInput) -> TokenStream {
        let url = String::from_utf8(input.program.value()).unwrap();
        let program = reqwest::blocking::get(url).unwrap().text().unwrap();
        execute(ProgramInput {
            program: syn::LitByteStr::new(&program.as_bytes(), Span::call_site()),
            input: input.input,
        })
    }
}

fn main() {
    let hello_world = "Hello World!\n";
    let result = brainfuck::execute!(b"++++++++++[>+++++++>++++++++++>+++>+<<<<-]>++.>+.+++++++..+++.>++.<<+++++++++++++++.>.+++.------.--------.>+.>.", b"");
    assert_eq!(result, hello_world);
    let result = brainfuck::execute!(b">,[>,]<[.<]", b"\n!dlroW olleH");
    assert_eq!(result, hello_world);

    let result = brainfuck::request_and_execute!(b"https://gist.githubusercontent.com/vldm/f796f0d6235a608c0bed5957d146f8c0/raw/a068d4a8b2764fbc02b909322f31321b1b7eb7fc/reverse.bf", b"\n!dlroW olleH");
    println!("result: {result}");
}
