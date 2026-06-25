mod llm;
mod model;
mod cli;

use color_eyre as eyre;
use ratatui::{
    Terminal,
    backend::{
        Backend, CrosstermBackend
    }
};
use tokio::{
    sync::mpsc,
    time
};
use std::{
    io,
    time::Duration
};
use futures::StreamExt;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent,
        KeyEventKind, KeyModifiers, MouseEvent
    },
    terminal::{
        self, EnterAlternateScreen, LeaveAlternateScreen
    }
};
use tui_textarea::TextArea;
use tracing_appender as ta;
use tracing_subscriber as ts;

use crate::{
    llm::EndpointController,
    model::Model
};

const MODEL: &str = "glm-5.2";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let current_dir = std::env::current_dir().unwrap();
    let file_appender = ta::rolling::never(current_dir, "setboy.log");
    let (non_blocking, _guard) = ta::non_blocking(file_appender);
    ts::fmt()
        .with_writer(non_blocking)
        .init();

    eyre::install()?;
    dotenvy::dotenv().ok();

    let mut out = io::stdout();

    terminal::enable_raw_mode()?;
    print!("\x1b]11;#{:02x}{:02x}{:02x}\x1b\\", 30, 30, 30);
    crossterm::execute!(
        out,
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(out);
    let mut t = Terminal::new(backend)?;

    let result = run(&mut t).await;

    terminal::disable_raw_mode()?;
    print!("\x1b]111\x1b\\");
    crossterm::execute!(
        t.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    t.show_cursor()?;

    result
}

async fn run<B: Backend>(t: &mut Terminal<B>) -> eyre::Result<()>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let (tx, mut rx) = mpsc::unbounded_channel::<llm::Event>();
    let mut tick = time::interval(Duration::from_millis(500));
    let mut events = EventStream::new();

    let mut model = Model::new();
    let ec = EndpointController::new(tx);

    let mut input_box = cli::new_input_textarea("");

    loop {
        t.draw(|f| cli::draw(f, &mut model, &input_box))?;

        tokio::select! {
            term_event = events.next() => {
                match term_event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        handle_key(&mut model, &mut input_box, key, &ec);   
                    },
                    Some(Ok(Event::Mouse(mouse))) => {
                        handle_mouse(&mut model, &mut input_box, mouse);
                    },
                    _ => {
                    }
                }
            },

            Some(event) = rx.recv() => match event {
                llm::Event::Token(t) => {
                    model.push_stream_token(&t)
                },
                llm::Event::Done => {
                    model.finish_stream();
                },
                llm::Event::Error(e) => {
                    model.error_stream(&e)
                },
                llm::Event::ThinkingToken(t) => {
                    model.push_think_token(&t)
                },
            },

            _ = tick.tick() => {
                if model.is_streaming {
                    model.spinner_index = model.spinner_index.wrapping_add(1);
                }
            }
        }

        if model.should_quit() {
            break;
        }
    }

    Ok(())
}

fn handle_mouse(
    model: &mut Model,
    textarea: &mut TextArea<'_>,
    mouse: MouseEvent,
) {
    match mouse.kind {
        crossterm::event::MouseEventKind::ScrollDown => {
            model.scroll_down();
        },
        crossterm::event::MouseEventKind::ScrollUp => {
            model.scroll_up();
        },
        _ => {
        }
    }
}

fn handle_key(
    model: &mut Model,
    textarea: &mut TextArea<'_>,
    key: KeyEvent,
    ec: &EndpointController,
) {
    // During streaming, only scrolling and quit are allowed.
    if model.is_streaming {
        match key.code {
            KeyCode::PageUp => model.scroll_up(),
            KeyCode::PageDown => model.scroll_down(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                model.increment_quit();
                model.set_aux_status(Some("press esc again to quit"));
                return;
            },
            _ => {},
        }
        model.cancel_quit();
        model.set_aux_status(None);
        return;
    }

    let input_slot = model.chat.len();

    match key.code {
        KeyCode::Esc => {
            model.unfocus();
        },
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            model.increment_quit();
            model.set_aux_status(Some("press esc again to quit"));
        },
        KeyCode::Tab => {
            // up (smaller index)
            navigate_focus(model, textarea, -1);
        },
        KeyCode::BackTab => {
            navigate_focus(model, textarea, 1);
        },
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
            if model.focus.is_none_or(|i| i == input_slot) {
                model.focus_on(input_slot);
                let text = textarea.lines().join("\n");
                if model.flush_input(text) {
                    *textarea = cli::new_input_textarea("");

                    let last = model.chat.len() - 1;
                    let messages: Vec<llm::Message> = model.chat[..last]
                        .iter()
                        .filter_map(llm_message)
                        .collect();

                    let request = llm::Request {
                        model: MODEL.to_string(),
                        temp: None,
                        top_p: None,
                        top_k: None,
                        min_p: None,
                        messages
                    };

                    let mut async_ec = ec.clone();
                    tokio::spawn(async move {
                        async_ec.stream_request(&request).await
                    });
                }
            } else {
                model.snap();
                textarea.insert_newline();
            }
        },
        KeyCode::Enter => {
            if model.focus.is_none() {
                model.focus_on(input_slot);
            }
            model.snap();
            textarea.insert_newline();
        },
        KeyCode::PageUp => {
            model.scroll_up();
        },
        KeyCode::PageDown => {
            model.scroll_down();
        },
        _ => {
            if model.focus.is_none() {
                *textarea = cli::new_input_textarea("");
                model.focus_on(input_slot);
            } else {
                model.snap();
            }
            textarea.input(key);
        },
    }

    // Sync textarea content back to the focused slot.
    if let Some(idx) = model.focus {
        let content = textarea.lines().join("\n");
        if idx < model.chat.len() {
            model.chat[idx].content = content;
        } else {
            model.input_buffer = content;
        }
    }

    if key.code != KeyCode::Char('c')
    || !key.modifiers.contains(KeyModifiers::CONTROL) {
        model.cancel_quit();
        model.set_aux_status(None);
    }
}

fn navigate_focus(model: &mut Model, textarea: &mut TextArea<'_>, dir: i32) {
    let focusable = model.focusable_indices();
    if focusable.is_empty() {
        return;
    }

    let current_item = model.focus
        .and_then(|f| focusable.iter().position(|&i| i == f));

    let next_item = match current_item {
        None => {
            if dir > 0 { 0 } else { focusable.len() - 1 }
        },
        Some(p) => {
            let len = focusable.len();
            ((p as i32 + dir).rem_euclid(len as i32)) as usize
        },
    };

    let next_idx = focusable[next_item];

    let content = content_for_slot(model, next_idx);
    *textarea = if next_idx == model.chat.len() {
        cli::new_input_textarea(&content) 
    } else {
        cli::new_editing_textarea(&content) 
    };

    model.focus_on(next_idx);
}

fn content_for_slot(model: &Model, idx: usize) -> String {
    if idx < model.chat.len() {
        model.chat[idx].content.clone()
    } else {
        model.input_buffer.clone()
    }
}

fn llm_message(m: &model::Message) -> Option<llm::Message> {
    match m.kind {
        model::MessageKind::Response => {
            Some(llm::Message::assistant(&m.content))
        },
        model::MessageKind::User => {
            Some(llm::Message::user(&m.content))
        },
        _ => {
            None
        }
    }
}
