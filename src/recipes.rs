//! Recipes: a prompt plus the agent command that receives it, one Markdown
//! file each in the recipes folder. The app never talks to a model or a
//! service itself; a recipe hands the transcript to a tool the user already
//! has (OpenCode, Claude Code, anything with a command line) and that tool's
//! own setup decides the model and the connections (Notion, Linear, ...).
//!
//! ```text
//! ---
//! name: Summary and action items
//! description: Shown under the name.
//! command: opencode run --archived {prompt}
//! sends_to: The model your OpenCode uses
//! ---
//! The prompt, with {{title}}, {{date}}, {{duration}}, {{app}}, {{id}},
//! {{summary}} and {{transcript}} filled in.
//! ```
//!
//! `{prompt}` in the command is replaced by the filled-in prompt as one
//! argument; nothing goes through a shell. `sends_to` is shown before every
//! run, so the person sees where the transcript goes.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    /// The file's stem, stable across edits.
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: Vec<String>,
    pub sends_to: String,
    pub prompt: String,
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

impl Recipe {
    /// The program and its arguments for one meeting.
    pub fn argv(&self, vars: &Vars) -> Vec<String> {
        let prompt = vars.fill(&self.prompt);
        let mut argv: Vec<String> = self
            .command
            .iter()
            .map(|arg| if arg == "{prompt}" { prompt.clone() } else { vars.fill(arg) })
            .collect();
        if !self.command.iter().any(|a| a == "{prompt}") {
            argv.push(prompt);
        }
        argv
    }

    /// The command as the recipe writes it, for display.
    pub fn command_line(&self) -> String {
        self.command
            .iter()
            .map(|a| if a.contains(' ') { format!("\"{a}\"") } else { a.clone() })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Splits a command line on spaces, keeping "double quoted" words together.
fn split_command(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut word));
                    any = false;
                }
            }
            c => {
                word.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(word);
    }
    out
}

pub fn parse(id: &str, path: &Path, source: &str) -> Result<Recipe, String> {
    let source = source.replace("\r\n", "\n");
    let rest = source
        .strip_prefix("---\n")
        .ok_or_else(|| format!("{id}: a recipe starts with a --- block"))?;
    let (head, body) = rest
        .split_once("\n---\n")
        .or_else(|| rest.strip_suffix("\n---").map(|h| (h, "")))
        .ok_or_else(|| format!("{id}: the --- block never closes"))?;
    let mut name = None;
    let mut description = String::new();
    let mut command = None;
    let mut sends_to = None;
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
            "command" => command = Some(split_command(&value)),
            "sends_to" => sends_to = Some(value),
            other => return Err(format!("{id}: unknown key \"{other}\"")),
        }
    }
    let command = command.filter(|c| !c.is_empty()).ok_or_else(|| format!("{id}: needs a command"))?;
    let sends_to = sends_to.unwrap_or_else(|| format!("Whatever {} is set up to use", command[0]));
    Ok(Recipe {
        id: id.to_string(),
        name: name.ok_or_else(|| format!("{id}: needs a name"))?,
        description,
        command,
        sends_to,
        prompt: body.trim().to_string(),
        path: path.to_path_buf(),
    })
}

/// The recipes shipped with the app, written to the folder the first time
/// it's empty. People edit or delete them freely afterwards.
pub const DEFAULTS: &[(&str, &str)] = &[
    ("summary", include_str!("../recipes/summary.md")),
    ("notion-sales-call", include_str!("../recipes/notion-sales-call.md")),
    ("support-issues", include_str!("../recipes/support-issues.md")),
];

/// [`DEFAULTS`] parsed, without touching the disk (scenes and tests).
pub fn defaults() -> (Vec<Recipe>, Vec<String>) {
    let mut recipes = Vec::new();
    let mut errors = Vec::new();
    for (id, source) in DEFAULTS {
        match parse(id, Path::new(&format!("{id}.md")), source) {
            Ok(r) => recipes.push(r),
            Err(e) => errors.push(e),
        }
    }
    recipes.sort_by_key(|r| r.name.to_lowercase());
    (recipes, errors)
}

/// Every recipe in `dir`, sorted by name, and a message for each file that
/// couldn't be read. Writes [`DEFAULTS`] first when the folder doesn't exist.
pub fn load(dir: &Path) -> (Vec<Recipe>, Vec<String>) {
    if !dir.exists() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            return (Vec::new(), vec![format!("Could not make {}: {e}", dir.display())]);
        }
        for (id, source) in DEFAULTS {
            let _ = std::fs::write(dir.join(format!("{id}.md")), source);
        }
    }
    let mut recipes = Vec::new();
    let mut errors = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => return (recipes, vec![format!("Could not read {}: {e}", dir.display())]),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let id = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        match std::fs::read_to_string(&path) {
            Ok(source) => match parse(&id, &path, &source) {
                Ok(recipe) => recipes.push(recipe),
                Err(e) => errors.push(e),
            },
            Err(e) => errors.push(format!("{id}: {e}")),
        }
    }
    recipes.sort_by_key(|r| r.name.to_lowercase());
    errors.sort();
    (recipes, errors)
}

#[cfg(test)]
mod tests;
