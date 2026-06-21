// Force throwing error only with span.
#[allow(unused)]
macro_rules! bail {
    ($($err:tt)*) => {
        return Err(error!($($err)*))
    };

}

macro_rules! error {
    ($span:expr => $($message:tt)*) => {
        syn::Error::new( $span, format!($($message)*))
    };
    ($($message:tt)*) => {
        syn::Error::new(proc_macro2::Span::call_site(), format!($($message)*))
    };
}

pub trait MapCompileError {
    fn map_compile_error(self) -> proc_macro2::TokenStream;
}

impl MapCompileError for super::Result<proc_macro2::TokenStream> {
    fn map_compile_error(self) -> proc_macro2::TokenStream {
        self.unwrap_or_else(|e| e.to_compile_error())
    }
}

macro_rules! debug {
    (level: $level:expr, $($message:tt)*) => {
        if crate::path::env_print_level($level) && crate::DEBUG {
            let fmt = format_args!($($message)*);
            eprintln!("[{module}] {fmt}", module = module_path!());
        }
    };
    ($($message:tt)*) => {
        debug!(level: 0, $($message)*);
    };
}
