use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;

use super::registry::deregister_thread;
use super::registry::register_thread;
use super::status::RegisteredStatus;

/// Handle on a thread returned by [`Builder::spawn`].
///
/// [`Builder::spawn`] ../builder/struct.Builder.html
pub struct Thread<T: Send + 'static> {
    join: JoinHandle<T>,
    shutdown: Arc<AtomicBool>,
}

impl<T: Send + 'static> Thread<T> {
    pub(crate) fn new(join: JoinHandle<T>, shutdown: Arc<AtomicBool>) -> Thread<T> {
        Thread { join, shutdown }
    }

    /// Waits for the associated thread to finish.
    ///
    /// If the child thread panics, [`Err`] is returned with the parameter given
    /// to [`panic`].
    ///
    /// [`Err`]: https://doc.rust-lang.org/std/result/enum.Result.html#variant.Err
    /// [`panic`]: https://doc.rust-lang.org/std/macro.panic.html
    pub fn join(self) -> ::std::thread::Result<T> {
        self.join.join()
    }

    /// Signal the thread is should terminate as soon as possible.
    ///
    /// NOTE: you should take precautions when implementing the thread body to
    /// periodiaclly check if it needs to terminate or not.
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Additional metadata and state for a specific thread.
///
/// You can think of a `ThreadScope` as a handle a thread has on itself.
/// Each `ThreadScope` is an interface to advanced theard API below.
pub struct ThreadScope {
    shutdown: Arc<AtomicBool>,
}

impl ThreadScope {
    pub(crate) fn new(shutdown: Arc<AtomicBool>) -> ThreadScope {
        ThreadScope { shutdown }
    }

    /// Check if the thread was requested to shutdown.
    pub fn should_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
}

/// Thread lifecycle guard.
///
/// Created when a thread is started and [`dropped`] only when it exits
/// either successfully or through [`unwinding`].
///
/// This allows decupling the [`ThreadScope`] for this thread from the thread's lifecycle.
/// By doing so, [`ThreadScope`]s can be dropped if not needed without the introspection
/// features loosing track of a thread.
///
/// [`dropped`] https://doc.rust-lang.org/std/ops/trait.Drop.html
/// [`unwinding`] https://doc.rust-lang.org/nomicon/unwinding.html
/// [`ThreadScope`] struct.ThreadScope.html
pub(crate) struct ThreadGuard {
    id: u64,
}

impl ThreadGuard {
    pub(crate) fn new(id: u64, status: RegisteredStatus) -> ThreadGuard {
        register_thread(id, status);
        ThreadGuard { id }
    }
}

impl Drop for ThreadGuard {
    fn drop(&mut self) {
        deregister_thread(self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::super::Builder;

    #[test]
    fn request_shutdown() {
        let thread = Builder::new("request_shutdown")
            .spawn(|scope| loop {
                ::std::thread::sleep(::std::time::Duration::from_millis(50));
                if scope.should_shutdown() {
                    break;
                }
            })
            .expect("to spawn test thread");
        thread.request_shutdown();
        thread.join().expect("the thread to stop");
    }
}
