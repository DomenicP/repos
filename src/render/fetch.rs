use super::{
    DIM, GREEN, RED, color_enabled,
    text::{escape, path_text},
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct FetchProgress {
    display: MultiProgress,
    bars: BTreeMap<PathBuf, ProgressBar>,
    color: bool,
}

impl FetchProgress {
    pub fn new(root: &Path, paths: &[PathBuf]) -> Self {
        let display = MultiProgress::new();
        let color = color_enabled(&io::stderr());
        let mut bars = BTreeMap::new();
        if !display.is_hidden() {
            let template = if color {
                "{spinner:.cyan} {prefix:.bold} {wide_msg}"
            } else {
                "{spinner} {prefix} {wide_msg}"
            };
            let style =
                ProgressStyle::with_template(template).expect("fetch progress template is valid");
            for path in paths {
                let relative = path.strip_prefix(root).unwrap_or(path);
                let name = if relative.as_os_str().is_empty() {
                    ".".into()
                } else {
                    path_text(relative)
                };
                let bar = display.add(ProgressBar::new_spinner());
                bar.set_style(style.clone());
                bar.set_prefix(name);
                bar.set_message("queued");
                bars.insert(path.clone(), bar);
            }
        }
        Self {
            display,
            bars,
            color,
        }
    }

    pub fn update(&self, path: &Path, remote: &str, message: &str) {
        if let Some(bar) = self.bars.get(path) {
            if message == "fetching" {
                bar.enable_steady_tick(Duration::from_millis(100));
            }
            bar.set_message(format!("[{}]: {message}", escape(remote)));
        }
    }

    pub fn finish(&self, path: &Path, count: usize, failed: bool) {
        let message = if failed {
            format!("failed ({count} remotes fetched)")
        } else if count == 0 {
            "no remotes".into()
        } else {
            format!("fetched {count} remotes")
        };
        self.finish_message(path, &message, failed);
    }

    pub fn finish_message(&self, path: &Path, message: &str, failed: bool) {
        if let Some(bar) = self.bars.get(path) {
            let template = match (self.color, failed) {
                (true, true) => "✗ {prefix:.bold} {msg:.red}",
                (true, false) => "✓ {prefix:.bold} {msg:.green}",
                (false, true) => "✗ {prefix} {msg}",
                (false, false) => "✓ {prefix} {msg}",
            };
            bar.set_style(
                ProgressStyle::with_template(template).expect("finished fetch template is valid"),
            );
            bar.finish_with_message(message.to_owned());
        }
    }

    pub fn clear(self) -> io::Result<()> {
        self.display.clear()
    }
}

pub fn fetch_result(
    out: &mut impl Write,
    root: &Path,
    path: &Path,
    count: usize,
    failed: bool,
    color: bool,
) -> io::Result<()> {
    let message = if failed {
        format!("fetch failed ({count} remotes fetched)")
    } else if count == 0 {
        "no remotes".into()
    } else {
        format!("fetched {count} remotes")
    };
    let (marker, tone) = if failed {
        ("✗", RED)
    } else if count == 0 {
        ("·", DIM)
    } else {
        ("✓", GREEN)
    };

    super::write_result(out, root, path, &message, (marker, tone), color)
}
