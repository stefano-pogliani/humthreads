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
//! let thread = Builder::new("os-thread-name")
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
//! let thread1 = Builder::new("thread1")
//!     .spawn(|scope| {
//!         while !scope.should_shutdown() {
//!             sleep(Duration::from_millis(10));
//!         }
//!     })
//!     .expect("failed to spawn thread1");
//! let thread2 = Builder::new("thread2")
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
//! let thread = Builder::new("os-thread-name")
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
//! ### Waiting for threads with Select
//! ```
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! use crossbeam_channel::Select;
//!
//! use humthreads::Builder;
//!
//!# fn main() {
//! let thread1 = Builder::new("thread1")
//!     .spawn(|_| {
//!         sleep(Duration::from_millis(50));
//!     })
//!     .expect("failed to spawn thread1");
//! let thread2 = Builder::new("thread2")
//!     .spawn(|_| {
//!         sleep(Duration::from_millis(10));
//!     })
//!     .expect("failed to spawn thread2");
//!
//! // Wait for a thread to exit with the Select API.
//! let mut set = Select::new();
//! let idx1 = thread1.select_add(&mut set);
//! let idx2 = thread2.select_add(&mut set);
//! let op = set.select_timeout(Duration::from_millis(20)).expect("selection to find thread2");
//! assert_eq!(idx2, op.index());
//! thread2.select_join(op).expect("thread2 to have exited successfully");
//!# }
//! ```
//!
//! You can also use the [`Select::ready`] API and then use [`Thread::join`] or
//! [`Thread::join_timeout`] to join with the thread.
//!
//! [`Builder`]: struct.Builder.html
//! [`Thread::join`]: struct.Thread.html#method.join
//! [`Thread::join_timeout`]: struct.Thread.html#method.join_timeout
//! [`std::thread`]: https://doc.rust-lang.org/stable/std/thread/index.html
//! [`std::thread::Builder`]: https://doc.rust-lang.org/stable/std/thread/struct.Builder.html
//! [`Select::ready`]: https://docs.rs/crossbeam-channel/*/crossbeam_channel/struct.Select.html
#![doc(html_root_url = "https://docs.rs/humthreads/0.2.1")]

mod builder;
mod error;
mod handles;
mod registry;
mod status;
#[cfg(feature = "with_test_support")]
pub mod test_support;

pub use self::builder::Builder;
pub use self::error::Error;
pub use self::error::ErrorKind;
pub use self::error::Result;
pub use self::handles::MapThread;
pub use self::handles::Thread;
pub use self::handles::ThreadScope;
pub use self::handles::ThreadScopeActivityGuard;
pub use self::registry::registered_threads;
pub use self::status::ThreadStatus;
