/// Internal status traking for registered threads.
pub(crate) struct RegisteredStatus {
    name: String,
    short_name: String,
}

impl RegisteredStatus {
    pub(crate) fn new(name: String, short_name: String) -> RegisteredStatus {
        RegisteredStatus { name, short_name }
    }
}

/// Public view of a point in time status of a thread.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct ThreadStatus {
    /// Full name of the thread.
    pub name: String,

    /// OS name of the thread.
    ///
    /// This is called the short name because OS threads names usually have a limit.
    pub short_name: String,
}

impl From<&RegisteredStatus> for ThreadStatus {
    fn from(status: &RegisteredStatus) -> ThreadStatus {
        ThreadStatus {
            name: status.name.clone(),
            short_name: status.short_name.clone(),
        }
    }
}
