use std::mem;

use ratatui::{
    Frame,
    layout::{
        Constraint, Direction, Layout, Rect
    },
    style::{
        Color, Modifier, Style, Stylize
    },
    text::{
        Line, Text
    },
    widgets::{
        Block, Borders, Padding, Paragraph
    },
};
use tui_textarea::{TextArea, WrapMode};

use crate::model::{
    self,
    MessageKind, Model
};

const SPINNER_GLYPHS: [char; 4] = ['|', '/', '-', '\\'];

pub fn draw(f: &mut Frame, model: &mut Model, textarea: &TextArea<'_>) {
    let total_area = f.area();

    let default_style = Style::default()
        .bg(Color::Rgb(30, 30, 30))
        .fg(Color::Rgb(210, 220, 230));

    let bg = Block::default()
        .style(default_style);

    f.render_widget(bg, total_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
        ])
        .split(total_area);

    draw_status(f, model, layout[0]);
    draw_chat(f, model, textarea, layout[1]);
}

fn draw_status(f: &mut Frame, model: &Model, area: Rect) {
    let spinner_glyph = if model.is_streaming {
        SPINNER_GLYPHS[model.spinner_index % SPINNER_GLYPHS.len()]
    } else {
        ' '
    };

    let text = format!("{} {}", model.status, spinner_glyph);

    let aux_status_text = model.ui_status.clone()
        .unwrap_or_default();

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Max(aux_status_text.len() as u16)
        ])
        .split(area);

    let status_style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::ITALIC);

    let block = Block::new()
        .padding(Padding::new(0, 0, 0, 1))
        .style(status_style);

    let status_widget = Paragraph::new(text)
        .fg(Color::DarkGray)
        .add_modifier(Modifier::ITALIC)
        .block(block.clone());

    let aux_status_widget = Paragraph::new(aux_status_text)
        .block(block);

    f.render_widget(status_widget, layout[0]);
    f.render_widget(aux_status_widget, layout[1]);
}

fn draw_chat(f: &mut Frame, model: &mut Model, textarea: &TextArea<'_>, area: Rect) {
    let inner_width = area.width.saturating_sub(2).max(1) as usize;
    let chat_len = model.chat.len();

    let focused_on_input = model.focus == Some(chat_len);
    let input_height: u16 = if focused_on_input {
        (textarea.lines().len() as u16 + 2).max(3)
    } else {
        3
    };

    let all_heights: Vec<u16> = model.chat.iter()
        .enumerate()
        .map(|(i, m)| {
            if model.focus == Some(i) {
                let lines = textarea.lines().join("\n");
                let textarea_height = wrap_text(&lines, inner_width).len();

                (textarea_height as u16 + 2).max(3)
            } else {
                height_of_msg(m, inner_width)
            }
        })
        .chain(std::iter::once(input_height))
        .collect();

    let total_height: u16 = all_heights.iter().sum();
    let max_scroll = total_height.saturating_sub(area.height);

    // enable follow when scrolling below bottom
    if !model.should_follow && model.scroll > max_scroll {
        model.should_follow = true;
    }

    let focused_snap_scroll = model.focus.map(|idx| {
        let focused_offset: u16 = all_heights[..idx].iter().sum();
        let focused_height = all_heights[idx];
        let s = if focused_height <= area.height {
            (focused_offset + focused_height).saturating_sub(area.height)
        } else {
            focused_offset
        };
        s.min(max_scroll)
    });

    let scroll_offset = if model.should_snap {
        tracing::info!("view snapped to offset {}", focused_snap_scroll.unwrap_or(max_scroll));

        model.should_snap = false;
        focused_snap_scroll.unwrap_or(max_scroll)
    } else if model.should_follow {
        tracing::info!("view following");

        max_scroll
    } else {
        model.scroll.min(max_scroll)
    };

    model.scroll = scroll_offset;

    let mut y = area.y;
    let bottom = area.y + area.height;
    let mut remaining_skip = scroll_offset;

    let msg_height_enumerated = model.chat.iter()
        .zip(all_heights.iter())
        .enumerate();

    for (i, (msg, &height)) in msg_height_enumerated {
        if remaining_skip >= height {
            remaining_skip -= height;
            continue;
        }
        if y >= bottom {
            break;
        }

        let line_skip = remaining_skip;
        remaining_skip = 0;

        let rendered_height = (height - line_skip).min(bottom - y);
        let msg_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height: rendered_height,
        };

        if model.focus == Some(i) {
            f.render_widget(textarea, msg_area);
        } else {
            draw_message(f, msg, model, msg_area, line_skip);
        }

        y += rendered_height;
    }

    if y < bottom {
        let &input_h = all_heights.last().unwrap();
        if remaining_skip >= input_h {
            return;
        }

        let line_skip = remaining_skip;
        let rendered_height = (input_h - line_skip).min(bottom - y);
        let input_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height: rendered_height,
        };

        if focused_on_input {
            f.render_widget(textarea, input_area);
        } else {
            draw_input_placeholder(f, model, input_area);
        }
    }
}

fn height_of_msg(m: &model::Message, width: usize) -> u16 {
    let body_lines = wrap_text(&m.content, width).len();

    body_lines as u16 + 2
}

fn draw_message(f: &mut Frame, msg: &model::Message, model: &Model, area: Rect, line_skip: u16) {
    // line_skip=0: render top padding + content + bottom padding
    // line_skip=1: top padding consumed, content_scroll=0
    // line_skip=2+: top padding gone, content scrolled by (line_skip-1)
    let top_pad: u16 = if line_skip > 0 { 0 } else { 1 };

    let content_scroll = line_skip.saturating_sub(1);
    let inner_width = area.width.saturating_sub(2) as usize;

    let (block, body) = match msg.kind {
        MessageKind::Response => {
            let assistant_style = Style::default().bg(Color::Rgb(30, 30, 30));
            let block = Block::default()
                .style(assistant_style)
                .padding(Padding::new(1, 1, top_pad, 1));

            let body = if msg.content.is_empty() && model.is_streaming {
                "..."
            } else {
                msg.content.as_str()
            };

            (block, body)
        },
        MessageKind::User => {
            let user_style = Style::default()
                .bg(Color::Rgb(10, 10, 10));
            let block = Block::default()
                .style(user_style)
                .padding(Padding::new(1, 1, top_pad, 1));

            (block, msg.content.as_str())
        },
        MessageKind::Error => {
            let error_style = Style::default().bg(Color::Red);

            let title = Line::from("Error").add_modifier(Modifier::BOLD);

            let block = Block::default()
                .style(error_style)
                .title(title)
                .padding(Padding::new(1, 1, 0, 1));

            (block, msg.content.as_str())
        },
        MessageKind::Thinking => {
            let thinking_style = Style::default().fg(Color::Rgb(125, 125, 125));

            let block = Block::default()
                .style(thinking_style)
                .padding(Padding::new(1, 1, top_pad, 0));

            (block, msg.content.as_str())
        },
    };

    let lines: Vec<Line> = wrap_text(body, inner_width)
        .into_iter()
        .map(Line::raw)
        .collect();

    let p = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((content_scroll, 0));

    f.render_widget(p, area);
}

fn draw_input_placeholder(f: &mut Frame, model: &Model, area: Rect) {
    let text = if model.is_streaming { "..." } else { "" };

    let block = Block::default()
        .padding(Padding::new(1, 1, 0, 1))
        .title(" >>>")
        .bg(Color::Black);

    let p = Paragraph::new(text)
        .fg(Color::DarkGray)
        .block(block);

    f.render_widget(p, area);
}

pub fn new_input_textarea<'a>(content: &str) -> TextArea<'a> {
    let lines: Vec<String> = if content.is_empty() {
        vec![String::new()]
    } else {
        content.split('\n').map(str::to_string).collect()
    };

    let mut ta = TextArea::from(lines);
    ta.set_wrap_mode(WrapMode::WordOrGlyph);

    let block = Block::default()
        .padding(Padding::new(1, 1, 0, 1))
        .title(" >>>")
        .bg(Color::Black);

    // ta.set_style(Style::default().fg(Color::White));
    ta.set_block(block);
    ta.set_cursor_line_style(Style::default());

    ta
}

pub fn new_editing_textarea<'a>(content: &str) -> TextArea<'a> {
    let lines: Vec<String> = if content.is_empty() {
        vec![String::new()]
    } else {
        content.split('\n').map(str::to_string).collect()
    };

    let mut ta = TextArea::from(lines);
    ta.set_wrap_mode(WrapMode::WordOrGlyph);

    let edit_textarea_style = Style::default()
        .bg(Color::Rgb(80, 90, 100));

    let block = Block::default()
        .style(edit_textarea_style)
        .padding(Padding::new(1, 1, 1, 1));

    // ta.set_style(Style::default().fg(Color::White));
    ta.set_block(block);
    ta.set_cursor_line_style(Style::default());

    ta
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut rows = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            rows.push(String::new());
            continue;
        }

        let mut row = String::new();
        for mut word in line.split(' ') {
            // if the word is longer than the width, we'll split the word aggressively
            while word.chars().count() > width {
                let split_index = word
                    .char_indices()
                    .nth(width)
                    .map(|(i, _)| i)
                    .unwrap_or(word.len());

                // empty the row first
                if !row.is_empty() {
                    rows.push(mem::take(&mut row));
                }

                let (head, tail) = word.split_at(split_index);
                rows.push(head.to_string());
                word = tail;
            }

            // account for the space before a new word
            let extra = if row.is_empty() { 0 } else { 1 };

            // flush current row if too wide
            let combined_width = row.chars().count() + extra + word.chars().count();
            if combined_width > width {
                rows.push(mem::take(&mut row));
            }

            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(word);
        }

        rows.push(row);
    }

    rows
}
