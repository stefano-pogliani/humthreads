use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Mutex;

use super::status::RegisteredStatus;
use super::status::ThreadStatus;

lazy_static::lazy_static! {
    static ref THREADS_REGISTRY: Mutex<HashMap<u64, RegisteredStatus>> = {
        Mutex::new(HashMap::new())
    };
}

/// Return the current thread id as an unsigned integer.
pub(crate) fn current_thread_id() -> u64 {
    let id = ::std::thread::current().id();
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

/// Removes thread state information for the specified thread.
pub(crate) fn deregister_thread(id: u64) {
    THREADS_REGISTRY
        .lock()
        .expect("global THREADS_REGISTRY lock poisoned")
        .remove(&id);
}

/// Insert thread state information for a new thread.
pub(crate) fn register_thread(id: u64, status: RegisteredStatus) {
    THREADS_REGISTRY
        .lock()
        .expect("global THREADS_REGISTRY lock poisoned")
        .insert(id, status);
}

/// Return a snapshot of the current status of threads.
pub fn registered_threads() -> Vec<ThreadStatus> {
    THREADS_REGISTRY
        .lock()
        .expect("global THREADS_REGISTRY lock poisoned")
        .iter()
        .map(|(_, status)| status.into())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::Builder;
    use super::registered_threads;

    #[test]
    fn thread_registration_lifecycle() {
        // Create a thread.
        let thread = Builder::new("thread_registration_lifecycle")
            .full_name("thread registration lifecycle long")
            .spawn(|scope| loop {
                ::std::thread::sleep(::std::time::Duration::from_millis(50));
                if scope.should_shutdown() {
                    break;
                }
            })
            .expect("to spawn test thread");

        // Give it a chance to register and collect list.
        ::std::thread::sleep(::std::time::Duration::from_millis(50));
        let running_threads = registered_threads();

        // Stop background thread now that we do not need it.
        thread.request_shutdown();
        thread.join().expect("the thread to stop");
        let stopped_threads = registered_threads();

        // Assert test results.
        let thread = running_threads
            .into_iter()
            .find(|t| t.short_name == "thread_registration_lifecycle");
        assert_eq!(true, thread.is_some());
        assert_eq!("thread registration lifecycle long", thread.unwrap().name);
        let thread = stopped_threads
            .into_iter()
            .find(|t| t.short_name == "thread_registration_lifecycle");
        assert_eq!(false, thread.is_some());
    }
}
