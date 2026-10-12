use std::io::{self, IsTerminal, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Progress {
    stop: Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl Progress {
    pub fn start() -> Self {
        let (stop, stopped) = mpsc::channel();
        let enabled = io::stdout().is_terminal()
            && io::stderr().is_terminal()
            && std::env::var("TERM").as_deref() != Ok("dumb");
        let worker = enabled.then(|| {
            // Stderr locks only for each write, never while waiting. Holding
            // its lock across waits could block an inspection's panic hook.
            thread::spawn(move || animate(stopped, &mut io::stderr()))
        });
        Self { stop, worker }
    }
}

fn animate(stopped: Receiver<()>, out: &mut impl Write) {
    // A quick run finishes before any animation appears.
    if stopped.recv_timeout(Duration::from_millis(150)) != Err(RecvTimeoutError::Timeout) {
        return;
    }
    for frame in ['|', '/', '-', '\\'].into_iter().cycle() {
        let text = format!("\r{frame} Inspecting repositories…");
        if out
            .write_all(text.as_bytes())
            .and_then(|()| out.flush())
            .is_err()
        {
            break;
        }
        if stopped.recv_timeout(Duration::from_millis(100)) != Err(RecvTimeoutError::Timeout) {
            break;
        }
    }
    let _ = out.write_all(b"\r\x1b[2K").and_then(|()| out.flush());
}

impl Drop for Progress {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_completion_has_no_animation() {
        let (stop, stopped) = mpsc::channel();
        stop.send(())
            .expect("stop signal should reach the animation");
        let mut output = Vec::new();
        animate(stopped, &mut output);
        assert!(output.is_empty());
    }

    #[test]
    fn disconnected_sender_stops_and_clears_a_running_animation() {
        struct NotifyingWriter {
            bytes: Vec<u8>,
            first: Option<Sender<()>>,
        }
        impl Write for NotifyingWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.bytes.extend_from_slice(bytes);
                if let Some(first) = self.first.take() {
                    let _ = first.send(());
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let (stop, stopped) = mpsc::channel();
        let (first, started) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut output = NotifyingWriter {
                bytes: Vec::new(),
                first: Some(first),
            };
            animate(stopped, &mut output);
            output.bytes
        });
        started
            .recv_timeout(Duration::from_secs(5))
            .expect("animation should start before the timeout");
        drop(stop);
        let output = worker
            .join()
            .expect("animation worker should finish without panicking");
        assert!(String::from_utf8_lossy(&output).contains("Inspecting repositories"));
        assert!(output.ends_with(b"\r\x1b[2K"));
    }
}
