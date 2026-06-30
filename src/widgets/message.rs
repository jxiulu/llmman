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

use crate::{
    model::{
        self, MessageKind
    },
    wrapped_text::WrappedText
};

pub struct Message<'a> {
    skip: usize,
    kind: MessageKind,
    content: &'a str,
    wrapped: Option<WrappedText<'a>>
}

const PADDING: usize = 2;

impl<'a> Message<'a> {
    pub fn new(model: &'a model::Message) -> Self {
        Self {
            skip: 0,
            kind: model.kind,
            content: &model.content,
            wrapped: None
        }
    }

    pub fn skip(&mut self, lines: usize) {
        self.skip = lines;
    }

    pub fn height(&self) -> usize {
        self.wrapped.as_ref()
            .map(|w| w.height().saturating_add(PADDING))
            .unwrap_or(1)
    }

    /// area width is the TOTAL width given to the widget
    pub fn init_wrap(&mut self, area_width: usize) -> &[&'a str] {
        let wrapped = WrappedText::new(self.content, area_width.saturating_sub(PADDING));

        self.wrapped = Some(wrapped);

        match &self.wrapped {
            Some(t) => t.lines(),
            None => unreachable!()
        }
    }

    fn message_block(&self) -> Block {
        let top_pad: u16 = if self.skip > 0 {
            0
        } else {
            1
        };
        match self.kind {
            MessageKind::Response => {
                let assistant_style = Style::default().bg(Color::Rgb(30, 30, 30));

                Block::default()
                    .style(assistant_style)
                    .padding(Padding::new(1, 1, top_pad, 1))
            },
            MessageKind::ResponsePlaceholder => {
                let assistant_style = Style::default().bg(Color::Rgb(30, 30, 30));
                Block::default()
                    .style(assistant_style)
                    .padding(Padding::new(1, 1, top_pad, 1))
            },
            MessageKind::User => {
                let user_style = Style::default()
                    .bg(Color::Rgb(10, 10, 10));
                Block::default()
                    .style(user_style)
                    .padding(Padding::new(1, 1, top_pad, 1))
            },
            MessageKind::Thinking => {
                let thinking_style = Style::default().fg(Color::Rgb(125, 125, 125));

                Block::default()
                    .style(thinking_style)
                    .padding(Padding::new(1, 1, top_pad, 0))
            },
            MessageKind::Error => {
                let error_style = Style::default().bg(Color::Red);

                let title = Line::from("Error").add_modifier(Modifier::BOLD);

                Block::default()
                    .style(error_style)
                    .title(title)
                    .padding(Padding::new(1, 1, 0, 1))
            },
        }
    }

}

impl<'a> Widget for &Message<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized
    {
        let content_skip = self.skip.saturating_sub(1);

        let body = match (self.kind, &self.wrapped) {
            (MessageKind::Error, _) => {
                vec!["..."]
            },
            (_, Some(w)) if w.width() == (area.width as usize).saturating_sub(PADDING) => {
                w.lines().to_vec()
            },
            _ => {
                WrappedText::new(self.content, (area.width as usize).saturating_sub(PADDING))
                    .lines()
                    .to_vec()
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
