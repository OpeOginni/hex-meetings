//! The agent command-line tools a report can hand a call to. The app only
//! starts them; each one's own setup decides the model and the connected
//! tools (Notion, Slack, Linear, ...).

/// One agent CLI and how to run it once, without a terminal.
#[derive(Debug, PartialEq)]
pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    pub program: &'static str,
    /// The arguments for one run. `{prompt}` is where the filled-in prompt goes.
    pub args: &'static [&'static str],
    /// Added when the report sends the result somewhere, so the agent can use
    /// its connected tools without stopping to ask (nobody is there to answer).
    pub tool_args: &'static [&'static str],
    /// Added when it doesn't: an agent that would otherwise run tools unasked
    /// gets none for a result that only shows on the call.
    pub plain_args: &'static [&'static str],
    /// How to connect a tool such as Notion, for the Setup page.
    pub connect_hint: &'static str,
    /// The official one-line install command for macOS.
    pub install: &'static str,
    /// Where to find the names `--model` takes.
    pub model_hint: &'static str,
    /// How to know which models it offers.
    pub models: Models,
}

/// Where an agent's model names come from.
#[derive(Debug, PartialEq)]
pub enum Models {
    /// It has no way to list them: the name is typed.
    Typed,
    /// A short documented list.
    Fixed(&'static [&'static str]),
    /// Its own command lists them (`opencode models`, `pi --list-models`).
    Listed(&'static [&'static str]),
}

pub const AGENTS: &[Agent] = &[
    Agent {
        id: "opencode",
        name: "OpenCode",
        program: "opencode",
        args: &["run", "--archived", "--title", "Hex Meetings: {{title}}", "{prompt}"],
        tool_args: &["--auto"],
        plain_args: &[],
        connect_hint: "opencode mcp add",
        install: "curl -fsSL https://opencode.ai/v2/install | bash",
        model_hint: "See the names with opencode models.",
        models: Models::Listed(&["models"]),
    },
    Agent {
        id: "claude",
        name: "Claude Code",
        program: "claude",
        args: &["-p", "--no-session-persistence", "{prompt}"],
        tool_args: &["--dangerously-skip-permissions"],
        plain_args: &[],
        connect_hint: "claude mcp add",
        install: "curl -fsSL https://claude.ai/install.sh | bash",
        model_hint: "sonnet, opus, haiku or a full model name.",
        models: Models::Fixed(&["sonnet", "opus", "haiku"]),
    },
    Agent {
        id: "codex",
        name: "Codex",
        program: "codex",
        // Not a git repository: the app's data folder.
        args: &["exec", "--skip-git-repo-check", "--color", "never", "{prompt}"],
        tool_args: &["--full-auto"],
        plain_args: &[],
        connect_hint: "codex mcp add",
        install: "curl -fsSL https://chatgpt.com/codex/install.sh | sh",
        model_hint: "A name from Codex’s /model menu.",
        models: Models::Typed,
    },
    Agent {
        id: "cursor",
        name: "Cursor Agent",
        program: "cursor-agent",
        args: &["-p", "--output-format", "text", "{prompt}"],
        tool_args: &["--force"],
        plain_args: &[],
        connect_hint: "cursor-agent mcp",
        install: "curl https://cursor.com/install -fsS | bash",
        model_hint: "A name from Cursor’s /model menu.",
        models: Models::Typed,
    },
    Agent {
        id: "pi",
        name: "Pi",
        program: "pi",
        args: &["-p", "--no-session", "{prompt}"],
        // Pi never asks before using a tool, and connects services through
        // extensions instead of MCP.
        tool_args: &[],
        plain_args: &["--no-tools"],
        connect_hint: "a Pi extension (pi install)",
        install: "curl -fsSL https://pi.dev/install.sh | sh",
        model_hint: "See the names with pi --list-models.",
        models: Models::Listed(&["--list-models"]),
    },
];

pub fn by_id(id: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|a| a.id == id)
}

impl Agent {
    /// The program and arguments for one run; `tools` lets it use its
    /// connections unasked, and `model` overrides the agent's own default.
    pub fn command(&self, tools: bool, model: Option<&str>) -> Vec<String> {
        let mut out = vec![self.program.to_string()];
        for arg in self.args {
            if *arg == "{prompt}" {
                let extra = if tools { self.tool_args } else { self.plain_args };
                out.extend(extra.iter().map(|a| a.to_string()));
                if let Some(m) = model {
                    out.extend(["--model".to_string(), m.to_string()]);
                }
            }
            out.push(arg.to_string());
        }
        out
    }

    pub fn installed(&self) -> bool {
        crate::paths::which(self.program).is_some()
    }

    /// Who reads the transcript: the chosen model, or the agent's default.
    pub fn model_label(&self, model: Option<&str>) -> String {
        match model {
            Some(m) => format!("{m} through {}", self.name),
            None => format!("The model your {} is set to use", self.name),
        }
    }
}

/// How long an agent gets to list its models before the name is typed instead.
const LIST_TIME: std::time::Duration = std::time::Duration::from_secs(20);

impl Agent {
    /// The models its own command lists. Starts a process and waits: call it
    /// from a worker thread. Any failure is an error, and the name is typed.
    pub fn list_models(&self) -> Result<Vec<String>, String> {
        let Models::Listed(args) = self.models else { return Err("This agent can't list its models.".into()) };
        let program = crate::paths::which(self.program).ok_or(format!("{} isn't installed.", self.name))?;
        let mut child = std::process::Command::new(program)
            .args(args)
            .env("PATH", crate::paths::search_path())
            .current_dir(std::env::temp_dir())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        // Read both streams while it runs (Pi prints its list to stderr), so a
        // long list can't fill a pipe and stall it.
        fn drain(pipe: Option<impl std::io::Read + Send + 'static>) -> std::thread::JoinHandle<String> {
            std::thread::spawn(move || {
                let mut out = String::new();
                if let Some(mut p) = pipe {
                    let _ = p.read_to_string(&mut out);
                }
                out
            })
        }
        let (out, err) = (drain(child.stdout.take()), drain(child.stderr.take()));
        let started = std::time::Instant::now();
        loop {
            match child.try_wait().map_err(|e| e.to_string())? {
                Some(status) if status.success() => break,
                Some(_) => return Err(format!("{} couldn't list its models.", self.name)),
                None if started.elapsed() > LIST_TIME => {
                    let _ = child.kill();
                    return Err(format!("{} took too long to list its models.", self.name));
                }
                None => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
        let text = out.join().unwrap_or_default() + "\n" + &err.join().unwrap_or_default();
        let models = parse_models(&text);
        if models.is_empty() { Err(format!("{} listed no models.", self.name)) } else { Ok(models) }
    }
}

/// Model names from a list command: one `provider/model` per line
/// (OpenCode), or a table whose first two columns are provider and model
/// (Pi). Anything not shaped like a name (headers, warnings) is skipped.
/// Repeats are dropped, order kept.
pub fn parse_models(text: &str) -> Vec<String> {
    let id = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    let model = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c));
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let name = match words.as_slice() {
            ["provider", "model", ..] => continue,
            [one] if one.contains('/') && model(one) => one.to_string(),
            [provider, name, ..] if id(provider) && model(name) && !line.contains(": ") => format!("{provider}/{name}"),
            _ => continue,
        };
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// The agent reports use by default: the chosen one, else the first installed.
pub fn default_agent(chosen: Option<&str>, installed: &[&'static str]) -> Option<&'static Agent> {
    chosen
        .and_then(by_id)
        .filter(|a| installed.contains(&a.id))
        .or_else(|| AGENTS.iter().find(|a| installed.contains(&a.id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_flags_go_before_the_prompt() {
        let claude = by_id("claude").unwrap();
        assert_eq!(claude.command(false, None), ["claude", "-p", "--no-session-persistence", "{prompt}"]);
        assert_eq!(
            claude.command(true, Some("opus")),
            ["claude", "-p", "--no-session-persistence", "--dangerously-skip-permissions", "--model", "opus", "{prompt}"]
        );
    }

    #[test]
    fn pi_gets_no_tools_unless_the_report_sends_somewhere() {
        let pi = by_id("pi").unwrap();
        assert_eq!(pi.command(false, None), ["pi", "-p", "--no-session", "--no-tools", "{prompt}"]);
        assert_eq!(pi.command(true, None), ["pi", "-p", "--no-session", "{prompt}"]);
    }

    #[test]
    fn model_lists_read_both_formats() {
        let opencode = "github-copilot/auto\nopenai/gpt-5\n\nopenai/gpt-5\n";
        assert_eq!(parse_models(opencode), ["github-copilot/auto", "openai/gpt-5"]);
        let pi = "provider        model             context  max-out\ngithub-copilot  gpt-4.1           128K     16.4K\ngithub-copilot  claude-opus-4.5   160K     32K\n";
        assert_eq!(parse_models(pi), ["github-copilot/gpt-4.1", "github-copilot/claude-opus-4.5"]);
        assert!(parse_models("").is_empty());
        // Warnings and stray words aren't models.
        assert!(parse_models("Warning: no API key set\nloading...\n").is_empty());
    }

    /// The installed OpenCode and Pi list their models: `cargo test -- --ignored lists_real_models`.
    #[test]
    #[ignore]
    fn lists_real_models() {
        for id in ["opencode", "pi"] {
            let a = by_id(id).unwrap();
            match a.list_models() {
                Ok(m) => println!("{id}: {} models, first {}", m.len(), m[0]),
                Err(e) => println!("{id}: {e}"),
            }
        }
    }

    #[test]
    fn the_default_is_the_chosen_agent_if_installed() {
        assert_eq!(default_agent(Some("claude"), &["opencode", "claude"]).unwrap().id, "claude");
        assert_eq!(default_agent(Some("claude"), &["cursor"]).unwrap().id, "cursor");
        assert_eq!(default_agent(None, &["claude", "opencode"]).unwrap().id, "opencode");
        assert!(default_agent(None, &[]).is_none());
    }
}
