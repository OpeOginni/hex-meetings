//! A record of every report run: which call went to which tool, when, and
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
    /// Older logs call it `recipe`.
    #[serde(alias = "recipe")]
    pub report: String,
    pub sends_to: String,
    /// The program that ran (`opencode`), not its arguments: those hold the transcript.
    pub program: String,
    pub outcome: Outcome,
    pub output: String,
    /// The report's `to:`, when Hex Meetings is to post the result.
    #[serde(default)]
    pub to: Option<String>,
    /// What happened to that post; None while it waits for approval.
    #[serde(default)]
    pub delivery: Option<Delivery>,
    /// It ran by itself when the call finished.
    #[serde(default)]
    pub auto: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Delivery {
    Posted { to: String, links: Vec<String> },
    Failed { to: String, error: String },
    Skipped,
}

impl Run {
    /// The output as Markdown to read: a list of issues instead of their JSON.
    pub fn readable(&self) -> String {
        if self.output.is_empty() {
            return "(no output)".into();
        }
        match crate::places::parse_issues(&self.output) {
            Ok(issues) if !issues.is_empty() && self.output.trim_start().starts_with(['[', '`']) => {
                issues.iter().map(|i| format!("- **{}**", i.title)).collect::<Vec<_>>().join("\n")
            }
            _ => self.output.clone(),
        }
    }

    /// A finished result still waiting to be posted or skipped.
    pub fn awaits_post(&self) -> bool {
        self.outcome == Outcome::Done && self.to.is_some() && self.delivery.is_none()
    }
}

impl Run {
    /// Writes the result into `dir` as `<date> <title> - <report>.md`, never
    /// over an existing file, and returns the path.
    pub fn save_markdown(&self, dir: &Path) -> Result<std::path::PathBuf, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("Could not make {}: {e}", dir.display()))?;
        let date = crate::time::date(self.at_ms);
        let clean = |s: &str| s.chars().map(|c| if matches!(c, '/' | ':' | '\\') { '-' } else { c }).collect::<String>();
        let stem = format!("{date} {} - {}", clean(&self.meeting_title), clean(&self.report));
        let mut path = dir.join(format!("{stem}.md"));
        let mut n = 2;
        while path.exists() {
            path = dir.join(format!("{stem} {n}.md"));
            n += 1;
        }
        let body = format!("# {}\n\n{} · {}\n\n{}\n", self.meeting_title, crate::time::long(self.at_ms), self.report, self.output.trim());
        std::fs::write(&path, body).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
        Ok(path)
    }
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

/// Writes every run again (after a post changes one), oldest first.
pub fn save_all(file: &Path, runs: &[Run]) -> Result<(), String> {
    let mut text = String::new();
    for run in runs.iter().rev() {
        text.push_str(&serde_json::to_string(run).map_err(|e| e.to_string())?);
        text.push('\n');
    }
    std::fs::write(file, text).map_err(|e| format!("Could not save {}: {e}", file.display()))
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
            report: "Summary".into(),
            sends_to: "OpenCode".into(),
            program: "opencode".into(),
            outcome,
            output: "ok".into(),
            to: None,
            delivery: None,
            auto: false,
        };
        append(&file, &run(1, Outcome::Done)).unwrap();
        std::fs::OpenOptions::new().append(true).open(&file).unwrap().write_all(b"not json\n").unwrap();
        append(&file, &run(2, Outcome::Failed)).unwrap();
        let runs = load(&file);
        assert_eq!(runs.iter().map(|r| r.at_ms).collect::<Vec<_>>(), [2, 1]);
        let mut runs = load(&file);
        runs[0].delivery = Some(Delivery::Posted { to: "Slack".into(), links: vec![] });
        save_all(&file, &runs).unwrap();
        assert_eq!(load(&file), runs);
        clear(&file).unwrap();
        assert!(load(&file).is_empty());
        clear(&file).unwrap();
    }

    #[test]
    fn a_result_saves_as_markdown_without_overwriting() {
        let dir = std::env::temp_dir().join(format!("hex-meetings-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let run = Run {
            at_ms: 1_790_000_000_000,
            meeting_title: "Acme / intro".into(),
            report: "Summary".into(),
            output: "**Done**".into(),
            ..crate::demo::summary_run()
        };
        let first = run.save_markdown(&dir).unwrap();
        let second = run.save_markdown(&dir).unwrap();
        assert_ne!(first, second);
        assert!(first.file_name().unwrap().to_string_lossy().ends_with("Acme - intro - Summary.md"), "{first:?}");
        assert!(std::fs::read_to_string(&first).unwrap().contains("**Done**"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn issues_read_as_a_list() {
        let mut run = crate::demo::summary_run();
        run.output = r#"[{"title": "Export times out", "body": "x"}]"#.into();
        assert_eq!(run.readable(), "- **Export times out**");
        run.output = "Plain [link] text".into();
        assert_eq!(run.readable(), "Plain [link] text");
    }

    #[test]
    fn long_output_is_cut_on_a_character() {
        let long = "é".repeat(OUTPUT_LIMIT + 5);
        assert_eq!(truncate(&long).chars().count(), OUTPUT_LIMIT + 4);
        assert_eq!(truncate("short"), "short");
    }
}
