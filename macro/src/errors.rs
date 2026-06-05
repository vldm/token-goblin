pub type AnyError = Box<dyn std::error::Error + Send + Sync + 'static>;

// Force throwing error only with span.
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
    ($($message:tt)*) => {
        if crate::DEBUG {
            eprintln!($($message)*);
        }
    };
}
