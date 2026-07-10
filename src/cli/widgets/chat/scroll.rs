use ratatui::prelude::Rect;

use crate::{
    model::Focus,
    cli::widgets::{Chat, Message}
};

/// use this to keep a state
#[derive(Clone, Copy, Default)]
pub struct ScrollPos {
    /// absolute index of a message in the Model
    pub message: usize,

    /// line offset into that message that should be at the top of the viewport
    pub skip: usize,
}

#[derive(PartialEq, Eq, Default)]
pub enum Scroll {
    #[default]
    Max,

    /// please remember to clear this to Delta(0) or something else when done
    Delta(i64),
    Focus,
}

/// ephemeral. scroll logic
pub struct ChatCoords<'a> {
    messages: &'a [Message],
    editing: Option<usize>,
    input_box_height: usize,
}

impl<'a> ChatCoords<'a> {
    pub fn from(messages: &'a [Message], focus: &Focus, state: &Chat, area: Rect) -> Self {
        let editing = match focus {
            Focus::Edit(i) => Some(*i),
            _ => None
        };
        let input_box_height = state.input_box_height(area.width as usize);

        Self { messages, editing, input_box_height }
    }

    pub fn focused_index(&self) -> Option<usize> {
        self.editing
    }

    /// height a message actually renders at: the input box's height if
    /// it's the focused message, otherwise its own wrapped content height
    pub fn height(&self, index: usize) -> usize {
        if Some(index) == self.editing {
            self.input_box_height
        } else {
            self.messages[index].height()
        }
    }

    pub fn total_height(&self) -> usize {
        self.rows_before(self.messages.len())
    }

    pub fn pos_below(&self, pos: &ScrollPos, rows: usize) -> ScrollPos {
        self.at_row(self.at_pos(pos) + rows)
    }

    pub fn pos_above(&self, pos: &ScrollPos, rows: usize) -> ScrollPos {
        self.at_row(self.at_pos(pos).saturating_sub(rows))
    }

    /// nearest visible shape at or after `from`, if any
    fn next_visible(&self, from: usize) -> Option<usize> {
        (from..self.messages.len()).find(|&i| self.messages[i].visible())
    }

    /// nearest visible shape at or before `from`, if any
    fn prev_visible(&self, from: usize) -> Option<usize> {
        (0..=from).rev().find(|&i| self.messages[i].visible())
    }

    /// sum of heights of all visible shapes before `index`
    pub fn rows_before(&self, index: usize) -> usize {
        (0..index)
            .filter(|&i| self.messages[i].visible())
            .map(|i| self.height(i))
            .sum()
    }

    pub fn at_row(&self, row: usize) -> ScrollPos {
        if self.messages.is_empty() {
            return ScrollPos { message: 0, skip: 0 };
        }

        let mut remaining = row;
        for (i, message) in self.messages.iter().enumerate() {
            if !message.visible() {
                continue;
            }
            let h = self.height(i);
            if remaining < h {
                return ScrollPos { message: i, skip: remaining };
            }
            remaining -= h;
        }

        // row is past the end of the visible content: clamp to the bottom
        // of the last visible message
        match self.prev_visible(self.messages.len().saturating_sub(1)) {
            Some(i) => ScrollPos {
                message: i,
                skip: self.height(i).saturating_sub(1),
            },
            None => ScrollPos { message: 0, skip: 0 },
        }
    }

    pub fn at_pos(&self, pos: &ScrollPos) -> usize {
        if self.messages.is_empty() {
            return 0;
        }

        let msg_idx = pos.message.min(self.messages.len() - 1);

        let (anchor, skip) = if self.messages[msg_idx].visible() {
            (msg_idx, pos.skip.min(self.height(msg_idx).saturating_sub(1)))
        } else if let Some(i) = self.next_visible(msg_idx) {
            // the hidden message's content slides up to be replaced by
            // whatever comes after it, so anchor to the top of that
            (i, 0)
        } else if let Some(i) = self.prev_visible(msg_idx) {
            (i, self.height(i).saturating_sub(1))
        } else {
            return 0;
        };

        self.rows_before(anchor) + skip
    }
}
