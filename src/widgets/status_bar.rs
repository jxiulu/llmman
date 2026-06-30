use ratatui::{
    layout::{
        Constraint, Direction, Layout, Rect
    }, prelude::Buffer, style::{
        Color, Modifier, Style, Stylize
    }, widgets::{
        Block, Padding, Paragraph, Widget
    }
};

use crate::model::{
    Model, StreamingState
};

pub struct StatusBar {
    left: String,
    right: String,
}

impl StatusBar {
    const SPINNER_GLYPHS: [char; 4] = ['|', '/', '-', '\\'];

    pub fn new(model: &Model) -> Self {
        let spinner_glyph = match model.streaming_state() {
            StreamingState::Streaming | StreamingState::AwaitingStream => {
                Self::SPINNER_GLYPHS[model.spinner_index % Self::SPINNER_GLYPHS.len()]
            },
            _ => {
                ' '
            }
        };

        let left = format!("{} {}", model.left_status, spinner_glyph);
        let right = model.right_status.clone()
            .unwrap_or_default();

        Self {
            left,
            right,
        }
    }
}

impl Widget for &StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized
    {
        let areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Max(self.right.len() as u16)
            ])
            .split(area);

        let status_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);

        let block = Block::new()
            .padding(Padding::new(0, 0, 0, 1))
            .style(status_style);

        let left_side = Paragraph::new(self.left.clone())
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC)
            .block(block.clone());
        left_side.render(areas[0], buf);

        let right_side = Paragraph::new(self.right.clone())
            .block(block);
        right_side.render(areas[1], buf);
    }
}
