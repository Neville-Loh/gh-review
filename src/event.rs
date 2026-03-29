use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,

    PrLoaded { pr: u64, data: Box<crate::types::PrMetadata> },
    FilesLoaded { pr: u64, data: Vec<crate::types::DiffFile> },
    CommentsLoaded { pr: u64, data: Vec<crate::types::ExistingComment> },
    FileContentLoaded {
        path: String,
        base_content: String,
        head_content: String,
    },
    ThreadsLoaded { pr: u64, data: std::collections::HashMap<u64, crate::types::ThreadInfo> },
    ThreadResolveToggled,
    ReviewDismissed,
    SuggestionAccepted,
    ReviewSubmitted,
    StackTitlesLoaded(Vec<(u64, String)>),
    StackPrefetchLoaded(std::collections::HashMap<u64, crate::stack::PrSnapshot>),
    CustomActionComplete {
        description: String,
        result: Result<(), String>,
    },
    Error(String),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    tx: mpsc::UnboundedSender<AppEvent>,
    cancel: CancellationToken,
    paused: Arc<AtomicBool>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let paused = Arc::new(AtomicBool::new(false));

        let term_tx = tx.clone();
        let term_cancel = cancel.clone();
        let term_paused = paused.clone();
        std::thread::spawn(move || {
            while !term_cancel.is_cancelled() {
                if term_paused.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    if term_paused.load(Ordering::Relaxed) {
                        continue;
                    }
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

        Self {
            rx,
            tx,
            cancel,
            paused,
        }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.tx.clone()
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.cancel.cancel();
    }
}
