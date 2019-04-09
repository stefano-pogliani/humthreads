//! A rust library built on top of [`std::thread`](https://doc.rust-lang.org/stable/std/thread/)
//! to provide additional features and tools to make development easier and to support operators.
//!
//! ## Spawning threads
//! Threads are created with a [`Builder`] similar to [`std::thread::Builder`].
//!
//! ```
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! use humthreads::Builder;
//!
//!# fn main() {
//! let mut thread = Builder::new("os-thread-name")
//!     .spawn(|scope| {
//!         // This code is run in a separate thread.
//!         // The `scope` attribute can be used to interact with advanced APIs.
//!         while !scope.should_shutdown() {
//!             // Do work in the background
//!             sleep(Duration::from_millis(10));
//!         }
//!     })
//!     .expect("failed to spawn thread");
//!
//! // With a humthreads thread we can request the background thread to stop itself.
//! thread.request_shutdown();
//! thread.join().expect("background thread paniced");
//!# }
//! ```
//!
//! ## Inspecting running threads
//! One of the biggest differences from [`std::thread`] is that `humthreads` provide an API to
//! fetch a list of running threads and inspect some of their properties.
//!
//! ```
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! use humthreads::registered_threads;
//! use humthreads::Builder;
//!
//!# fn main() {
//! let mut thread1 = Builder::new("thread1")
//!     .spawn(|scope| {
//!         while !scope.should_shutdown() {
//!             sleep(Duration::from_millis(10));
//!         }
//!     })
//!     .expect("failed to spawn thread1");
//! let mut thread2 = Builder::new("thread2")
//!     .spawn(|scope| {
//!         while !scope.should_shutdown() {
//!             sleep(Duration::from_millis(10));
//!         }
//!     })
//!     .expect("failed to spawn thread2");
//!
//! // Give threads a chance to start or we'll have nothing to inspect.
//! sleep(Duration::from_millis(20));
//!
//! // Fetch a snapshot of running threads and print some information.
//! let threads = registered_threads();
//! for thread in threads {
//!     println!("Thread name: {}", thread.name);
//!     println!("Thread name passed to the OS: {}", thread.short_name);
//!     println!("Current thread activity: {:?}", thread.activity);
//! }
//!
//! // With a humthreads thread we can request the background thread to stop itself.
//! thread1.request_shutdown();
//! thread2.request_shutdown();
//! thread1.join().expect("background thread1 paniced");
//! thread2.join().expect("background thread2 paniced");
//!# }
//! ```
//!
//! ### Reporting threads activity
//! End users of multi-threaded application often wish for a way to understand what the
//! application is doing without having to read the code and study the concurrent actions.
//! This is valuable insight not just in the presence of bugs but also in case of poor performance
//! or simply for users that wish to know more of what is going on under the hood.
//!
//! Threads must themselves report what they are working on.
//! What `humthreads` does is provide helpers for developers to make it easy to report
//! and expose current activity for each thread:
//!
//! ```
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! use humthreads::Builder;
//!
//!# fn main() {
//! let mut thread = Builder::new("os-thread-name")
//!     .spawn(|scope| {
//!         // Set the current activity, overriding the current message.
//!         scope.activity("waiting for work");
//!
//!         // Simulare looping over messages.
//!         for task in 0..10 {
//!             // Change the reported activity for the duration of the scope.
//!             // The message is reverted to what we set above when `_activity` is dropped.
//!             let _activity = scope.scoped_activity(format!("processing task {}", task));
//!             sleep(Duration::from_millis(10));
//!         }
//!     })
//!     .expect("failed to spawn thread");
//! thread.join().expect("background thread paniced");
//!# }
//! ```
//!
//! [`Builder`]: struct.Builder.html
//! [`std::thread`]: https://doc.rust-lang.org/stable/std/thread/index.html
//! [`std::thread::Builder`]: https://doc.rust-lang.org/stable/std/thread/struct.Builder.html
#![doc(html_root_url = "https://docs.rs/humthreads/0.1.1")]

extern crate crossbeam_channel;
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
pub use self::handles::ThreadScopeActivityGuard;
pub use self::registry::registered_threads;
pub use self::status::ThreadStatus;
