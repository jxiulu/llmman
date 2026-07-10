use ratatui::{
    prelude::{
        Buffer, Rect
    },
    style::{
        Color, Style, Stylize
    }, 
    widgets::{
        Block, Padding, Widget
    }
};
use tui_textarea::{
    CursorMove, Input, TextArea, WrapMode
};

/// state-tracked
#[derive(Clone)]
pub struct InputBox {
    ta: TextArea<'static>,
}

impl InputBox {
    pub fn input(&mut self, input: impl Into<Input>) -> bool {
        self.ta.input(input)
    }

    pub fn lines(&self) -> &[String] {
        self.ta.lines()
    }

    pub fn clear(&mut self) {
        self.ta.clear();
    }

    pub fn total_widget_height(&self, w: usize) -> usize {
        textwrap::wrap(&self.lines().join("\n"), w).len().saturating_add(2)
    }

    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.split('\n').map(str::to_owned).collect()
        };

        let mut ta = TextArea::from(lines);
        ta.set_wrap_mode(WrapMode::WordOrGlyph);
        ta.move_cursor(CursorMove::Bottom);
        ta.move_cursor(CursorMove::End);

        let block = Block::default()
            .padding(Padding::new(1, 1, 0, 1))
            .title(" >>>")
            .bg(Color::Black);

        let cursor_style = Style::default()
            .bg(Color::Blue);

        ta.set_block(block);
        ta.set_cursor_style(cursor_style);

        Self {
            ta
        }
    }
}

impl Widget for &InputBox {
    fn render(
        self, area: Rect, buf: &mut Buffer
    ) where
        Self: Sized
    {
        self.ta.render(area, buf);
    }
}
