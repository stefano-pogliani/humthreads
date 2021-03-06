use std::any::Any;
use std::fmt;
use std::sync::Mutex;

use failure::Backtrace;
use failure::Context;
use failure::Fail;

/// Error information returned by functions in case of errors.
#[derive(Debug)]
pub struct Error(Context<ErrorKind>);

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.0.get_context()
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.0.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.0.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error(inner)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error(Context::new(kind))
    }
}

/// Exhaustive list of possible errors emitted by this crate.
#[derive(Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "unable to join thread")]
    Join(Mutex<Box<dyn Any + Send + 'static>>),

    #[fail(display = "thread did not stop within the allotted time")]
    JoinTimeout,

    #[fail(display = "thread already joined")]
    JoinedAlready,

    #[fail(display = "unable to spawn new thread")]
    Spawn,
}

/// Short form alias for functions returning `Error`s.
pub type Result<T> = ::std::result::Result<T, Error>;
