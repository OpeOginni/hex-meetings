//! A record of every recipe run: which call went to which tool, when, and
//! what came back. It stays on this computer (`activity.jsonl` in the data
//! folder), so the person can always answer "where did this call go?".

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Done,
    Failed,
    Stopped,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Outcome::Done => "Done",
            Outcome::Failed => "Failed",
            Outcome::Stopped => "Stopped",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Run {
    pub at_ms: i64,
    pub meeting_id: String,
    pub meeting_title: String,
    pub recipe: String,
    pub sends_to: String,
    /// The program that ran (`opencode`), not its arguments: those hold the transcript.
    pub program: String,
    pub outcome: Outcome,
    pub output: String,
}

/// Agent output kept per run, in characters.
pub const OUTPUT_LIMIT: usize = 20_000;

pub fn append(file: &Path, run: &Run) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .map_err(|e| format!("Could not open {}: {e}", file.display()))?;
    let line = serde_json::to_string(run).map_err(|e| e.to_string())?;
    writeln!(f, "{line}").map_err(|e| e.to_string())
}

/// Every readable run, newest first. Lines that don't parse are skipped.
pub fn load(file: &Path) -> Vec<Run> {
    let Ok(text) = std::fs::read_to_string(file) else { return Vec::new() };
    let mut runs: Vec<Run> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    runs.reverse();
    runs
}

pub fn clear(file: &Path) -> Result<(), String> {
    match std::fs::remove_file(file) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Could not clear {}: {e}", file.display())),
    }
}

pub fn truncate(output: &str) -> String {
    match output.char_indices().nth(OUTPUT_LIMIT) {
        Some((i, _)) => format!("{}\n[…]", &output[..i]),
        None => output.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_round_trip_newest_first_and_bad_lines_are_skipped() {
        let file = std::env::temp_dir().join(format!("hex-meetings-activity-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&file);
        let run = |at_ms, outcome| Run {
            at_ms,
            meeting_id: "m".into(),
            meeting_title: "Call".into(),
            recipe: "Summary".into(),
            sends_to: "OpenCode".into(),
            program: "opencode".into(),
            outcome,
            output: "ok".into(),
        };
        append(&file, &run(1, Outcome::Done)).unwrap();
        std::fs::OpenOptions::new().append(true).open(&file).unwrap().write_all(b"not json\n").unwrap();
        append(&file, &run(2, Outcome::Failed)).unwrap();
        let runs = load(&file);
        assert_eq!(runs.iter().map(|r| r.at_ms).collect::<Vec<_>>(), [2, 1]);
        clear(&file).unwrap();
        assert!(load(&file).is_empty());
        clear(&file).unwrap();
    }

    #[test]
    fn long_output_is_cut_on_a_character() {
        let long = "é".repeat(OUTPUT_LIMIT + 5);
        assert_eq!(truncate(&long).chars().count(), OUTPUT_LIMIT + 4);
        assert_eq!(truncate("short"), "short");
    }
}
