//! Error types for `genpdfi`.

use std::error;
use std::fmt;
use std::io;

/// Helper trait for creating [`Error`][] instances.
///
/// This trait is inspired by [`anyhow::Context`][].
///
/// # Examples
///
/// ```
/// use genpdfi_extended::error::{Context as _, Error};
/// use std::io;
/// let res: Result<(), io::Error> = Err(io::Error::new(io::ErrorKind::Other, "boom"));
/// let mapped = res.context("wrapped");
/// assert!(mapped.is_err());
/// ```
///
/// [`Error`]: struct.Error.html
/// [`anyhow::Context`]: https://docs.rs/anyhow/latest/anyhow/trait.Context.html
pub trait Context<T> {
    /// Maps the error to an [`Error`][] instance with the given message.
    ///
    /// [`Error`]: struct.Error.html
    fn context(self, msg: impl Into<String>) -> Result<T, Error>;

    /// Maps the error to an [`Error`][] instance message produced by the given callback.
    ///
    /// [`Error`]: struct.Error.html
    fn with_context<F, S>(self, cb: F) -> Result<T, Error>
    where
        F: Fn() -> S,
        S: Into<String>;
}

impl<T, E: Into<ErrorKind>> Context<T> for Result<T, E> {
    fn context(self, msg: impl Into<String>) -> Result<T, Error> {
        self.map_err(|err| Error::new(msg, err))
    }

    fn with_context<F, S>(self, cb: F) -> Result<T, Error>
    where
        F: Fn() -> S,
        S: Into<String>,
    {
        self.map_err(move |err| Error::new(cb(), err))
    }
}

/// An error that occured in a `genpdfi` function.
///
/// The error consists of an error message (provided by the `Display` implementation) and an error
/// kind, see [`kind`](#method.kind).
#[derive(Debug)]
pub struct Error {
    msg: String,
    kind: ErrorKind,
}

impl Error {
    /// Creates a new error.
    ///
    /// # Examples
    ///
    /// ```
    /// use genpdfi_extended::error::{Error, ErrorKind};
    /// let e = Error::new("oops", ErrorKind::Internal);
    /// assert_eq!(format!("{}", e), "oops");
    /// match e.kind() { ErrorKind::Internal => {}, k => panic!("unexpected: {:?}", k) }
    /// ```
    pub fn new(msg: impl Into<String>, kind: impl Into<ErrorKind>) -> Error {
        Error {
            msg: msg.into(),
            kind: kind.into(),
        }
    }

    /// Returns the error kind for this error.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Internal => None,
            ErrorKind::InvalidData => None,
            ErrorKind::InvalidFont => None,
            ErrorKind::PageSizeExceeded => None,
            ErrorKind::UnsupportedEncoding => None,
            ErrorKind::IoError(err) => Some(err),
            ErrorKind::PdfError(_err) => None,
            #[cfg(feature = "images")]
            ErrorKind::ImageError(err) => Some(err),
            _ => None,
        }
    }
}

/// The kind of an [`Error`](struct.Error.html).
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An internal error.
    Internal,
    /// An error caused by invalid data.
    InvalidData,
    /// An error caused by an invalid font.
    InvalidFont,
    /// An element exceeds the page size and could not be printed.
    PageSizeExceeded,
    /// A string with unsupported characters was used with a built-in font.
    UnsupportedEncoding,
    /// An IO error.
    IoError(io::Error),
    /// An error caused by invalid data in `printpdf`.
    PdfError(String),
    /// An error caused by `image`.
    ///
    /// *Only available if the `images` feature is enabled.*
    #[cfg(feature = "images")]
    ImageError(image::ImageError),
}

impl From<io::Error> for ErrorKind {
    fn from(error: io::Error) -> ErrorKind {
        ErrorKind::IoError(error)
    }
}

impl From<String> for ErrorKind {
    fn from(error: String) -> ErrorKind {
        ErrorKind::PdfError(error)
    }
}

impl From<&str> for ErrorKind {
    fn from(error: &str) -> ErrorKind {
        ErrorKind::PdfError(error.to_string())
    }
}

#[cfg(feature = "images")]
impl From<image::ImageError> for ErrorKind {
    fn from(error: image::ImageError) -> ErrorKind {
        ErrorKind::ImageError(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;
    use std::io;

    #[test]
    fn test_error_display_and_source() {
        let io_err = io::Error::new(io::ErrorKind::Other, "io fail");
        let e: Error = Error::new("oops", ErrorKind::IoError(io_err));
        assert_eq!(format!("{}", e), "oops");
        match e.source() {
            Some(src) => {
                let src_str = format!("{}", src);
                assert!(src_str.contains("io fail"));
            }
            None => panic!("expected source for IoError"),
        }
    }

    #[test]
    fn test_context_maps_errors() {
        let res: Result<(), io::Error> = Err(io::Error::new(io::ErrorKind::Other, "boom"));
        let mapped = res.context("wrapped");
        match mapped {
            Err(err) => {
                assert_eq!(format!("{}", err), "wrapped");
                match err.kind() {
                    ErrorKind::IoError(ioe) => assert_eq!(ioe.kind(), io::ErrorKind::Other),
                    k => panic!("unexpected kind: {:?}", k),
                }
            }
            Ok(_) => panic!("expected Err"),
        }
    }
}
