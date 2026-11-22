//! Reports: what to make from a call, one Markdown file each in the
//! reports folder. A report is a name, what to focus on (the file's body),
//! where the result goes, and optionally which agent writes it:
//!
//! ```text
//! ---
//! name: Customer feedback
//! to: slack
//! ---
//! Feature requests and complaints. Quote the customer.
//! ```
//!
//! The person writes only the focus; [`Report::prompt`] adds the rest (the
//! call's details, who "You" and "Them" are, the format the destination
//! needs, the transcript). A body that uses `{{transcript}}` is the whole
//! prompt instead, for full control.
//!
//! Where it goes: nowhere (`to` unset, the result shows here), or `to:
//! slack|github|linear|issues`, which Hex Meetings posts after the person
//! approves it. `sends_to` instead names a place the agent posts to itself,
//! with its own connections and its tools allowed. `agent: claude` picks an
//! agent; nothing runs through a shell. `auto: true` runs the report on
//! each call as it finishes; posting still waits for the person, so it can't
//! be combined with `sends_to`.

use std::path::{Path, PathBuf};

use crate::agents::{self, Agent};
use crate::places::To;

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// The file's stem, stable across edits.
    pub id: String,
    pub name: String,
    pub description: String,
    /// An agent this report always uses, instead of the default.
    pub agent: Option<&'static Agent>,
    /// A place the agent posts to itself, with its own tools.
    pub sends_to: Option<String>,
    /// A place Hex Meetings posts the result to, once approved.
    pub to: Option<To>,
    /// Runs by itself on each call as it finishes.
    pub auto: bool,
    /// What to focus on: the file's body.
    pub focus: String,
    pub path: PathBuf,
}

/// The values a prompt and a command can use.
pub struct Vars<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub date: &'a str,
    pub duration: &'a str,
    pub app: &'a str,
    pub summary: &'a str,
    pub transcript: &'a str,
}

impl Vars<'_> {
    /// Fills every placeholder but the transcript.
    fn fill_meta(&self, template: &str) -> String {
        template
            .replace("{{id}}", self.id)
            .replace("{{title}}", self.title)
            .replace("{{date}}", self.date)
            .replace("{{duration}}", self.duration)
            .replace("{{app}}", self.app)
            .replace("{{summary}}", self.summary)
    }

    fn fill(&self, template: &str) -> String {
        // The transcript goes in last so text inside it is never treated as a placeholder.
        template
            .replace("{{id}}", self.id)
            .replace("{{title}}", self.title)
            .replace("{{date}}", self.date)
            .replace("{{duration}}", self.duration)
            .replace("{{app}}", self.app)
            .replace("{{summary}}", self.summary)
            .replace("{{transcript}}", self.transcript)
    }
}

/// How one report runs with the agent chosen at the time.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub command: Vec<String>,
    /// Everywhere the transcript or its result goes, for the confirm step.
    pub sends_to: String,
    /// The agent's name.
    pub runs: String,
    pub agent: Option<&'static str>,
    /// The agent may use its connected tools without asking.
    pub tools: bool,
}

impl Report {
    /// One line about the report: its description, else its focus's first line.
    pub fn blurb(&self) -> String {
        let line = if self.description.is_empty() { self.focus.lines().find(|l| !l.trim().is_empty()).unwrap_or_default() } else { &self.description };
        let line = line.trim().trim_start_matches(['-', '*', '#', ' ']);
        match line.char_indices().nth(110) {
            Some((i, _)) => format!("{}…", &line[..i]),
            None => line.to_string(),
        }
    }

    /// The prompt for one call: the focus wrapped in what every report needs,
    /// or the body as it is when it places `{{transcript}}` itself.
    pub fn prompt(&self, vars: &Vars) -> String {
        let format = match (self.to, &self.sends_to) {
            (Some(to), _) => to.instructions(),
            (None, Some(_)) => "",
            (None, None) => "Reply in Markdown, with no preamble. Write action items as to-dos: `- [ ] Owner: task`.",
        };
        if self.focus.contains("{{transcript}}") {
            let prompt = vars.fill(&self.focus);
            return if format.is_empty() || self.to.is_none() { prompt } else { format!("{prompt}\n\n{format}") };
        }
        let mut parts = vec![
            format!("Turn this recorded call into \"{}\".", self.name),
            format!(
                "The call: {} ({}, {}, {}). Hex transcribed it from two sides: \"You\" is the person who recorded it, and \"Them\" \
                 is everyone else on the call, so several people may speak as \"Them\". Use only what the transcript says. \
                 If something is unclear, say so instead of guessing.",
                vars.title, vars.app, vars.date, vars.duration
            ),
        ];
        if !self.focus.trim().is_empty() {
            parts.push(format!("Focus on:\n{}", vars.fill_meta(self.focus.trim())));
        }
        if !format.is_empty() {
            parts.push(format.to_string());
        }
        parts.push(format!("Transcript:\n\n{}", vars.transcript));
        parts.join("\n\n")
    }

    /// The report as its file: the settings, then the focus.
    pub fn to_markdown(&self) -> String {
        let mut head = vec![format!("name: {}", self.name.trim())];
        if !self.description.is_empty() {
            head.push(format!("description: {}", self.description));
        }
        if let Some(a) = self.agent {
            head.push(format!("agent: {}", a.id));
        }
        if let Some(to) = self.to {
            head.push(format!("to: {}", to.id()));
        }
        if let Some(place) = &self.sends_to {
            head.push(format!("sends_to: {place}"));
        }
        if self.auto {
            head.push("auto: true".into());
        }
        format!("---\n{}\n---\n{}\n", head.join("\n"), self.focus.trim())
    }

    /// The command and destinations for this report, given the default agent
    /// and the model chosen for an agent (`model(agent_id)`).
    pub fn plan(&self, default: Option<&'static Agent>, model: impl Fn(&str) -> Option<String>) -> Result<Plan, String> {
        let agent = self.agent.or(default).ok_or("No agent is installed. Install one of the agents listed in Setup, such as OpenCode or Claude Code.")?;
        let model = model(agent.id);
        let label = agent.model_label(model.as_deref());
        let sends_to = match (&self.sends_to, self.to) {
            (Some(place), _) => format!("{label}, then {place}"),
            (None, Some(to)) => format!("{label}, then {} once you approve it", to.name()),
            (None, None) => label,
        };
        let tools = self.sends_to.is_some();
        Ok(Plan { command: agent.command(tools, model.as_deref()), sends_to, runs: agent.name.to_string(), agent: Some(agent.id), tools })
    }

    /// The program and its arguments for one meeting.
    pub fn argv(&self, command: &[String], vars: &Vars) -> Vec<String> {
        let prompt = self.prompt(vars);
        let mut argv: Vec<String> =
            command.iter().map(|arg| if arg == "{prompt}" { prompt.clone() } else { vars.fill(arg) }).collect();
        if !command.iter().any(|a| a == "{prompt}") {
            argv.push(prompt);
        }
        argv
    }
}

/// A command as a report would write it, for display.
pub fn command_line(command: &[String]) -> String {
    command
        .iter()
        .map(|a| if a.contains(' ') { format!("\"{a}\"") } else { a.clone() })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn parse(id: &str, path: &Path, source: &str) -> Result<Report, String> {
    let source = source.replace("\r\n", "\n");
    let rest = source
        .strip_prefix("---\n")
        .ok_or_else(|| format!("{id}: a report starts with a --- block"))?;
    let (head, body) = rest
        .split_once("\n---\n")
        .or_else(|| rest.strip_suffix("\n---").map(|h| (h, "")))
        .ok_or_else(|| format!("{id}: the --- block never closes"))?;
    let mut name = None;
    let mut description = String::new();
    let mut agent = None;
    let mut sends_to = None;
    let mut to = None;
    let mut auto = false;
    for line in head.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("{id}: \"{line}\" isn't key: value"));
        };
        let value = value.trim().to_string();
        match key.trim() {
            "name" => name = Some(value),
            "description" => description = value,
            "agent" => {
                let names = agents::AGENTS.iter().map(|a| a.id).collect::<Vec<_>>().join(", ");
                agent = Some(agents::by_id(&value).ok_or_else(|| format!("{id}: unknown agent \"{value}\" (use one of {names})"))?);
            }
            "auto" => {
                auto = match value.as_str() {
                    "true" | "yes" => true,
                    "false" | "no" => false,
                    _ => return Err(format!("{id}: auto: is true or false, not \"{value}\"")),
                }
            }
            "command" => return Err(format!("{id}: command: isn't supported any more. Pick an agent with agent: instead")),
            "sends_to" => sends_to = Some(value).filter(|v| !v.is_empty()),
            "to" => to = Some(To::parse(&value).ok_or_else(|| format!("{id}: to: is slack, github, linear or issues, not \"{value}\""))?),
            other => return Err(format!("{id}: unknown key \"{other}\"")),
        }
    }
    if auto && sends_to.is_some() {
        return Err(format!("{id}: auto can't be used with sends_to, or your agent would post without anyone approving it"));
    }
    if to.is_some() && sends_to.is_some() {
        return Err(format!("{id}: use to (Hex Meetings posts it) or sends_to (your agent posts it), not both"));
    }
    Ok(Report {
        id: id.to_string(),
        name: name.ok_or_else(|| format!("{id}: needs a name"))?,
        description,
        agent,
        sends_to,
        to,
        auto,
        focus: body.trim().to_string(),
        path: path.to_path_buf(),
    })
}

/// A file name for a new report: its name in lowercase words, unique in `dir`.
pub fn new_id(dir: &Path, name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if slug.is_empty() { "report".to_string() } else { slug };
    let mut id = slug.clone();
    let mut n = 2;
    while dir.join(format!("{id}.md")).exists() {
        id = format!("{slug}-{n}");
        n += 1;
    }
    id
}

/// The one report every call has without making any: a summary shown on
/// the call. It isn't a file, so it can't be edited or deleted; people make
/// their own reports for anything else.
pub const BUILTIN_ID: &str = "@summary";

pub fn builtin() -> &'static Report {
    static BUILTIN: std::sync::LazyLock<Report> = std::sync::LazyLock::new(|| Report {
        id: BUILTIN_ID.into(),
        name: "Summary".into(),
        description: "A summary, decisions, action items and open questions, shown on the call".into(),
        agent: None,
        sends_to: None,
        to: None,
        auto: false,
        focus: "A summary in 3 to 5 sentences, then the decisions that were made, then the action items, each with its owner \
                (You, Them, or a name if one is said), then any open questions."
            .into(),
        path: PathBuf::new(),
    });
    &BUILTIN
}

/// Every report in `dir`, sorted by name, and a message for each file that
/// couldn't be read. A missing folder is no reports.
pub fn load(dir: &Path) -> (Vec<Report>, Vec<String>) {
    if !dir.exists() {
        return (Vec::new(), Vec::new());
    }
    let mut reports = Vec::new();
    let mut errors = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => return (reports, vec![format!("Could not read {}: {e}", dir.display())]),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let id = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        match std::fs::read_to_string(&path) {
            Ok(source) => match parse(&id, &path, &source) {
                Ok(report) => reports.push(report),
                Err(e) => errors.push(e),
            },
            Err(e) => errors.push(format!("{id}: {e}")),
        }
    }
    reports.sort_by_key(|r| r.name.to_lowercase());
    errors.sort();
    (reports, errors)
}

#[cfg(test)]
mod tests;
