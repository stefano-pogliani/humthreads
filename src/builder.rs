use std::thread::Builder as StdBuilder;

use failure::ResultExt;

use super::handles::ThreadGuard;
use super::ErrorKind;
use super::Result;
use super::Thread;
use super::ThreadScope;

/// Thread factory to configure the properties of a new thread.
///
/// These threads wrap [`std::thread`]s to provide a few additional features:
///
///   * Shutdown requests: thread handles can signal their thread it should stop
///     (be warned the thread function may ignore this).
///   * Threads introspection: APIs provide information about running threads
///     (threads can even report what they are doing at the time).
///
/// [`std::thread`]: https://doc.rust-lang.org/std/thread/index.html
pub struct Builder {
    full_name: String,
    std: StdBuilder,
}

impl Builder {
    pub fn new<S: Into<String>>(name: S) -> Builder {
        let name = name.into();
        let std = StdBuilder::new().name(name.clone());
        Builder {
            full_name: name,
            std,
        }
    }

    /// Set the full name used for introspection.
    ///
    /// This is stored as a rust [`String`] and it is not passed to the OS
    /// so it is NOT subject to the same limit that [std threads] have on names.
    ///
    /// [std threads]: https://doc.rust-lang.org/std/thread/index.html#naming-threads
    pub fn full_name<S: Into<String>>(mut self, name: S) -> Builder {
        self.full_name = name.into();
        self
    }

    /// Spawns a new thread by taking ownership of the Builder.
    ///
    /// On success a [`Thread`] handle is returned.
    ///
    /// [`Thread`] ../handles/struct.Thread.html
    pub fn spawn<F, T>(self, f: F) -> Result<Thread<T>>
    where
        F: FnOnce(ThreadScope) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        let join = self
            .std
            .spawn(|| {
                // Keep a ThreadGuard alive as long as the thread is.
                let _guard = ThreadGuard::new();
                let scope = ThreadScope::new();
                f(scope)
            })
            .with_context(|_| ErrorKind::Spawn)?;
        Ok(Thread::new(join))
    }
}

#[cfg(test)]
mod tests {
    use super::Builder;

    #[test]
    fn spawn_and_join() {
        Builder::new("test")
            .spawn(|_| {})
            .expect("failed to spawn thread")
            .join()
            .expect("failed to join thread");
    }
}
