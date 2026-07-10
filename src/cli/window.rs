use ratatui::{
    Frame,
    layout::{
        Constraint, Direction, Layout
    },
    style::{
        Color, Style
    },
    widgets::Block
};

use super::widgets::{
    Chat, ChatWidget, StatusBar
};

use crate::model::Model;

#[derive(Default)]
pub struct Window {
    pub chat: Chat
}

impl Window {
    pub fn new() -> Self {
        Self {
            chat: Chat::new()
        }
    }

    pub fn submit_input(&mut self, model: &mut Model) {
        let input = self.chat.lines().join("\n");
        if model.new_input(&input) {
            self.chat.clear_input();
        }
    }
}

pub fn sync(view: &mut Window, model: &mut Model) {
    view.chat.sync(model);
}

pub fn draw(state: &mut Window, model: &Model, f: &mut Frame) {
    let total_area = f.area();

    let default_style = Style::default()
        .bg(Color::Rgb(30, 30, 30))
        .fg(Color::Rgb(210, 220, 230));

    let bg = Block::default().style(default_style);

    f.render_widget(bg, total_area);

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Max(1), Constraint::Fill(1)
        ])
        .split(total_area);

    let status_bar_widget = StatusBar::new(model);
    f.render_widget(&status_bar_widget, areas[0]);

    let chat_widget = ChatWidget::new(model);
    f.render_stateful_widget(&chat_widget, areas[1], &mut state.chat);
}
