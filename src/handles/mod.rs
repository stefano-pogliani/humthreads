use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::Receiver;
use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::Select;
use crossbeam_channel::SelectedOperation;
use crossbeam_channel::Sender;

use crate::registry::deregister_thread;
use crate::registry::register_thread;
use crate::status::RegisteredStatus;
use crate::ErrorKind;
use crate::Result;

mod map;

pub use self::map::MapThread;

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
    // Interior mutability is used to consume the join handle from the join method(s).
    // It is save because the handle is borrowed only within join methods and the
    // Thread type is not Sync (therefore two methods can't be called at once).
    join: RefCell<Option<JoinHandle<T>>>,
    join_check: Receiver<()>,
    shutdown: Arc<AtomicBool>,
}

impl<T: Send + 'static> Thread<T> {
    pub(crate) fn new(
        join: JoinHandle<T>,
        join_check: Receiver<()>,
        shutdown: Arc<AtomicBool>,
    ) -> Thread<T> {
        let join = RefCell::new(Some(join));
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
    pub fn join(&self) -> Result<T> {
        // It should always be possible to borrow the handle but in case users manage
        // to create uses that lead to multiple concurrent invocations of this method
        // return an error instead of panicing.
        // One of the calls will be able to proceed and actually join the thread.
        let handle = self
            .join
            .try_borrow_mut()
            .map_err(|_| ErrorKind::JoinedAlready)?
            .take();
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
    pub fn join_timeout(&self, timeout: Duration) -> Result<T> {
        match self.join_check.recv_timeout(timeout) {
            Err(RecvTimeoutError::Timeout) => Err(ErrorKind::JoinTimeout.into()),
            _ => self.join(),
        }
    }

    /// Apply a transformation function when joining to the thread.
    // NOTE: the callback cannot be an `FnOnce` because it is applied
    // only when the thread is joined and `FnOnce` can't be `Box`ed.
    pub fn map<U, F>(self, mut f: F) -> MapThread<U>
    where
        U: Send + 'static,
        F: FnMut(T) -> U + 'static,
    {
        let mut join = self.join.into_inner();
        let join = move || {
            let join = match join.take() {
                Some(join) => join,
                None => return Err(ErrorKind::JoinedAlready.into()),
            };
            // Result::map needs an `FnOnce` so we use an explicit clousure wrapping an `FnMut`.
            // Clippy is not too happy about it though.
            #[allow(clippy::redundant_closure)]
            join.join()
                .map_err(|error| ErrorKind::Join(Mutex::new(error)).into())
                .map(|r| f(r))
        };
        MapThread::new(join, self.join_check, self.shutdown)
    }

    /// Signal the thread is should terminate as soon as possible.
    ///
    /// NOTE: you should take precautions when implementing the thread body to
    /// periodiaclly check if it needs to terminate or not.
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Add the thread to a [`Select`] set.
    ///
    /// [`Select`]: crossbeam_channel/struct.Select.html
    pub fn select_add<'a>(&'a self, select: &mut Select<'a>) -> usize {
        select.recv(&self.join_check)
    }

    /// Completes a join operation that was started by the [`Select`] interface.
    ///
    /// This method should be used if one of the `select` operations are used.
    /// If the `ready` familiy of methods is used, use one of the other join methods.
    pub fn select_join(&self, operation: SelectedOperation) -> Result<T> {
        // Complete the receive operation to avoid panics.
        // Regardless of the operation result, this indicates the thread exit.
        let _ = operation.recv(&self.join_check);
        self.join()
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

    use crossbeam_channel::Select;

    use super::super::registered_threads;
    use super::super::Builder;

    #[test]
    fn activity() {
        // Create a thread that reports activity.
        let thread = Builder::new("activity")
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
        let thread = Builder::new("idle")
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
        let thread = Builder::new("request_shutdown")
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
        let thread = Builder::new("request_shutdown")
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
        let thread = Builder::new("scoped_activity")
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

    #[test]
    fn select_interface() {
        // Create a thread.
        let thread = Builder::new("select_interface")
            .spawn(|_| {
                ::std::thread::sleep(Duration::from_millis(10));
            })
            .expect("to spawn test thread");

        // Select-join the thread.
        let mut set = Select::new();
        let idx = thread.select_add(&mut set);
        let op = set.select_timeout(Duration::from_millis(30)).unwrap();
        thread.select_join(op).unwrap();
        assert_eq!(0, idx);
    }

    #[test]
    fn select_multiple_threads() {
        // Create a thread.
        let thread1 = Builder::new("select_multiple_threads_1")
            .spawn(|_| {
                ::std::thread::sleep(Duration::from_millis(50));
            })
            .expect("to spawn test thread");
        let thread2 = Builder::new("select_multiple_threads_2")
            .spawn(|_| {
                ::std::thread::sleep(Duration::from_millis(10));
            })
            .expect("to spawn test thread");

        // Select-join the thread.
        let mut set = Select::new();
        thread1.select_add(&mut set);
        thread2.select_add(&mut set);
        let op = set.select_timeout(Duration::from_millis(30)).unwrap();
        let idx = op.index();
        thread2.select_join(op).unwrap();
        assert_eq!(1, idx);
    }

    #[test]
    fn select_panic() {
        // Create a thread.
        let thread = Builder::new("select_panic")
            .spawn(|_| {
                ::std::thread::sleep(Duration::from_millis(10));
                panic!("this panic is expected");
            })
            .expect("to spawn test thread");

        // Select-join the thread.
        let mut set = Select::new();
        thread.select_add(&mut set);
        let op = set.select_timeout(Duration::from_millis(30)).unwrap();
        let idx = op.index();
        let result = thread.select_join(op);
        assert_eq!(0, idx);
        assert_eq!(true, result.is_err());
    }

    #[test]
    fn select_ready_interface() {
        // Create a thread.
        let thread = Builder::new("select_panic")
            .spawn(|_| {
                ::std::thread::sleep(Duration::from_millis(10));
            })
            .expect("to spawn test thread");

        // Select-join the thread.
        let mut set = Select::new();
        thread.select_add(&mut set);
        let idx = set.ready_timeout(Duration::from_millis(30)).unwrap();
        assert_eq!(0, idx);
        thread.join_timeout(Duration::from_millis(10)).unwrap();
    }
}
