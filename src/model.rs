use std::fmt::Display;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Response,
    User,
    Thinking,
    Error,
}

#[derive(Clone)]
pub struct Message {
    pub kind: MessageKind,
    pub content: String,
}

#[derive(PartialEq, Eq, Clone)]
pub enum StreamingState {
    Streaming,
    AwaitingStream,
    Idle,
}

#[derive(PartialEq, Eq, Clone)]
pub enum FocusState {
    Focused(usize),
    Unfocused
}

pub enum ScrollState {
    Following,
    Fixed(u16)
}

pub struct Model {
    pub chat: Vec<Message>,

    /// this field should hold the number of rows from the top that the view is offset,
    /// including any ui padding/whitespace
    pub scroll: u16,
    pub should_follow: bool,
    pub should_snap: bool,

    pub streaming_state: StreamingState,
    pub focus_state: FocusState,

    pub status: String,
    pub ui_status: Option<String>,

    pub spinner_index: usize,

    pub quit_count: u16,

    pub input_buffer: String,
}

impl Model {
    pub fn new() -> Self {
        Self {
            chat: Vec::new(),
            scroll: 0,
            should_follow: true,
            should_snap: false,
            streaming_state: StreamingState::Idle,
            status: "ready".to_string(),
            ui_status: None,
            spinner_index: 0,
            quit_count: 0,
            input_buffer: String::new(),
            focus_state: FocusState::Focused(0),
        }
    }

    pub fn input_buffer(&self) -> &str {
        &self.input_buffer
    }

    pub fn streaming_state(&self) -> StreamingState {
        self.streaming_state.clone()
    }

    pub fn set_streaming_state(&mut self, s: StreamingState) {
        self.streaming_state = s;
    }

    pub fn focus_state(&self) -> FocusState {
        self.focus_state.clone()
    }

    pub fn set_focus_state(&mut self, s: FocusState) {
        self.focus_state = s;
    }

    pub fn set_aux_status(&mut self, status: Option<&str>) {
        self.ui_status = status.map(|s| s.to_owned());
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

    pub fn focus_on(&mut self, idx: usize) {
        self.set_focus_state(FocusState::Focused(idx));
        self.should_snap = true;
        self.should_follow = false;

        tracing::info!("model told to focus on index {idx}")
    }

    pub fn unfocus(&mut self) {
        self.should_follow = self.focus_state() == FocusState::Focused(self.chat.len());
        self.set_focus_state(FocusState::Unfocused);

        tracing::info!("model unfocused");
    }

    pub fn snap(&mut self) {
        self.should_snap = true;
        if self.focus_state() == FocusState::Unfocused {
            self.set_focus_state(FocusState::Focused(self.chat.len()));
        }

        self.should_follow = false;

        tracing::info!("model snapped");
    }

    /// the indices that contain valid content to focus on and edit
    pub fn focusable_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.chat.iter()
            .enumerate()
            .filter(|(_, m)| m.kind != MessageKind::Error)
            .map(|(i, _)| i)
            .collect();
        indices.push(self.chat.len());
        indices
    }

    pub fn flush_input(&mut self, text: String) -> bool {
        if self.streaming_state() == StreamingState::Streaming {
            return false;
        }
        if text.is_empty() && self.chat.is_empty() {
            return false;
        }

        if !text.is_empty() {
            self.chat.push(Message {
                kind: MessageKind::User,
                content: text,
            });
        }

        self.chat.push(Message {
            kind: MessageKind::Response,
            content: String::new(),
        });

        self.set_streaming_state(StreamingState::AwaitingStream);
        self.should_follow = true;
        self.set_focus_state(FocusState::Unfocused);
        self.input_buffer = String::new();
        self.status = "streaming".to_string();

        true
    }

    pub fn push_think_token(&mut self, token: &str) {
        if self.streaming_state() == StreamingState::AwaitingStream {
            self.set_streaming_state(StreamingState::Streaming);
            if let Some(last) = self.chat.last_mut() {
                last.kind = MessageKind::Thinking;
                last.content = token.to_string();
            }

            return;
        }

        if let Some(last) = self.chat.last_mut() {
            last.kind = MessageKind::Thinking;
            last.content.push_str(token);
        }
    }

    pub fn push_stream_token(&mut self, token: &str) {
        if self.streaming_state() == StreamingState::AwaitingStream {
            self.set_streaming_state(StreamingState::Streaming);
            if let Some(last) = self.chat.last_mut() {
                last.kind = MessageKind::Response;
                last.content = token.to_string();
            }
            return;
        }

        if let Some(last) = self.chat.last_mut() {
            if last.kind == MessageKind::Thinking {
                self.chat.push(Message {
                    kind: MessageKind::Response,
                    content: token.to_string(),
                });

                return;
            }
            last.content.push_str(token);
        }
    }

    pub fn finish_stream(&mut self) {
        self.set_streaming_state(StreamingState::Idle);
        self.should_follow = false;
        self.status = "ready".to_string();
    }

    pub fn error_stream(&mut self, msg: &impl Display) {
        if let Some(last) = self.chat.last_mut()
            && last.content.is_empty()
        {
            self.chat.pop();
        }

        self.chat.push(Message {
            kind: MessageKind::Error,
            content: msg.to_string(),
        });
        self.set_streaming_state(StreamingState::Idle);
        self.should_follow = false;
        self.status = "error".to_string();
    }

    pub fn scroll_up(&mut self) {
        self.should_follow = false;
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.should_follow = false;
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn last_index(&self) -> usize {
        self.chat.len().saturating_sub(1)
    }
}
