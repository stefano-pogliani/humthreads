use std::thread::JoinHandle;

/// Handle on a thread returned by [`Builder::spawn`].
///
/// [`Builder::spawn`] ../builder/struct.Builder.html
pub struct Thread<T: Send + 'static> {
    join: JoinHandle<T>,
}

impl<T: Send + 'static> Thread<T> {
    pub(crate) fn new(join: JoinHandle<T>) -> Thread<T> {
        Thread { join }
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
}

/// Additional metadata and state for a specific thread.
///
/// You can think of a `ThreadScope` as a handle a thread has on itself.
/// Each `ThreadScope` is an interface to advanced theard API below.
pub struct ThreadScope {
    // TODO
}

impl ThreadScope {
    pub(crate) fn new() -> ThreadScope {
        ThreadScope {}
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
pub struct ThreadGuard {
    // TODO
}

impl ThreadGuard {
    pub(crate) fn new() -> ThreadGuard {
        ThreadGuard {}
    }
}
