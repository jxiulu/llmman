use std::time::Duration;

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
    cli::widgets::{
        self, ChatCoords, InputBox, Scroll, ScrollPos
    }, model::{
        Focus, Model, NotificationKind
    }
};


#[derive(PartialEq, Eq, Clone)]
pub enum VisMode {
    All,
    NonThinking,
}

#[derive(smart_default::SmartDefault)]
/// state-tracking
pub struct Chat {
    #[default(Scroll::Max)]
    pub scroll: Scroll,

    pub(super) pos: ScrollPos,

    /// updated before every render
    #[default(Focus::Input)]
    pub(super) focus_state: Focus,

    #[default(InputBox::new(""))]
    pub(super) input_box: InputBox,

    #[default(VisMode::All)]
    pub(super) vis_mode: VisMode,
}

impl Chat {
    pub fn new() -> Self {
        Self::default()
    }

    /// syncs the state with the model
    /// update stale old data
    pub fn sync(&mut self, model: &mut Model) {
        if &self.focus_state == model.current_focus() {
            return;
        }

        let last_msg = model.last_msg_idx();
        if matches!(
            self.focus_state,
            Focus::Edit(i) | Focus::Select(i) if i == last_msg
        ) && model.current_focus() == &Focus::Input {
            self.scroll = Scroll::Max;
            model.notify(
                NotificationKind::Info, "scroll pinned", Duration::from_secs(1)
            );
        }

        let content = self.input_box.lines().join("\n");
        match self.focus_state {
            Focus::Edit(i) => model.all_messages_mut()[i].content = content,
            Focus::Input | Focus::Select(_) => model.input_box = content,
        }

        let content = match model.current_focus().clone() {
            Focus::Edit(i) => model.all_messages_mut()[i].content.clone(),
            Focus::Input | Focus::Select(_) => model.input_box.clone(),
        };
        self.input_box = InputBox::new(&content);

        self.focus_state = model.current_focus().clone();
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
        self.scroll = Scroll::Delta(-(i as i64));
    }

    pub fn scroll_down(&mut self, i: usize) {
        self.scroll = Scroll::Delta(i as i64);
    }

    pub fn input_box_height(&self, width: usize) -> usize {
        self.input_box.total_widget_height(width)
    }

    pub fn vis_mode(&self) -> &VisMode {
        &self.vis_mode
    }
    pub fn set_visibility_opt(&mut self, filter_opt: VisMode) {
        self.vis_mode = filter_opt;
    }

    pub fn toggle_vis(&mut self) {
        match self.vis_mode {
            VisMode::All => self.vis_mode = VisMode::NonThinking,
            VisMode::NonThinking => self.vis_mode = VisMode::All,
        }
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
            focus: model.current_focus()
        }
    }

    fn resolve_scroll(&self, area: Rect, coords: &ChatCoords, state: &mut Chat) -> usize {
        let max_scroll = coords.total_height().saturating_sub(area.height as usize);

        let target_row = match state.scroll {
            Scroll::Delta(delta) => {
                let new_pos = if delta >= 0 {
                    coords.pos_below(&state.pos, delta as usize)
                } else {
                    coords.pos_above(&state.pos, (-delta) as usize)
                };
                coords.at_pos(&new_pos)
            },
            Scroll::Max => max_scroll,
            Scroll::Focus => {
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

        // reset delta
        if matches!(state.scroll, Scroll::Delta(_)) {
            state.scroll = Scroll::Delta(0);
        }
        state.pos = coords.at_row(row);

        row
    }

    fn render_chat(&self, area: Rect, buf: &mut Buffer, state: &mut Chat) -> usize {
        let messages: Vec<widgets::Message> = self.messages.iter()
            .map(|m| {
                let mut widget = widgets::Message::new(m, &*state);
                widget.init_wrap(area.width as usize);
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
                Focus::Edit(foc) if foc == &i => {
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
            Focus::Select(_) | Focus::Edit(_) => {
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
