/// Single-line status bar message state.
pub struct StatusLine {
    text: String,
    is_error: bool,
}

impl StatusLine {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            is_error: false,
        }
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.text = msg.into();
        self.is_error = false;
    }

    pub fn success(&mut self, msg: impl Into<String>) {
        self.text = format!("✓ {}", msg.into());
        self.is_error = false;
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.text = msg.into();
        self.is_error = true;
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.is_error = false;
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
}
