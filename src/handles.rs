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

/// An RAII implementation of a "scoped activity" of a thread.
///
/// When this structure is dropped (falls out of scope), the thread reported activity
/// will be reverted back to what it was when the guard was created.
pub struct ThreadScopeActivityGuard {
    activity: Arc<Mutex<Option<String>>>,
    current: Option<String>,
}

impl Drop for ThreadScopeActivityGuard {
    fn drop(&mut self) {
        let mut guard = self
            .activity
            .lock()
            .expect("ThreadScopeActivityGuard::activity lock poisoned");
        *guard = self.current.take();
    }
}

/// Handle on a thread returned by [`Builder::spawn`].
///
/// [`Builder::spawn`]: struct.Builder.html
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
    ///
    /// [`Thread::join`]: struct.Thread.html#method.join
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
/// You can think of a [`ThreadScope`] as a handle a thread has on itself.
/// Each `ThreadScope` is an interface to advanced theard API below.
///
/// [`ThreadScope`]: struct.ThreadScope.html
pub struct ThreadScope {
    activity: Arc<Mutex<Option<String>>>,
    shutdown: Arc<AtomicBool>,
}

impl ThreadScope {
    pub(crate) fn new(
        activity: Arc<Mutex<Option<String>>>,
        shutdown: Arc<AtomicBool>,
    ) -> ThreadScope {
        ThreadScope { activity, shutdown }
    }

    /// Report the current thread activity.
    ///
    /// This information will become accessible from the introspection API.
    /// The main use case is to aid application end users monitor, debug, and understand
    /// complex software they use and operate but not implement.
    pub fn activity<S: Into<String>>(&self, activity: S) {
        let activity = activity.into();
        let mut guard = self
            .activity
            .lock()
            .expect("ThreadScope::activity lock poisoned");
        *guard = Some(activity);
    }

    /// Clear any previously reported activity.
    pub fn idle(&self) {
        let mut guard = self
            .activity
            .lock()
            .expect("ThreadScope::activity lock poisoned");
        *guard = None;
    }

    /// Report the given activity for the duration of a scope.
    ///
    /// The scope is considered over once the returned [`ThreadScopeActivityGuard`] is dropped.
    ///
    /// [`ThreadScopeActivityGuard`]: struct.ThreadScopeActivityGuard.html
    pub fn scoped_activity<S: Into<String>>(&self, activity: S) -> ThreadScopeActivityGuard {
        let activity = activity.into();
        let mut guard = self
            .activity
            .lock()
            .expect("ThreadScope::activity lock poisoned");
        let current: Option<String> = guard.clone();
        *guard = Some(activity);
        drop(guard);
        let activity = Arc::clone(&self.activity);
        ThreadScopeActivityGuard { activity, current }
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
/// [`dropped`]: https://doc.rust-lang.org/std/ops/trait.Drop.html
/// [`unwinding`]: https://doc.rust-lang.org/nomicon/unwinding.html
/// [`ThreadScope`]: struct.ThreadScope.html
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

    use super::super::registered_threads;
    use super::super::Builder;

    #[test]
    fn activity() {
        // Create a thread that reports activity.
        let mut thread = Builder::new("activity")
            .spawn(|scope| {
                scope.activity("testing activity API");
                loop {
                    ::std::thread::sleep(Duration::from_millis(10));
                    if scope.should_shutdown() {
                        break;
                    }
                }
            })
            .expect("to spawn test thread");

        // Give it a chance to register and collect list.
        ::std::thread::sleep(::std::time::Duration::from_millis(10));
        let threads = registered_threads();

        // Stop background thread now that we do not need it.
        thread.request_shutdown();
        thread.join().expect("the thread to stop");

        // Assert test results.
        let thread = threads
            .into_iter()
            .find(|t| t.name == "activity")
            .expect("test thread not found");
        assert_eq!(Some("testing activity API".into()), thread.activity);
    }

    #[test]
    fn idle() {
        // Create a thread that reports activity.
        let mut thread = Builder::new("idle")
            .spawn(|scope| {
                scope.activity("testing activity API");
                scope.idle();
                loop {
                    ::std::thread::sleep(Duration::from_millis(10));
                    if scope.should_shutdown() {
                        break;
                    }
                }
            })
            .expect("to spawn test thread");

        // Give it a chance to register and collect list.
        ::std::thread::sleep(::std::time::Duration::from_millis(10));
        let threads = registered_threads();

        // Stop background thread now that we do not need it.
        thread.request_shutdown();
        thread.join().expect("the thread to stop");

        // Assert test results.
        let thread = threads
            .into_iter()
            .find(|t| t.name == "idle")
            .expect("test thread not found");
        assert_eq!(None, thread.activity);
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
    fn scoped_activity() {
        let (notifier, notifiction) = ::crossbeam_channel::bounded(0);
        // A background thread manipulates scoped activity ...
        let mut thread = Builder::new("scoped_activity")
            .spawn(move |scope| {
                notifiction
                    .recv_timeout(Duration::from_millis(50))
                    .expect("proceed to scope1");
                {
                    let scope1 = scope.scoped_activity("scope1");
                    notifiction
                        .recv_timeout(Duration::from_millis(50))
                        .expect("proceed to scope2");
                    let scope2 = scope.scoped_activity("scope2");
                    notifiction
                        .recv_timeout(Duration::from_millis(50))
                        .expect("proceed out of scope2");
                    drop(scope2);
                    notifiction
                        .recv_timeout(Duration::from_millis(50))
                        .expect("proceed out of scope1");
                    drop(scope1);
                    notifiction
                        .recv_timeout(Duration::from_millis(50))
                        .expect("proceed to scope3");
                }
                let scope3 = scope.scoped_activity("scope3");
                notifiction
                    .recv_timeout(Duration::from_millis(50))
                    .expect("proceed out of scope3");
                drop(scope3);
                notifiction
                    .recv_timeout(Duration::from_millis(50))
                    .expect("proceed to thread exit");
            })
            .expect("to spawn test thread");

        // ... while the test body controls its progess.
        ::std::thread::sleep(Duration::from_millis(10));
        let start = registered_threads();
        notifier.send(()).expect("proceed to scope1");

        ::std::thread::sleep(Duration::from_millis(10));
        let scope1_in = registered_threads();
        notifier.send(()).expect("proceed to scope2");

        ::std::thread::sleep(Duration::from_millis(10));
        let scope2_in = registered_threads();
        notifier.send(()).expect("proceed out of scope2");

        ::std::thread::sleep(Duration::from_millis(10));
        let scope2_out = registered_threads();
        notifier.send(()).expect("proceed out of scope1");

        ::std::thread::sleep(Duration::from_millis(10));
        let scope1_out = registered_threads();
        notifier.send(()).expect("proceed to scope3");

        ::std::thread::sleep(Duration::from_millis(10));
        let scope3_in = registered_threads();
        notifier.send(()).expect("proceed out of scope3");

        ::std::thread::sleep(Duration::from_millis(10));
        let scope3_out = registered_threads();
        notifier.send(()).expect("proceed to thread exit");

        // Shut thread down and move on to assertions.
        thread.request_shutdown();
        thread.join().expect("the thread to stop");

        let status = start
            .into_iter()
            .find(|t| t.name == "scoped_activity")
            .expect("test thread not found");
        assert_eq!(None, status.activity);
        let status = scope1_in
            .into_iter()
            .find(|t| t.name == "scoped_activity")
            .expect("test thread not found");
        assert_eq!(Some("scope1".into()), status.activity);
        let status = scope2_in
            .into_iter()
            .find(|t| t.name == "scoped_activity")
            .expect("test thread not found");
        assert_eq!(Some("scope2".into()), status.activity);
        let status = scope2_out
            .into_iter()
            .find(|t| t.name == "scoped_activity")
            .expect("test thread not found");
        assert_eq!(Some("scope1".into()), status.activity);
        let status = scope1_out
            .into_iter()
            .find(|t| t.name == "scoped_activity")
            .expect("test thread not found");
        assert_eq!(None, status.activity);
        let status = scope3_in
            .into_iter()
            .find(|t| t.name == "scoped_activity")
            .expect("test thread not found");
        assert_eq!(Some("scope3".into()), status.activity);
        let status = scope3_out
            .into_iter()
            .find(|t| t.name == "scoped_activity")
            .expect("test thread not found");
        assert_eq!(None, status.activity);
    }
}
