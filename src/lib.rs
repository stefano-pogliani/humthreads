extern crate failure;
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod builder;
mod error;
mod handles;
mod registry;
mod status;

pub use self::builder::Builder;
pub use self::error::Error;
pub use self::error::ErrorKind;
pub use self::error::Result;
pub use self::handles::Thread;
pub use self::handles::ThreadScope;
pub use self::registry::registered_threads;
pub use self::status::ThreadStatus;
