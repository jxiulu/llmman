mod llm;
mod wrapped_text;
mod widgets;
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
    io, time::Duration
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
use tracing_appender as ta;
use tracing_subscriber as ts;

use crate::{
    llm::EndpointController,
    widgets::Scroll,
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
    let mut tick = time::interval(Duration::from_millis(100));
    let mut events = EventStream::new();

    let mut model = model::Model::new();
    let ec = EndpointController::new(tx);
    let mut view = cli::ViewState::new();

    loop {
        view.sync_with(&mut model);
        t.draw(|f| cli::draw(&mut view, &model, f))?;

        tokio::select! {
            term_event = events.next() => {
                match term_event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        handle_key(&mut model, &mut view, key, &ec);
                    },
                    Some(Ok(Event::Mouse(mouse))) => {
                        handle_mouse(&mut view, mouse);
                    },
                    _ => {}
                }
            },

            Some(event) = rx.recv() => match event {
                llm::Event::Token(token) => model.push_stream_token(&token),
                llm::Event::Done => model.finish_stream(),
                llm::Event::Error(e) => model.error_stream(&e),
                llm::Event::ThinkingToken(token) => model.push_think_token(&token),
            },

            _ = tick.tick() => {
                if model.streaming_state() == model::StreamingState::Streaming {
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

fn handle_mouse(view: &mut cli::ViewState, mouse: MouseEvent) {
    match mouse.kind {
        crossterm::event::MouseEventKind::ScrollDown => view.chat.scroll_down(3),
        crossterm::event::MouseEventKind::ScrollUp => view.chat.scroll_up(3),
        _ => {}
    }
}

fn handle_key(
    model: &mut model::Model,
    view: &mut cli::ViewState,
    key: KeyEvent,
    ec: &EndpointController,
) {
    if model.streaming_state() == model::StreamingState::Streaming
        || model.streaming_state() == model::StreamingState::AwaitingStream
    {
        match key.code {
            KeyCode::PageUp => view.chat.scroll_up(3),
            KeyCode::PageDown => view.chat.scroll_down(3),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                model.increment_quit();
                model.right_status = Some("press ctrl+c again to quit".to_string());
                return;
            },
            _ => {},
        }
        model.cancel_quit();
        model.right_status = None;
        return;
    }

    match key.code {
        KeyCode::Esc => {
            model.focus_on_input();
        },
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            model.increment_quit();
            model.right_status = Some("press ctrl+c again to quit".to_string());
            return;
        },
        KeyCode::Tab => {
            model.focus_up(1);
            view.chat.scroll = Scroll::Focus;
        },
        KeyCode::BackTab => {
            model.focus_down(1);
            view.chat.scroll = Scroll::Focus;
        },
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
            if matches!(model.focus_state(), model::FocusState::Input) {
                view.submit_input(model);
                if model.streaming_state() == model::StreamingState::AwaitingStream {
                    let messages = llm::build_context(model);
                    let request = llm::Request {
                        model: MODEL.to_string(),
                        temp: None,
                        top_p: None,
                        top_k: None,
                        min_p: None,
                        messages,
                    };
                    let mut async_ec = ec.clone();
                    tokio::spawn(async move {
                        async_ec.stream_request(&request).await;
                    });
                    view.chat.scroll = Scroll::Max;
                }
            }
        },
        KeyCode::PageUp => view.chat.scroll_up(3),
        KeyCode::PageDown => view.chat.scroll_down(3),
        _ => {
            view.chat.input(key);
        }
    }

    model.cancel_quit();
    model.right_status = None;
}
