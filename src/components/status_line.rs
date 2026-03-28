use std::time::{Duration, Instant};

const AUTO_CLEAR_AFTER: Duration = Duration::from_secs(2);

/// Single-line status bar message state with auto-expiry.
pub struct StatusLine {
    text: String,
    is_error: bool,
    set_at: Option<Instant>,
}

impl StatusLine {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            is_error: false,
            set_at: None,
        }
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.text = msg.into();
        self.is_error = false;
        self.set_at = Some(Instant::now());
    }

    pub fn success(&mut self, msg: impl Into<String>) {
        self.text = format!("✓ {}", msg.into());
        self.is_error = false;
        self.set_at = Some(Instant::now());
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.text = msg.into();
        self.is_error = true;
        self.set_at = Some(Instant::now());
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.is_error = false;
        self.set_at = None;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_error(&self) -> bool {
        self.is_error
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Returns true if the message has expired and was cleared.
    /// Call this during each draw cycle to auto-dismiss stale messages.
    pub fn tick(&mut self) -> bool {
        if let Some(at) = self.set_at
            && at.elapsed() >= AUTO_CLEAR_AFTER
        {
            self.clear();
            return true;
        }
        false
    }
}
