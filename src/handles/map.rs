use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use crossbeam_channel::RecvTimeoutError;

use super::super::ErrorKind;
use super::super::Result;

/// Thread handle that maps the return of a join operation.
pub struct MapThread<T: Send + 'static> {
    join: Option<Box<dyn FnMut() -> Result<T>>>,
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
        MapThread {
            join: Some(Box::new(join)),
            join_check,
            shutdown,
        }
    }

    /// Same as [`Thread::join`] but applies a transformation to the join result.
    ///
    /// [`Thread::join`]: struct.Thread.html#method.join
    pub fn join(&mut self) -> Result<T> {
        let handle = self.join.take();
        let mut handle = match handle {
            Some(handle) => handle,
            None => return Err(ErrorKind::JoinedAlready.into()),
        };
        handle()
    }

    /// Same as [`Thread::join_timeout`] but applies a transformation to the join result.
    ///
    /// [`Thread::join_timeout`]: struct.Thread.html#method.join_timeout
    pub fn join_timeout(&mut self, timeout: Duration) -> Result<T> {
        if self.join.is_none() {
            return Err(ErrorKind::JoinedAlready.into());
        }
        match self.join_check.recv_timeout(timeout) {
            Err(RecvTimeoutError::Timeout) => Err(ErrorKind::JoinTimeout.into()),
            _ => self.join.take().expect("the handle should be Some here")(),
        }
    }

    /// Same as [`Thread::request_shutdown`].
    ///
    /// [`Thread::request_shutdown`]: struct.Thread.html#method.request_shutdown
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

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
        let mut thread = Builder::new("request_shutdown")
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
}
