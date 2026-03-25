use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,

    PrLoaded(Box<crate::types::PrMetadata>),
    FilesLoaded(Vec<crate::types::DiffFile>),
    CommentsLoaded(Vec<crate::types::ExistingComment>),
    FileContentLoaded {
        path: String,
        base_content: String,
        head_content: String,
    },
    ReviewSubmitted,
    Error(String),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    tx: mpsc::UnboundedSender<AppEvent>,
    cancel: CancellationToken,
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let term_tx = tx.clone();
        let term_cancel = cancel.clone();
        std::thread::spawn(move || {
            while !term_cancel.is_cancelled() {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                            if term_tx.send(AppEvent::Key(key)).is_err() {
                                break;
                            }
                        }
                        Ok(Event::Resize(w, h)) => {
                            let _ = term_tx.send(AppEvent::Resize(w, h));
                        }
                        _ => {}
                    }
                }
            }
        });

        let tick_tx = tx.clone();
        let tick_cancel = cancel.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(200));
            loop {
                tokio::select! {
                    _ = tick_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        if tick_tx.send(AppEvent::Tick).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self { rx, tx, cancel }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.tx.clone()
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }

    pub fn stop(&self) {
        self.cancel.cancel();
    }
}
