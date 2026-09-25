// Shared, thread-safe log buffer shown in the app's console panel, so
// script output and errors are visible inside the window.
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct Console {
    lines: Arc<Mutex<Vec<ConsoleLine>>>,
}

#[derive(Clone)]
pub struct ConsoleLine {
    pub text: String,
    pub level: Level,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Level {
    Info,  // regular print() output from the running script
    Error, // compile errors, parse errors, VM panics
}

impl Console {
    pub fn log(&self, text: impl Into<String>) {
        self.push(text, Level::Info);
    }

    pub fn error(&self, text: impl Into<String>) {
        self.push(text, Level::Error);
    }

    fn push(&self, text: impl Into<String>, level: Level) {
        self.lines.lock().unwrap().push(ConsoleLine { text: text.into(), level });
    }

    pub fn clear(&self) {
        self.lines.lock().unwrap().clear();
    }

    pub fn lines(&self) -> Vec<ConsoleLine> {
        self.lines.lock().unwrap().clone()
    }
}