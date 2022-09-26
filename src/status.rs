use std::sync::Arc;
use std::sync::Mutex;

use serde::Deserialize;
use serde::Serialize;

/// Internal status tracking for registered threads.
pub(crate) struct RegisteredStatus {
    activity: Arc<Mutex<Option<String>>>,
    name: String,
    short_name: String,
}

impl RegisteredStatus {
    /// Provide mutable access to the thread's activity status attribute.
    pub(crate) fn activity(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.activity)
    }

    pub(crate) fn new(name: String, short_name: String) -> RegisteredStatus {
        let activity = Arc::new(Mutex::new(None));
        RegisteredStatus {
            activity,
            name,
            short_name,
        }
    }
}

/// Public view of a point in time status of a thread.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct ThreadStatus {
    /// Description of the activity currently in progress by the thread.
    ///
    /// NOTE: threads are responsible for reporting their own activity.
    pub activity: Option<String>,

    /// Full name of the thread.
    pub name: String,

    /// OS name of the thread.
    ///
    /// This is called the short name because OS threads names usually have a limit.
    pub short_name: String,
}

impl From<&RegisteredStatus> for ThreadStatus {
    fn from(status: &RegisteredStatus) -> ThreadStatus {
        let activity = status
            .activity
            .lock()
            .expect("RegisteredStatus::activity lock poisoned")
            .clone();
        ThreadStatus {
            activity,
            name: status.name.clone(),
            short_name: status.short_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RegisteredStatus;
    use super::ThreadStatus;

    #[test]
    fn from_register() {
        let register = RegisteredStatus::new("long name".into(), "name".into());
        let status = ThreadStatus::from(&register);
        assert_eq!(status.activity, None);
        assert_eq!(status.name, "long name");
        assert_eq!(status.short_name, "name");
    }

    #[test]
    fn report_activity() {
        let register = RegisteredStatus::new("long name".into(), "name".into());
        *register.activity.lock().unwrap() = Some("test".into());
        let status = ThreadStatus::from(&register);
        assert_eq!(status.activity, Some("test".into()));
    }
}
