use std::{default, fmt::Display};

use smart_default::SmartDefault;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Response,

    /// placeholder for any pending streams without any content (AwaitingStream)
    ResponsePlaceholder,
    User,

    Thinking,
    Error,
}

#[derive(Clone)]
pub struct Message {
    pub index: usize,
    pub kind: MessageKind,
    pub content: String,
}

#[derive(PartialEq, Eq, Clone)]
pub enum StreamingState {
    Streaming,
    AwaitingStream,
    Idle,
}

/// What box is currently being focused on right now
#[derive(PartialEq, Eq, Clone)]
pub enum Focus {
    Message(usize),
    Input
}

#[derive(SmartDefault)]
pub struct Model {
    messages: Vec<Message>,

    #[default(StreamingState::Idle)]
    streaming_state: StreamingState,

    #[default(Focus::Input)]
    focus: Focus,

    #[default("ready")]
    pub left_status: String,
    pub right_status: Option<String>,

    pub spinner_index: usize,

    quit_count: u16,

    pub input_box: String,
}

impl Model {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn all_messages(&self) -> &Vec<Message> {
        &self.messages
    }

    pub fn all_messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }

    pub fn content_messages(&self) -> Vec<&Message> {
        self.messages.iter()
            .filter(|m| matches!(
                m.kind,
                MessageKind::User | MessageKind::Response | MessageKind::Thinking
            ))
            .collect()
    }

    pub fn streaming_state(&self) -> StreamingState {
        self.streaming_state.clone()
    }

    pub fn increment_quit(&mut self) {
        self.quit_count += 1;
    }

    pub fn cancel_quit(&mut self) {
        self.quit_count = 0;
    }

    pub fn should_quit(&mut self) -> bool {
        self.quit_count > 1
    }

    pub fn last_index(&self) -> usize {
        self.messages.len().saturating_sub(1)
    }

    /// the indices that contain valid content to focus on and edit
    pub fn focusable_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.messages.iter()
            .enumerate()
            .filter(|(_, m)| m.kind != MessageKind::Error)
            .map(|(i, _)| i)
            .collect();
        indices.push(self.messages.len());
        indices
    }

    pub fn num_messages(&self) -> usize {
        self.messages.len()
    }

    pub fn new_input(&mut self, text: &str) -> bool {
        if self.streaming_state() == StreamingState::Streaming {
            return false;
        }
        if text.is_empty() && self.messages.is_empty() {
            return false;
        }

        if let Some(m) = self.messages.last() && m.kind == MessageKind::Error {
            self.messages.pop();
        }

        if !text.is_empty() {
            self.messages.push(Message {
                index: self.num_messages(),
                kind: MessageKind::User,
                content: text.to_string(),
            });
        }

        self.messages.push(Message {
            index: self.num_messages(),
            kind: MessageKind::ResponsePlaceholder,
            content: String::new(),
        });

        self.streaming_state = StreamingState::AwaitingStream;
        self.input_box = String::new();
        self.left_status = "streaming".to_string();

        true
    }

    pub fn push_think_token(&mut self, token: &str) {
        if self.streaming_state() == StreamingState::AwaitingStream {
            self.streaming_state = StreamingState::Streaming;
            if let Some(last) = self.messages.last_mut() {
                last.kind = MessageKind::Thinking;
                last.content = token.to_string();
            }

            return;
        }

        if let Some(last) = self.messages.last_mut() {
            last.kind = MessageKind::Thinking;
            last.content.push_str(token);
        }
    }

    pub fn push_stream_token(&mut self, token: &str) {
        if self.streaming_state() == StreamingState::AwaitingStream {
            self.streaming_state = StreamingState::Streaming;
            if let Some(last) = self.messages.last_mut() {
                last.kind = MessageKind::Response;
                last.content = token.to_string();
            }
            return;
        }

        if let Some(last) = self.messages.last_mut() {
            if last.kind == MessageKind::Thinking {
                self.messages.push(Message {
                    index: self.num_messages(),
                    kind: MessageKind::Response,
                    content: token.to_string(),
                });

                return;
            }
            last.content.push_str(token);
        }
    }

    pub fn finish_stream(&mut self) {
        self.streaming_state = StreamingState::Idle;
        self.left_status = "ready".to_string();
    }

    pub fn error_stream(&mut self, msg: &impl Display) {
        if let Some(MessageKind::ResponsePlaceholder)
            = self.messages.last_mut().map(|m| m.kind)
        {
            self.messages.pop();
        }

        self.messages.push(Message {
            index: self.num_messages(),
            kind: MessageKind::Error,
            content: msg.to_string(),
        });
        self.streaming_state = StreamingState::Idle;
        self.left_status = "error".to_string();
    }

    pub fn focus(&self) -> &Focus {
        &self.focus
    }

    /// safely clamps values
    pub fn focus_on(&mut self, i: usize) {
        if i >= self.messages.len() {
            self.focus = Focus::Input;
        } else {
            self.focus = Focus::Message(i);
        }
    }

    pub fn focus_up(&mut self, i: usize) {
        let current_i = match self.focus {
            Focus::Message(i) => {
                i
            },
            Focus::Input => {
                self.messages.len()
            },
        };

        self.focus_on(current_i.saturating_sub(i));
    }

    pub fn focus_down(&mut self, i: usize) {
        let current_i = match self.focus {
            Focus::Message(i) => {
                i
            },
            Focus::Input => {
                self.messages.len()
            },
        };

        self.focus_on(current_i.saturating_add(i));
    }

    pub fn focus_on_input(&mut self) {
        self.focus = Focus::Input
    }
}
