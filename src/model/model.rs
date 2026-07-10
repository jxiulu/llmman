use std::{
    collections::VecDeque, fmt::Display,
    time::{
        Duration, Instant
    }
};

use smart_default::SmartDefault;

use super::{
    Message, MessageKind, Notification, NotificationKind
};

#[derive(PartialEq, Eq, Clone)]
pub enum StreamActivity {
    Streaming,
    AwaitingStream,
    Idle,
    Error,
}

/// What box is currently being focused on right now
#[derive(PartialEq, Eq, Clone)]
pub enum Focus {
    Select(usize),
    Edit(usize),
    Input,
}

#[derive(SmartDefault)]
pub struct Model {
    messages: Vec<Message>,

    notifications: VecDeque<Notification>,

    #[default(StreamActivity::Idle)]
    stream_activity: StreamActivity,

    #[default(Focus::Input)]
    focus: Focus,

    pub spinner_index: usize,

    pub input_box: String,
}

impl Model {
    pub fn new() -> Self {
        Self::default()
    }

    /// pls call every tick
    pub fn tick(&mut self) {
        let now = Instant::now();
        self.notifications.retain(|n| n.expiry > now);
    }

    pub fn notify(
        &mut self, kind: NotificationKind, message: impl Display, duration: Duration
    ) {
        self.notifications.push_back(Notification::new(kind, message, duration));
    }

    pub fn all_notifications(&self) -> Vec<&Notification> {
        self.notifications.iter().collect()
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

    pub fn push_message(&mut self, kind: MessageKind, content: impl Display) {
        self.messages.push(Message::new(
            self.messages.len(),
            kind,
            content
        ));
    }

    pub fn streaming_state(&self) -> StreamActivity {
        self.stream_activity.clone()
    }

    pub fn last_msg_idx(&self) -> usize {
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

    //
    // streaming
    //

    pub fn new_input(&mut self, text: &str) -> bool {
        if self.streaming_state() == StreamActivity::Streaming {
            return false;
        }
        if text.is_empty() && self.messages.is_empty() {
            return false;
        }

        if let Some(m) = self.messages.last() && m.kind == MessageKind::Error {
            self.messages.pop();
        }

        if !text.is_empty() {
            self.messages.push(Message::new(
                self.num_messages(),
                MessageKind::User,
                text.to_string(),
            ));
        }

        self.push_message(
            MessageKind::ResponsePlaceholder,
            String::new(),
        );

        self.stream_activity = StreamActivity::AwaitingStream;
        self.input_box = String::new();

        true
    }

    pub fn push_think_token(&mut self, token: &str) {
        if self.streaming_state() == StreamActivity::AwaitingStream {
            self.stream_activity = StreamActivity::Streaming;
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
        if self.streaming_state() == StreamActivity::AwaitingStream {
            self.stream_activity = StreamActivity::Streaming;
            if let Some(last) = self.messages.last_mut() {
                last.kind = MessageKind::Response;
                last.content = token.to_string();
            }
            return;
        }

        if let Some(last) = self.messages.last_mut() {
            if last.kind == MessageKind::Thinking {
                self.push_message(
                    MessageKind::Response,
                    token.to_string(),
                );

                return;
            }
            last.content.push_str(token);
        }
    }

    pub fn finish_stream(&mut self) {
        self.stream_activity = StreamActivity::Idle;
    }

    pub fn error_stream(&mut self, msg: &impl Display) {
        if let Some(MessageKind::ResponsePlaceholder)
            = self.messages.last_mut().map(|m| m.kind)
        {
            self.messages.pop();
        }

        self.push_message(
            MessageKind::Error,
            msg.to_string(),
        );
        self.stream_activity = StreamActivity::Error;
    }

    //
    // focusing
    //
    
    pub fn stop_editing_current(&mut self) {
        match self.focus {
            Focus::Select(_) => {},
            Focus::Edit(i) => self.focus = Focus::Select(i),
            Focus::Input => {}
        }
    }

    pub fn start_editing_current(&mut self) {
        match self.focus {
            Focus::Select(i) => self.focus = Focus::Edit(i),
            Focus::Edit(_) => {},
            Focus::Input => {},
        }
    }

    pub fn stream_active(&self) -> bool {
        matches!(
            self.stream_activity,
            StreamActivity::AwaitingStream | StreamActivity::Streaming
        )
    }

    pub fn current_focus(&self) -> &Focus {
        &self.focus
    }

    /// only use if clamping!!!
    fn current_focus_index(&self) -> usize {
        match self.focus {
            Focus::Select(i) => i,
            Focus::Edit(i) => i,
            Focus::Input => self.messages.len(),
        }
    }

    /// safely clamps values
    /// keeps current focus state (edit, select) if possible
    pub fn move_focus(&mut self, i: usize) {
        if i >= self.messages.len() {
            self.focus = Focus::Input;
        } else {
            self.focus = match self.focus {
                Focus::Select(_) => Focus::Select(i),
                Focus::Edit(_) => Focus::Edit(i),
                Focus::Input => Focus::Select(i),
            }
        }
    }

    pub fn move_focus_up(&mut self, i: usize) {
        self.move_focus(self.current_focus_index().saturating_sub(i));
    }

    pub fn move_focus_down(&mut self, i: usize) {
        self.move_focus(self.current_focus_index().saturating_add(i));
    }

    pub fn set_focus(&mut self, opt: Focus) {
        self.focus = opt
    }
}
