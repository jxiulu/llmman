use std::{
    io::{
        self, Stdout
    },
    time::Duration
};

use futures::{
    StreamExt, stream
};
use tokio::{
    sync::mpsc, time
};
use tokio_stream::wrappers::{
    IntervalStream, UnboundedReceiverStream
};

use crossterm::{
    cursor,
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream,
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind
    },
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen}
};
use ratatui::{
    Terminal,
    backend::Backend,
};
use color_eyre as eyre;

use tracing_appender::{
    self as ta,
    non_blocking::WorkerGuard
};
use tracing_subscriber as ts;

use crate::{
    AppData, app_data::{self, config}, cli::{
        self, ViewState, widgets::scroll::ScrollOpt
    }, llm::{
        self, StreamingEndpoint
    }, model::{
        self, Model, StreamingState
    }
};

const MODEL: &str = "glm-5.2";

pub fn setup() -> eyre::Result<()> {
    eyre::install()?;
    dotenvy::dotenv().ok();

    Ok(())
}

pub fn setup_tracing_subscriber() -> WorkerGuard {
    let path = app_data::dirs().saves();

    let file_appender = ta::rolling::never(path, "setboy.log");
    let (non_blocking, guard) = ta::non_blocking(file_appender);
    ts::fmt()
        .with_writer(non_blocking)
        .init();

    guard
}

pub fn ready(out: &mut impl io::Write) -> eyre::Result<()> {
    terminal::enable_raw_mode()?;
    print!("\x1b]11;#{:02x}{:02x}{:02x}\x1b\\", 30, 30, 30);
    crossterm::execute!(out, EnterAlternateScreen, EnableMouseCapture)?;

    Ok(())
}

pub fn unready(out: &mut impl io::Write) -> eyre::Result<()> {
    terminal::disable_raw_mode()?;
    print!("\x1b]111\x1b\\");
    crossterm::execute!(
        out,
        LeaveAlternateScreen, DisableMouseCapture, cursor::Show
    )?;

    Ok(())
}

pub enum CliEvent {
    Term(crossterm::event::Event),
    Llm(llm::Event),
    Tick,
}

pub struct App<B: Backend> {
    appdata: AppData,
    model: Model,
    view: ViewState,
    terminal: Terminal<B>
}

impl<B: Backend + io::Write> App<B>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    pub fn new(backend: B, appdata: AppData) -> eyre::Result<Self> {
        let t = Terminal::new(backend)?;

        Ok(Self {
            appdata,
            model: Model::new(),
            view: ViewState::new(),
            terminal: t
        })
    }

    pub fn ready_terminal(&mut self) -> eyre::Result<()> {
        ready(self.terminal.backend_mut())?;

        Ok(())
    }

    pub fn unready_terminal(&mut self) -> eyre::Result<()> {
        unready(self.terminal.backend_mut())?;

        Ok(())
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let (tx, rx) = mpsc::unbounded_channel::<llm::Event>();
        let tick = time::interval(Duration::from_millis(100));

        let ec = StreamingEndpoint::new(tx);

        let term_stream = EventStream::new()
            .filter_map(|e| async { e.ok().map(CliEvent::Term) });
        let llm_stream = UnboundedReceiverStream::new(rx)
            .map(CliEvent::Llm);
        let tick_stream = IntervalStream::new(tick)
            .map(|_| CliEvent::Tick);

        let app_events = stream::select(
            stream::select(term_stream, llm_stream),
            tick_stream,
        );
        tokio::pin!(app_events);

        loop {
            cli::sync(&mut self.view, &mut self.model);
            self.terminal.draw(|f| cli::draw(&mut self.view, &self.model, f))?;

            if let Some(event) = app_events.next().await {
                match event {
                    CliEvent::Term(e) => self.handle_term_event(e, &ec),
                    CliEvent::Llm(e) => Self::handle_llm_event(&mut self.model, e),
                    CliEvent::Tick => {
                        if matches!(
                            self.model.streaming_state(),
                            StreamingState::AwaitingStream | StreamingState::Streaming
                        ) {
                            self.model.spinner_index = self.model.spinner_index.wrapping_add(1);
                        }
                    }
                }
            }

            if self.model.should_quit() {
                break;
            }
        }

        Ok(())
    }

    fn handle_term_event(&mut self, event: Event, ec: &StreamingEndpoint) {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.handle_key(key, ec);
            }
            Event::Mouse(mouse) => Self::handle_mouse(&mut self.view, mouse),
            _ => {}
        }
    }

    fn handle_llm_event(model: &mut Model, event: llm::Event) {
        match event {
            llm::Event::Token(token) => model.push_stream_token(&token),
            llm::Event::Done => model.finish_stream(),
            llm::Event::Error(e) => model.error_stream(&e),
            llm::Event::ThinkingToken(token) => model.push_think_token(&token),
        }
    }

    fn handle_mouse(view: &mut ViewState, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => view.chat.scroll_down(3),
            MouseEventKind::ScrollUp => view.chat.scroll_up(3),
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent, ec: &StreamingEndpoint) {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.model.increment_quit();
                self.model.right_status = Some("press ctrl+c again to quit".to_string());
                return;
            }
            _ => {
                self.model.cancel_quit();
                self.model.right_status = None;
            }
        }

        if self.model.streaming_state() == model::StreamingState::Streaming
            || self.model.streaming_state() == model::StreamingState::AwaitingStream
        {
            match key.code {
                KeyCode::PageUp => self.view.chat.scroll_up(3),
                KeyCode::PageDown => self.view.chat.scroll_down(3),
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.model.focus_on_input();
            }
            KeyCode::Tab => {
                self.model.focus_up(1);
                self.view.chat.scroll = ScrollOpt::Focus;
            }
            KeyCode::BackTab => {
                self.model.focus_down(1);
                self.view.chat.scroll = ScrollOpt::Focus;
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
                if matches!(self.model.focus(), model::Focus::Input) {
                    self.view.submit_input(&mut self.model);
                    if self.model.streaming_state() == model::StreamingState::AwaitingStream {
                        let messages = llm::build_context(&self.model);
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
                        self.view.chat.scroll = ScrollOpt::Max;
                    }
                }
            }
            KeyCode::PageUp => self.view.chat.scroll_up(3),
            KeyCode::PageDown => self.view.chat.scroll_down(3),
            _ => {
                self.view.chat.input(key);
            }
        }
    }
}
