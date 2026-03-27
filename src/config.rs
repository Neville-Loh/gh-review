use std::io::Write;
use std::sync::Mutex;

#[allow(dead_code)]
pub struct Config {
    pub smooth_scroll: bool,
    pub debug: bool,
    log_file: Mutex<Option<std::fs::File>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            smooth_scroll: true,
            debug: false,
            log_file: Mutex::new(None),
        }
    }
}

#[allow(dead_code)]
impl Config {
    pub fn enable_debug(&mut self) {
        self.debug = true;
        let dir = crate::dirs::state_dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(file) = std::fs::File::create(dir.join("debug.log")) {
            *self.log_file.lock().unwrap() = Some(file);
        }
    }

    pub fn log(&self, msg: &str) {
        if !self.debug {
            return;
        }
        if let Ok(mut guard) = self.log_file.lock()
            && let Some(ref mut f) = *guard
        {
            let _ = writeln!(f, "{msg}");
            let _ = f.flush();
        }
    }
}

#[macro_export]
macro_rules! debug_log {
    ($config:expr, $($arg:tt)*) => {
        if $config.debug {
            $config.log(&format!($($arg)*));
        }
    };
}
