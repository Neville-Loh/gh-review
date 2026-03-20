use crossterm::event::{self, Event, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
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
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tick_tx = tx.clone();

        // Terminal event reader
        let term_tx = tx.clone();
        tokio::spawn(async move {
            loop {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(key)) => {
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

        // Tick timer
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(200));
            loop {
                interval.tick().await;
                if tick_tx.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        });

        Self { rx, tx }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.tx.clone()
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
