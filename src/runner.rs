//! Runs one report's command on a worker thread and reports how it ended.
//! The child runs in the app's data folder with the widened PATH, never
//! through a shell; a stop request or the time limit kills it.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::activity::Outcome;

/// Long enough for an agent that writes to Notion, short enough that a hung
/// one doesn't sit forever.
pub const TIME_LIMIT: Duration = Duration::from_secs(15 * 60);

pub struct Ended {
    pub outcome: Outcome,
    pub output: String,
}

fn reader(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

/// Blocks until the command ends; call it from a worker thread.
pub fn run(argv: &[String], stop: &Arc<AtomicBool>) -> Ended {
    let Some((program, args)) = argv.split_first() else {
        return Ended { outcome: Outcome::Failed, output: "The report has no command.".into() };
    };
    let path = crate::paths::search_path();
    let resolved = crate::paths::which(program).unwrap_or_else(|| program.into());
    let dir = crate::paths::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let spawned = Command::new(&resolved)
        .args(args)
        .env("PATH", path)
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ended {
                outcome: Outcome::Failed,
                output: format!("\"{program}\" isn't installed, or isn't on the PATH this app searches."),
            };
        }
        Err(e) => return Ended { outcome: Outcome::Failed, output: format!("Could not start {program}: {e}") },
    };
    let out = child.stdout.take().map(reader);
    let err = child.stderr.take().map(reader);
    let started = Instant::now();
    let (status, killed) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (Some(status), None),
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                return Ended { outcome: Outcome::Failed, output: format!("Lost track of {program}: {e}") };
            }
        }
        let reason = if stop.load(Ordering::Relaxed) {
            Some(Outcome::Stopped)
        } else if started.elapsed() > TIME_LIMIT {
            Some(Outcome::Failed)
        } else {
            None
        };
        if let Some(reason) = reason {
            let _ = child.kill();
            let _ = child.wait();
            break (None, Some(reason));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let stdout = out.and_then(|h| h.join().ok()).unwrap_or_default();
    let stderr = err.and_then(|h| h.join().ok()).unwrap_or_default();
    let output = strip_ansi(stdout.trim());
    match (status, killed) {
        (_, Some(Outcome::Stopped)) => Ended { outcome: Outcome::Stopped, output },
        (_, Some(_)) => Ended {
            outcome: Outcome::Failed,
            output: format!("Stopped after {} minutes without finishing.\n\n{output}", TIME_LIMIT.as_secs() / 60),
        },
        (Some(status), None) if status.success() => Ended { outcome: Outcome::Done, output },
        (status, None) => {
            let code = status.and_then(|s| s.code()).map(|c| format!(" (exit {c})")).unwrap_or_default();
            let stderr = strip_ansi(stderr.trim());
            let detail = [output.as_str(), stderr.as_str()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join("\n\n");
            // A CLI that isn't signed in often prints its login screen, which
            // reads as noise here; say what to do instead.
            let lower = detail.to_lowercase();
            if ["sign in", "log in", "logged in", "login required", "not authenticated"].iter().any(|p| lower.contains(p)) {
                return Ended {
                    outcome: Outcome::Failed,
                    output: format!("{program} isn't signed in. Open Terminal, run `{program}` and sign in, then try again."),
                };
            }
            Ended { outcome: Outcome::Failed, output: format!("{program} failed{code}.\n\n{detail}").trim().to_string() }
        }
    }
}

/// Removes terminal colour codes some CLIs print even when piped.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_finished_command_reports_its_output() {
        let stop = Arc::new(AtomicBool::new(false));
        let ended = run(&argv(&["printf", "\u{1b}[1mhello\u{1b}[0m world"]), &stop);
        assert_eq!(ended.outcome, Outcome::Done);
        assert_eq!(ended.output, "hello world");
    }

    #[test]
    fn a_failing_command_reports_stderr_and_the_exit_code() {
        let stop = Arc::new(AtomicBool::new(false));
        let ended = run(&argv(&["sh", "-c", "echo nope >&2; exit 3"]), &stop);
        assert_eq!(ended.outcome, Outcome::Failed);
        assert!(ended.output.contains("exit 3") && ended.output.contains("nope"), "{}", ended.output);
    }

    #[test]
    fn an_agent_that_isnt_signed_in_says_so() {
        let stop = Arc::new(AtomicBool::new(false));
        let ended = run(&argv(&["sh", "-c", "echo '  )){##  Press any key to sign in...'; exit 1"]), &stop);
        assert_eq!(ended.outcome, Outcome::Failed);
        assert_eq!(ended.output, "sh isn't signed in. Open Terminal, run `sh` and sign in, then try again.");
    }

    #[test]
    fn a_missing_program_is_named() {
        let stop = Arc::new(AtomicBool::new(false));
        let ended = run(&argv(&["hex-meetings-no-such-tool"]), &stop);
        assert_eq!(ended.outcome, Outcome::Failed);
        assert!(ended.output.contains("isn't installed"), "{}", ended.output);
    }

    #[test]
    fn stop_kills_the_child() {
        let stop = Arc::new(AtomicBool::new(true));
        let started = Instant::now();
        let ended = run(&argv(&["sleep", "30"]), &stop);
        assert_eq!(ended.outcome, Outcome::Stopped);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
