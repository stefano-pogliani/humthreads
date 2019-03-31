use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::Receiver;
use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::Sender;

use super::registry::deregister_thread;
use super::registry::register_thread;
use super::status::RegisteredStatus;
use super::ErrorKind;
use super::Result;

/// Handle on a thread returned by [`Builder::spawn`].
///
/// [`Builder::spawn`] ../builder/struct.Builder.html
pub struct Thread<T: Send + 'static> {
    join: Option<JoinHandle<T>>,
    join_check: Receiver<()>,
    shutdown: Arc<AtomicBool>,
}

impl<T: Send + 'static> Thread<T> {
    pub(crate) fn new(
        join: JoinHandle<T>,
        join_check: Receiver<()>,
        shutdown: Arc<AtomicBool>,
    ) -> Thread<T> {
        let join = Some(join);
        Thread {
            join,
            join_check,
            shutdown,
        }
    }

    /// Waits for the associated thread to finish.
    ///
    /// If the child thread panics, [`Err`] is returned with the parameter given
    /// to [`panic`].
    ///
    /// [`Err`]: https://doc.rust-lang.org/std/result/enum.Result.html#variant.Err
    /// [`panic`]: https://doc.rust-lang.org/std/macro.panic.html
    pub fn join(&mut self) -> Result<T> {
        let handle = self.join.take();
        if handle.is_none() {
            return Err(ErrorKind::JoinedAlready.into());
        }
        handle
            .expect("the handle should be Some here")
            .join()
            .map_err(|error| ErrorKind::Join(Mutex::new(error)).into())
    }

    /// Similar to [`Thread::join`] but does not block forever.
    pub fn join_timeout(&mut self, timeout: Duration) -> Result<T> {
        if self.join.is_none() {
            return Err(ErrorKind::JoinedAlready.into());
        }
        match self.join_check.recv_timeout(timeout) {
            Err(RecvTimeoutError::Timeout) => Err(ErrorKind::JoinTimeout.into()),
            _ => self
                .join
                .take()
                .expect("the handle should be Some here")
                .join()
                .map_err(|error| ErrorKind::Join(Mutex::new(error)).into()),
        }
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
    join_check: Sender<()>,
}

impl ThreadGuard {
    pub(crate) fn new(id: u64, join_check: Sender<()>, status: RegisteredStatus) -> ThreadGuard {
        register_thread(id, status);
        ThreadGuard { id, join_check }
    }
}

impl Drop for ThreadGuard {
    fn drop(&mut self) {
        // Try to signal the parent thread we shut down but ignore errors.
        let _ = self.join_check.try_send(());
        deregister_thread(self.id);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::Builder;

    #[test]
    fn request_shutdown() {
        let mut thread = Builder::new("request_shutdown")
            .spawn(|scope| loop {
                ::std::thread::sleep(Duration::from_millis(10));
                if scope.should_shutdown() {
                    break;
                }
            })
            .expect("to spawn test thread");
        thread.request_shutdown();
        thread.join().expect("the thread to stop");
    }

    #[test]
    fn join_timeout() {
        let mut thread = Builder::new("request_shutdown")
            .spawn(|scope| loop {
                ::std::thread::sleep(Duration::from_millis(10));
                if scope.should_shutdown() {
                    break;
                }
            })
            .expect("to spawn test thread");
        thread.request_shutdown();
        thread
            .join_timeout(Duration::from_millis(15))
            .expect("the thread to stop");
    }
}
