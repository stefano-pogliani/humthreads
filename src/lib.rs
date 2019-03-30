extern crate failure;
//extern crate serde;
//#[macro_use]
//extern crate serde_derive;

mod builder;
mod error;
mod handles;

pub use self::builder::Builder;
pub use self::error::Error;
pub use self::error::ErrorKind;
pub use self::error::Result;
pub use self::handles::Thread;
pub use self::handles::ThreadScope;
