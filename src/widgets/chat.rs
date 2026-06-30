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
        FocusState, Model
    },
    widgets::{
        self, InputBox
    }
};

#[derive(PartialEq, Eq, Clone)]
pub enum Scroll {
    Offset(usize),
    Max,

    // snap to focus. if the focus is the input box, do nothing
    Focus
}

/// state-tracking
pub struct Chat {
    pub scroll: Scroll,
    last_scroll_offset: usize,
    last_focus_state: FocusState,
    input_box: InputBox,
}

impl Chat {
    pub fn new() -> Self {
        let input_box = InputBox::new("");

        Self {
            scroll: Scroll::Max,
            last_scroll_offset: 0,
            last_focus_state: FocusState::Input,
            input_box,
        }
    }

    /// syncs the state with the model
    /// focus state sync
    pub fn sync(&mut self, model: &mut Model) {
        if &self.last_focus_state == model.focus_state() {
            return;
        }

        let content = self.input_box.lines().join("\n");
        match self.last_focus_state {
            FocusState::Message(i) => model.messages[i].content = content,
            FocusState::Input => model.input_box = content,
        }

        let content = match model.focus_state() {
            FocusState::Message(i) => model.messages[*i].content.clone(),
            FocusState::Input => model.input_box.clone(),
        };

        self.input_box = InputBox::new(&content);
        self.last_focus_state = model.focus_state().clone();
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
        self.scroll = Scroll::Offset(self.last_scroll_offset.saturating_sub(i));
    }

    pub fn scroll_down(&mut self, i: usize) {
        self.scroll = Scroll::Offset(self.last_scroll_offset.saturating_add(i));
    }
}


pub struct ChatWidget<'m> {
    messages: &'m Vec<crate::model::Message>,
    focus: &'m FocusState,
}

impl<'m> ChatWidget<'m> {
    pub fn new(model: &'m Model) -> Self {
        Self {
            messages: &model.messages,
            focus: &model.focus_state()
        }
    }

    fn scroll_offset(&self, area: Rect, effective_heights: &[usize], state: &Chat) -> usize {
        let total_height: usize = effective_heights.iter().sum();
        let max_scroll = total_height.saturating_sub(area.height as usize);

        match state.scroll {
            Scroll::Offset(i) => {
                i.min(max_scroll)
            },
            Scroll::Max => {
                max_scroll
            },
            Scroll::Focus => {
                match self.focus {
                    FocusState::Message(i) => {
                        let focused_offset: usize = effective_heights[..*i].iter().sum();
                        let focused_height = effective_heights[*i];
                        let s = if focused_height <= area.height as usize {
                            (focused_offset + focused_height)
                                .saturating_sub(area.height as usize)
                        } else {
                            focused_offset
                        };

                        s.min(max_scroll)
                    },
                    _ => {
                        state.last_scroll_offset
                    }
                }
            },
        }
    }

    fn render_chat(&self, area: Rect, buf: &mut Buffer, state: &Chat) -> usize {
        let messages: Vec<widgets::Message> = self.messages.iter()
            .map(|m| {
                let mut widget = widgets::Message::new(m);
                widget.init_wrap(area.width as usize);
                widget
            })
            .collect();

        let focused_idx = match self.focus {
            FocusState::Message(i) => Some(*i),
            _ => None,
        };
        let input_box_height = state.input_box.total_widget_height(area.width as usize);

        // Use input_box_height for the focused message; all others use message height.
        let effective_heights: Vec<usize> = messages.iter()
            .enumerate()
            .map(|(i, m)| if Some(i) == focused_idx { input_box_height } else { m.height() })
            .collect();

        let scroll_offset = self.scroll_offset(area, &effective_heights, state);
        let mut remaining_skip = scroll_offset;
        let mut messages = messages.into_iter()
            .enumerate()
            .peekable();

        // skip to in view using effective heights
        while let Some((i, _)) = messages.peek() {
            if remaining_skip < effective_heights[*i] {
                break;
            }
            remaining_skip -= effective_heights[*i];
            messages.next();
        }

        // render each message
        let mut y = area.y as usize;
        let bottom = (area.y + area.height) as usize;

        for (i, mut message) in messages {
            if y >= bottom {
                break;
            }

            let effective_h = effective_heights[i];
            let render_height;

            match self.focus {
                FocusState::Message(foc) if foc == &i => {
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
        let scroll_offset = match self.focus {
            FocusState::Message(_) => {
                self.render_chat(area, buf, state)
            },
            FocusState::Input => {
                let areas = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Fill(1),
                        Constraint::Max(state.input_box.total_widget_height(area.width as usize) as u16)
                    ])
                    .split(area);

                state.input_box.render(areas[1], buf);
                self.render_chat(areas[0], buf, state)
            }
        };

        state.last_scroll_offset = scroll_offset
    }
}
