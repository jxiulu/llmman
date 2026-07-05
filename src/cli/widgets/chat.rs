use ratatui::{
    layout::{
        Constraint, Direction, Layout
    },
    prelude::{
        Buffer, Rect
    },
    widgets::{
        StatefulWidget, Widget
    }
};
use tui_textarea::Input;

use crate::{
    model::{
        Focus, Model
    },
    cli::widgets::{
        self, InputBox,
        scroll::{ScrollOpt, ScrollPos, ChatCoords}
    }
};

pub enum VisibilityOpt {
    All,
    NonThinking,
}

/// state-tracking
pub struct Chat {
    pub scroll: ScrollOpt,
    pos: ScrollPos,
    last_focus_state: Focus,
    input_box: InputBox,
    visibility_opt: VisibilityOpt,
}

impl Chat {
    pub fn new() -> Self {
        let input_box = InputBox::new("");

        Self {
            scroll: ScrollOpt::Max,
            pos: ScrollPos { message: 0, skip: 0 },
            last_focus_state: Focus::Input,
            input_box,
            visibility_opt: VisibilityOpt::All,
        }
    }

    /// syncs the state with the model
    /// focus state sync
    pub fn sync(&mut self, model: &mut Model) {
        if &self.last_focus_state == model.focus() {
            return;
        }

        let content = self.input_box.lines().join("\n");
        match self.last_focus_state {
            Focus::Message(i) => model.all_messages_mut()[i].content = content,
            Focus::Input => model.input_box = content,
        }

        let foc_state = model.focus().clone();
        let content = match foc_state {
            Focus::Message(i) => model.all_messages_mut()[i].content.clone(),
            Focus::Input => model.input_box.clone(),
        };

        self.input_box = InputBox::new(&content);
        self.last_focus_state = model.focus().clone();
    }

    pub fn input(&mut self, input: impl Into<Input>) -> bool {
        self.input_box.input(input)
    }

    pub fn lines(&self) -> &[String] {
        self.input_box.lines()
    }

    pub fn clear_input(&mut self) {
        self.input_box.clear();
    }

    pub fn scroll_up(&mut self, i: usize) {
        self.scroll = ScrollOpt::Delta(-(i as i64));
    }

    pub fn scroll_down(&mut self, i: usize) {
        self.scroll = ScrollOpt::Delta(i as i64);
    }

    pub fn input_box_height(&self, width: usize) -> usize {
        self.input_box.total_widget_height(width)
    }

    pub fn visibility_opt(&self) -> &VisibilityOpt {
        &self.visibility_opt
    }
    pub fn set_visibility_opt(&mut self, filter_opt: VisibilityOpt) {
        self.visibility_opt = filter_opt;
    }
}


pub struct ChatWidget<'m> {
    messages: &'m Vec<crate::model::Message>,
    focus: &'m Focus,
}

impl<'m> ChatWidget<'m> {
    pub fn new(model: &'m Model) -> Self {
        Self {
            messages: model.all_messages(),
            focus: model.focus()
        }
    }

    /// resolves `state.scroll` into a top-of-viewport row via `scroller` (which is
    /// effective-height aware: it substitutes the input box's height for whichever
    /// message is focused), then re-anchors `state.pos` to that row so future
    /// `ScrollOpt::Delta` scrolling stays stable across content/visibility changes
    /// elsewhere in the chat.
    fn resolve_scroll(&self, area: Rect, coords: &ChatCoords, state: &mut Chat) -> usize {
        let max_scroll = coords.total_height().saturating_sub(area.height as usize);

        let target_row = match state.scroll {
            ScrollOpt::Delta(delta) => {
                let new_pos = if delta >= 0 {
                    coords.pos_below(&state.pos, delta as usize)
                } else {
                    coords.pos_above(&state.pos, (-delta) as usize)
                };
                coords.at_pos(&new_pos)
            },
            ScrollOpt::Max => max_scroll,
            ScrollOpt::Focus => {
                match coords.focused_index() {
                    Some(i) => {
                        let focused_offset = coords.rows_before(i);
                        let focused_height = coords.height(i);
                        if focused_height <= area.height as usize {
                            (focused_offset + focused_height)
                                .saturating_sub(area.height as usize)
                        } else {
                            focused_offset
                        }
                    },
                    None => {
                        coords.at_pos(&state.pos)
                    }
                }
            },
        };

        let row = target_row.min(max_scroll);

        // a delta is a one-shot nudge; neutralize it so idle re-renders don't
        // keep re-applying it
        if matches!(state.scroll, ScrollOpt::Delta(_)) {
            state.scroll = ScrollOpt::Delta(0);
        }
        state.pos = coords.at_row(row);

        row
    }

    fn render_chat(&self, area: Rect, buf: &mut Buffer, state: &mut Chat) -> usize {
        let messages: Vec<widgets::Message> = self.messages.iter()
            .map(|m| {
                let mut widget = widgets::Message::new(m, &*state);
                {widget.init_wrap(area.width as usize);}
                widget
            })
            .collect();

        let coords = ChatCoords::from(&messages, self.focus, state, area);
        let scroll_offset = self.resolve_scroll(area, &coords, state);
        let heights: Vec<usize> = (0..messages.len()).map(|i| coords.height(i)).collect();

        let mut remaining_skip = state.pos.skip;
        let messages = messages.into_iter()
            .enumerate()
            .skip(state.pos.message);

        let mut y = area.y as usize;
        let bottom = (area.y + area.height) as usize;

        for (i, mut message) in messages {
            if y >= bottom {
                break;
            }

            let effective_h = heights[i];
            let render_height;

            match self.focus {
                Focus::Message(foc) if foc == &i => {
                    render_height = (effective_h - remaining_skip).min(bottom - y);
                    let render_area = Rect {
                        x: area.x,
                        y: y as u16,
                        width: area.width,
                        height: render_height as u16,
                    };

                    state.input_box.render(render_area, buf);
                },
                _ => {
                    render_height = (effective_h - remaining_skip).min(bottom - y);
                    let render_area = Rect {
                        x: area.x,
                        y: y as u16,
                        width: area.width,
                        height: render_height as u16,
                    };

                    message.skip(remaining_skip);
                    message.render(render_area, buf);
                }
            }

            y += render_height;
            remaining_skip = 0;
        }

        scroll_offset
    }
}

impl<'m> StatefulWidget for &ChatWidget<'m> {
    type State = Chat;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        match self.focus {
            Focus::Message(_) => {
                self.render_chat(area, buf, state);
            },
            Focus::Input => {
                let areas = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Fill(1),
                        Constraint::Max(state.input_box.total_widget_height(area.width as usize) as u16)
                    ])
                    .split(area);

                state.input_box.render(areas[1], buf);
                self.render_chat(areas[0], buf, state);
            }
        };
    }
}
