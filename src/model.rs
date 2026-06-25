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
pub enum UiState {
    Focused(usize),
    Unfocused
}

pub struct Model {
    pub chat: Vec<Message>,

    /// this field should hold the number of rows from the top that the view is offset,
    /// including any ui padding/whitespace
    pub scroll: u16,
    pub should_follow: bool,
    pub should_snap: bool,
    pub is_streaming: bool,
    pub is_awaiting_stream: bool,

    pub streaming_state: StreamingState,
    pub ui_state: UiState,

    pub status: String,
    pub ui_status: Option<String>,

    pub spinner_index: usize,

    pub quit_count: u16,

    pub focus: Option<usize>,
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
            is_streaming: false,
            is_awaiting_stream: false,
            status: "ready".to_string(),
            ui_status: None,
            spinner_index: 0,
            quit_count: 0,
            focus: None,
            input_buffer: String::new(),
            ui_state: UiState::Focused(0),
        }
    }

    pub fn streaming_state(&self) -> StreamingState {
        self.streaming_state.clone()
    }

    pub fn set_streaming_state(&mut self, s: StreamingState) {
        self.streaming_state = s;
    }

    pub fn ui_state(&self) -> UiState {
        self.ui_state().clone()
    }

    pub fn set_ui_state(&mut self, s: UiState) {
        self.ui_state = s;
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
        self.focus = Some(idx);

        self.set_ui_state(UiState::Focused(idx));
        self.should_snap = true;
        self.should_follow = false;

        tracing::info!("model told to focus on index {idx}")
    }

    pub fn unfocus(&mut self) {
        self.should_follow = self.focus == Some(self.chat.len());

        self.should_follow = self.ui_state() == UiState::Focused(self.chat.len());
        self.focus = None;

        tracing::info!("model unfocused");
    }

    pub fn snap(&mut self) {
        self.should_snap = true;
        if self.focus.is_none() {
            self.focus = Some(self.chat.len());
        }

        if self.ui_state() == UiState::Unfocused {
            self.set_ui_state(UiState::Focused(self.chat.len()));
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
        if self.is_streaming {
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

        self.is_awaiting_stream = true;
        self.is_streaming = true;
        self.should_follow = true;
        self.focus = None;
        self.input_buffer = String::new();
        self.status = "streaming".to_string();

        true
    }

    pub fn push_think_token(&mut self, token: &str) {
        if self.is_awaiting_stream {
            self.is_awaiting_stream = false;
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
        if self.is_awaiting_stream {
            self.is_awaiting_stream = false;
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
        self.is_streaming = false;
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
        self.is_streaming = false;
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
