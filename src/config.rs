pub struct Config {
    pub smooth_scroll: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            smooth_scroll: true,
        }
    }
}
