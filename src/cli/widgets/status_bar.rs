use ratatui::{
    layout::{
        Alignment, Constraint, Direction, Layout, Rect
    },
    prelude::Buffer,
    style::{
        Color, Modifier, Style, Stylize
    },
    text::{
        Line, Span
    },
    widgets::{
        Block, Paragraph, Widget
    }
};
use unicode_width::UnicodeWidthStr;

use crate::model::{
    Model, Notification, NotificationKind, StreamActivity
};

pub struct StatusBar<'a> {
    spinner: Option<char>,
    notifications: Vec<&'a Notification>,
    left: &'static str,
    left_style: Style
}

impl<'a> StatusBar<'a> {
    const SPINNER_GLYPHS: [char; 4] = ['|', '/', '-', '\\'];

    pub fn new(model: &'a Model) -> Self {
        let spinner_glyph = match model.streaming_state() {
            StreamActivity::Streaming | StreamActivity::AwaitingStream => {
                Some(
                    Self::SPINNER_GLYPHS[model.spinner_index % Self::SPINNER_GLYPHS.len()]
                )
            },
            _ => {
                None
            }
        };

        let (left, left_style) = match model.streaming_state() {
            StreamActivity::Streaming => {
                let style = Style::default().bg(Color::Rgb(200, 200, 50));
                (" streaming ", style)
            },
            StreamActivity::AwaitingStream => {
                let style = Style::default().bg(Color::Rgb(75, 75, 0));
                (" waiting for response ", style)
            },
            StreamActivity::Idle => {
                let style = Style::default();
                (" idle ", style)
            },
            StreamActivity::Error => {
                let style = Style::default().bg(Color::Rgb(100, 0, 0));
                (" error ", style)
            }
        };

        Self {
            spinner: spinner_glyph,
            notifications: model.all_notifications(),
            left,
            left_style,
        }
    }

    fn render_notifications(&self, area: Rect, buf: &mut Buffer) {
        let spans: Vec<Span> = self.notifications.iter()
            .map(|n| {
                let body = format!(" {} ", &n.message);
                let style = match n.kind {
                    NotificationKind::Info => {
                        Style::default().bg(Color::Rgb(0, 0, 100))
                    },
                    NotificationKind::Warning => {
                        Style::default().bg(Color::Rgb(20, 100, 0))
                    },
                    NotificationKind::Error => {
                        Style::default().bg(Color::Rgb(100, 0, 0))
                    },
                };

                Span::from(body).style(style)
            })
            .collect();

        let line = Line::from(spans);
        let overflow = line.width().saturating_sub(area.width as usize) as u16;

        Paragraph::new(line)
            .alignment(Alignment::Right)
            .scroll((0, overflow))
            .render(area, buf);
    }
}

impl<'a> Widget for &StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized
    {
        let areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Max(if self.spinner.is_some() { 3 } else { 0 }),
                Constraint::Max(self.left.width() as u16),
                Constraint::Fill(1)
            ])
            .split(area);

        let spinner = Paragraph::new(
            self.spinner.map(|c| format!(" {c} "))
                .unwrap_or("".to_string())
        );
        spinner.render(areas[0], buf);

        let left_side = Paragraph::new(self.left)
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC)
            .style(self.left_style);
        left_side.render(areas[1], buf);

        self.render_notifications(areas[2], buf);
    }
}
