use std::fmt::Display;

use serde::{
    Deserialize, Serialize
};

use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum MessageKind {
    Response,

    /// placeholder for any pending streams without any content (AwaitingStream)
    ResponsePlaceholder,
    User,
    Thinking,
    Error,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Message {
    pub uuid: Uuid,
    pub index: usize,
    pub kind: MessageKind,
    pub content: String,
}

impl Message {
    pub fn new(index: usize, kind: MessageKind, content: impl Display) -> Self {
        Self {
            uuid: Uuid::now_v7(),
            index,
            kind,
            content: content.to_string()
        }
    }
}
