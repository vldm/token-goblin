use std::cell::RefCell;

use proc_macro::Span;

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub span: Span,
    pub hints: Vec<Hint>,
}

impl Error {
    pub fn new(message: String, span: Span) -> Self {
        Self {
            message,
            span,
            hints: Vec::new(),
        }
    }
    pub fn with_hint(mut self, hint: Hint) -> Self {
        self.hints.push(hint);
        self
    }
}
// Note to a user that should be emitted in case of hard error.

#[derive(Debug)]
pub struct Hint {
    // Hint type: help, note, warning?
    // currently not supported by stable, so emitted as additional `error` in case of other errors.
    pub message: String,
    pub span: Span,
}

// Force throwing error only with span.
macro_rules! bail {
    ($span:expr => $($message:tt)*) => {
        return Err(crate::errors::Error::new(format!($($message)*), $span))
    };
}

macro_rules! error {
    ($span:expr => $($message:tt)*) => {
        crate::errors::Error::new(format!($($message)*), $span)
    };
}

macro_rules! hint {
    ($span:expr => $($message:tt)*) => {
        let message = format!($($message)*);
        return crate::errors::Hint { message, span: $span };
    };
    ($($message:tt)*) => {
       hint!(proc_macro::Span::call_site() => $($message)*);
    }
}

trait ResultExt<T, E> {
    /// Add hint to the error.
    fn with_hint(self, e: E) -> super::Result<T>;
}

impl<T, E> ResultExt<T, E> for super::Result<T>
where
    E: FnOnce() -> super::errors::Hint,
{
    fn with_hint(self, e: E) -> super::Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.with_hint(e())),
        }
    }
}

impl<AnyError> From<AnyError> for Error
where
    AnyError: std::error::Error + Send + Sync + 'static,
{
    fn from(error: AnyError) -> Self {
        Self::new(error.to_string(), Span::call_site())
    }
}

macro_rules! debug {
    ($($message:tt)*) => {
        if crate::DEBUG {
            eprintln!($($message)*);
        }
    };
}
