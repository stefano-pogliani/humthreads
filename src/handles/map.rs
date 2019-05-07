use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::Select;
use crossbeam_channel::SelectedOperation;

use super::super::ErrorKind;
use super::super::Result;

/// Transform function used by a MapThread<T> to process the result of a thead join.
type MapThreadFn<T> = Box<dyn FnMut() -> Result<T>>;

/// Thread handle that maps the return of a join operation.
pub struct MapThread<T: Send + 'static> {
    // Interior mutability is used to consume the join handle from the join method(s).
    // It is save because the handle is borrowed only within join methods and the
    // Thread type is not Sync (therefore two methods can't be called at once).
    join: RefCell<Option<MapThreadFn<T>>>,
    join_check: Receiver<()>,
    shutdown: Arc<AtomicBool>,
}

impl<T: Send + 'static> MapThread<T> {
    pub(crate) fn new<F>(
        join: F,
        join_check: Receiver<()>,
        shutdown: Arc<AtomicBool>,
    ) -> MapThread<T>
    where
        F: FnMut() -> Result<T> + 'static,
    {
        let join: MapThreadFn<T> = Box::new(join);
        let join = RefCell::new(Some(join));
        MapThread {
            join,
            join_check,
            shutdown,
        }
    }

    /// Same as [`Thread::join`] but applies a transformation to the join result.
    ///
    /// [`Thread::join`]: struct.Thread.html#method.join
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
        let mut handle = match handle {
            None => return Err(ErrorKind::JoinedAlready.into()),
            Some(handle) => handle,
        };
        handle()
    }

    /// Same as [`Thread::join_timeout`] but applies a transformation to the join result.
    ///
    /// [`Thread::join_timeout`]: struct.Thread.html#method.join_timeout
    pub fn join_timeout(&self, timeout: Duration) -> Result<T> {
        match self.join_check.recv_timeout(timeout) {
            Err(RecvTimeoutError::Timeout) => Err(ErrorKind::JoinTimeout.into()),
            _ => self.join(),
        }
    }

    /// Same as [`Thread::request_shutdown`].
    ///
    /// [`Thread::request_shutdown`]: struct.Thread.html#method.request_shutdown
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crossbeam_channel::Select;

    use super::super::super::Builder;

    #[test]
    fn spawn_and_join() {
        let flag: bool = Builder::new("spawn_and_join")
            .spawn(|_| {})
            .expect("failed to spawn thread")
            .map(|_| true)
            .join()
            .expect("failed to join thread");
        assert_eq!(true, flag);
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
            .expect("to spawn test thread")
            .map(|_| true);
        thread.request_shutdown();
        let flag = thread.join().expect("the thread to stop");
        assert_eq!(true, flag);
    }

    #[test]
    fn select_interface() {
        // Create a thread.
        let thread = Builder::new("select_interface")
            .spawn(|_| {
                ::std::thread::sleep(Duration::from_millis(10));
            })
            .expect("to spawn test thread")
            .map(|_| true);

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
            .expect("to spawn test thread")
            .map(|_| true);
        let thread2 = Builder::new("select_multiple_threads_2")
            .spawn(|_| {
                ::std::thread::sleep(Duration::from_millis(10));
            })
            .expect("to spawn test thread")
            .map(|_| true);

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
            .expect("to spawn test thread")
            .map(|_| true);

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
            .expect("to spawn test thread")
            .map(|_| true);

        // Select-join the thread.
        let mut set = Select::new();
        thread.select_add(&mut set);
        let idx = set.ready_timeout(Duration::from_millis(30)).unwrap();
        assert_eq!(0, idx);
        thread.join_timeout(Duration::from_millis(10)).unwrap();
    }
}
