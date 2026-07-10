use std::fmt::Display;

use serde::{
    Deserialize, Serialize
};
use uuid::Uuid;

use super::{
    Message, MessageKind
};

// for saving purposes
#[derive(Deserialize, Serialize)]
pub struct Chat {
    pub uuid: Uuid,
    pub name: String,
    pub messages: Vec<Message>
}

impl Chat {
    pub fn new(name: &str) -> Self {
        Self {
            uuid: Uuid::now_v7(),
            messages: Vec::new(),
            name: name.to_owned(),
        }
    }

    pub fn push_message(&mut self, kind: MessageKind, content: impl Display) {
        self.messages.push(Message::new(
            self.messages.len(),
            kind,
            content
        ));
    }

    pub fn num_messages(&self) -> usize {
        self.messages.len()
    }
}
