use std::{borrow::Cow, rc::Rc};

use ratatui::{
    prelude::{
        Buffer, Rect
    },
    style::{
        Color, Modifier, Style, Stylize
    },
    text::{
        Line, Text
    },
    widgets::{
        Block, Padding, Paragraph, Widget
    }
};

use crate::model::{
    self, MessageKind, Focus
};
use super::{
    Chat, VisMode
};

struct WrappedTextCache {
    lines: Vec<Rc<str>>,
    width: usize,
}

pub struct Message {
    index: usize,
    skip: usize,
    kind: MessageKind,
    selected: bool,
    content: Rc<str>,
     
    wrapped: Option<WrappedTextCache>,
    visible: bool,
}

const PADDING: usize = 2;

impl Message {
    pub fn new(msg: &model::Message, state: &Chat) -> Self {
        let content: Rc<str> = match msg.kind {
            MessageKind::Thinking if state.vis_mode() == &VisMode::NonThinking => {
                format!("Thinking... {} words", msg.content.split_whitespace().count())
                    .into()
            },
            MessageKind::ResponsePlaceholder => "Waiting for response...".into(),
            _ => msg.content.as_str().into()
        };

        let selected = matches!(
            state.focus_state,
            Focus::Select(i) if i == msg.index
        );

        Self {
            index: msg.index,
            skip: 0,
            kind: msg.kind,
            selected,
            content,
            wrapped: None,
            visible: true,
        }
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn skip(&mut self, lines: usize) {
        self.skip = lines;
    }

    pub fn height(&self) -> usize {
        if !self.visible() {
            return 0;
        }

        self.wrapped.as_ref()
            .map(|w| w.lines.len().saturating_add(PADDING))
            .unwrap_or(1 + PADDING)
    }

    /// area width is the TOTAL width given to the widget
    pub fn init_wrap(&mut self, area_width: usize) {
        let wrap_width = area_width.saturating_sub(PADDING);
        let wrapped: Vec<Rc<str>> = textwrap::wrap(&self.content, wrap_width)
            .into_iter()
            .map(|c| c.into())
            .collect();

        self.wrapped = Some(WrappedTextCache { lines: wrapped, width: wrap_width});
    }

    fn mod_color(&self, r: u8, g: u8, b: u8) -> Color {
        let m: f32 = if self.selected { 1.4 } else { 1.0 };

        Color::Rgb(
            (r as f32 * m).round() as u8, 
            (g as f32 * m).round() as u8,
            (b as f32 * m).round() as u8
        )
    }

    fn message_block<'a>(&'a self) -> Block<'a> {
        let top_pad: u16 = if self.skip > 0 { 0 } else { 1 };
        let block = match self.kind {
            MessageKind::Response => {
                let assistant_style = Style::default().bg(self.mod_color(30, 30, 30));

                Block::default()
                    .style(assistant_style)
                    .padding(Padding::new(1, 1, 0, 1))
            },
            MessageKind::ResponsePlaceholder => {
                let assistant_style = Style::default().bg(self.mod_color(20, 20, 20));

                Block::default()
                    .style(assistant_style)
                    .padding(Padding::new(1, 1, 0, 1))
            },
            MessageKind::User => {
                let user_style = Style::default()
                    .bg(self.mod_color(10, 10, 10));

                Block::default()
                    .style(user_style)
                    .padding(Padding::new(1, 1, 0, 1))
            },
            MessageKind::Thinking => {
                let thinking_style = Style::default()
                    .bg(self.mod_color(20, 20, 20))
                    .fg(Color::Rgb(125, 125, 125));

                Block::default()
                    .style(thinking_style)
                    .padding(Padding::new(1, 1, 0, 0))
            },
            MessageKind::Error => {
                let error_style = Style::default().bg(Color::Red);

                Block::default()
                    .style(error_style)
                    .padding(Padding::new(1, 1, 0, 1))
            },
        };

        match top_pad {
            0 => block,
            _ => {
                let title = Line::from(format!("{} ", self.index + 1))
                    .right_aligned()
                    .fg(Color::Rgb(50, 50, 50));
                block.title(title)
            }
        }
    }

}

impl Widget for &Message {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized
    {
        let content_skip = self.skip.saturating_sub(1);

        let body: Vec<Cow<str>> = match &self.wrapped {
            Some(w)
                if w.width == (area.width as usize).saturating_sub(PADDING)
            => {
                w.lines.iter()
                    .map(|r| Cow::Borrowed(r.as_ref()))
                    .collect()
            },
            _ => {
                textwrap::wrap(
                    &self.content,
                    area.width.saturating_sub(PADDING as u16) as usize
                )
            },
        };

        let block: Block = self.message_block();

        let lines: Vec<Line> = body
            .into_iter()
            .map(Line::raw)
            .collect();

        let p = Paragraph::new(Text::from(lines))
            .block(block)
            .scroll((content_skip as u16, 0));

        p.render(area, buf);
    }
}
