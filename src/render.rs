mod fetch;
mod status;
mod text;

pub use fetch::{FetchProgress, fetch_result};
pub use status::{StatusFilter, status_progress, table};

use std::io::IsTerminal;
use unicode_width::UnicodeWidthStr;

pub fn color_enabled(stream: &impl IsTerminal) -> bool {
    stream.is_terminal()
        && std::env::var("TERM").as_deref() != Ok("dumb")
        && std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
}

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";

struct TableCell {
    text: String,
    tone: &'static str,
}

impl TableCell {
    fn new(text: String, tone: &'static str) -> Self {
        Self { text, tone }
    }

    fn write(&self, line: &mut String, color: bool, width: Option<usize>) {
        if color && !self.tone.is_empty() {
            line.push_str(self.tone);
            line.push_str(&self.text);
            line.push_str("\x1b[0m");
        } else {
            line.push_str(&self.text);
        }
        if let Some(width) = width {
            line.push_str(&" ".repeat(width - self.text.width() + 1));
        }
    }
}
