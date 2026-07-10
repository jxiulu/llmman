use std::{
    time::{
        Instant, Duration
    },
    fmt::Display
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationKind {
    Info,
    Warning,
    Error,
}

#[derive(Clone)]
pub struct Notification {
    pub kind: NotificationKind,
    pub message: String,
    pub expiry: Instant,
}

impl Notification {
    pub fn new(kind: NotificationKind, message: impl Display, duration: Duration) -> Self {
        Self {
            kind,
            message: message.to_string(),
            expiry: Instant::now() + duration
        }
    }
}
