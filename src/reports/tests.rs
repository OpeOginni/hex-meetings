use super::*;

fn no_model(_: &str) -> Option<String> {
    None
}

fn vars<'a>(transcript: &'a str) -> Vars<'a> {
    Vars {
        id: "1790000000000-1001",
        title: "Acme intro",
        date: "2026-11-20 14:00",
        duration: "32 min",
        app: "Zoom",
        summary: "",
        transcript,
    }
}

#[test]
fn every_example_report_and_the_builtin_build_a_prompt() {
    for report in crate::demo::reports().iter().chain([builtin()]) {
        let id = &report.id;
        assert!(!report.focus.is_empty(), "{id}");
        assert!(report.agent.is_none(), "{id} should use the default agent");
        // The app adds the transcript; a starter only says what to focus on.
        assert!(report.prompt(&vars("[00:00] Them: hi\n")).ends_with("[00:00] Them: hi\n"), "{id}");
    }
}

#[test]
fn the_app_wraps_the_focus_in_what_every_report_needs() {
    let report = parse("f", Path::new("f.md"), "---\nname: Customer feedback\nto: slack\n---\nComplaints about {{app}}. Quote them.").unwrap();
    let prompt = report.prompt(&vars("[00:00] Them: the {{title}} export is slow\n"));
    assert!(prompt.starts_with("Turn this recorded call into \"Customer feedback\"."), "{prompt}");
    assert!(prompt.contains("The call: Acme intro (Zoom, 2026-11-20 14:00, 32 min)."), "{prompt}");
    assert!(prompt.contains("Focus on:\nComplaints about Zoom. Quote them."), "{prompt}");
    assert!(prompt.contains(crate::places::To::Place(crate::places::Place::Slack).instructions()), "{prompt}");
    // The transcript goes last and stays as spoken.
    assert!(prompt.ends_with("Transcript:\n\n[00:00] Them: the {{title}} export is slow\n"), "{prompt}");
    let here = parse("h", Path::new("h.md"), "---\nname: Notes\n---\n").unwrap().prompt(&vars("x"));
    assert!(!here.contains("Focus on") && here.contains("Reply in Markdown"), "{here}");
}

#[test]
fn a_report_writes_back_to_the_same_file() {
    let source = "---\nname: Issues\nagent: claude\nto: github\nauto: true\n---\nEvery bug.\n";
    let report = parse("i", Path::new("i.md"), source).unwrap();
    assert_eq!(report.to_markdown(), source);
    assert_eq!(parse("i", Path::new("i.md"), &report.to_markdown()).unwrap(), report);
    assert_eq!(report.blurb(), "Every bug.");
    let dir = std::env::temp_dir().join(format!("hex-meetings-ids-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(new_id(&dir, "Customer feedback!"), "customer-feedback");
    std::fs::write(dir.join("customer-feedback.md"), "").unwrap();
    assert_eq!(new_id(&dir, "Customer feedback"), "customer-feedback-2");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_report_runs_with_the_default_agent_and_tools_only_when_it_sends_somewhere() {
    let reports = crate::demo::reports();
    let opencode = crate::agents::by_id("opencode");
    let summary = builtin().plan(opencode, no_model).unwrap();
    assert_eq!(summary.sends_to, "The model your OpenCode is set to use");
    assert!(!summary.command.contains(&"--auto".to_string()));
    // Hex Meetings posts to Notion itself, so the agent needs no tools.
    let notion = reports.iter().find(|r| r.id == "notion-sales-call").unwrap().plan(opencode, no_model).unwrap();
    assert_eq!(notion.sends_to, "The model your OpenCode is set to use, then Notion once you approve it");
    assert!(!notion.tools);
    // Through the agent's own tools, it may use them unasked.
    let own = parse("o", Path::new("o.md"), "---\nname: O\nsends_to: our Sales Notion\n---\nA deal note.").unwrap().plan(opencode, no_model).unwrap();
    assert_eq!(own.sends_to, "The model your OpenCode is set to use, then our Sales Notion");
    assert!(own.command.contains(&"--auto".to_string()));
    let claude = reports[0].plan(crate::agents::by_id("claude"), no_model).unwrap();
    assert_eq!(claude.command[0], "claude");
    assert!(reports[0].plan(None, no_model).unwrap_err().contains("No agent"));
    let chosen = reports[0].plan(opencode, |_| Some("openai/gpt-5".into())).unwrap();
    assert!(chosen.sends_to.starts_with("openai/gpt-5 through OpenCode"), "{}", chosen.sends_to);
    assert!(chosen.command.windows(2).any(|w| w == ["--model", "openai/gpt-5"]));
}

#[test]
fn a_report_can_name_its_own_agent() {
    let report = parse("t", Path::new("t.md"), "---\nname: T\nagent: cursor\n---\n{{transcript}}").unwrap();
    let plan = report.plan(crate::agents::by_id("opencode"), no_model).unwrap();
    assert_eq!(plan.command[0], "cursor-agent");
    assert_eq!(plan.runs, "Cursor Agent");
}



#[test]
fn mistakes_are_named() {
    let err = |s: &str| parse("bad", Path::new("bad.md"), s).unwrap_err();
    assert!(err("no front matter").contains("starts with"));
    assert!(err("---\nname: x\n").contains("never closes"));
    assert!(err("---\ndescription: x\n---\nhi").contains("needs a name"));
    assert!(err("---\nname: x\nagent: gpt\n---\nhi").contains("unknown agent"));
    assert!(err("---\nname: x\ncommand: claude -p\n---\nhi").contains("isn't supported"));
    assert!(err("---\nname: x\ncolour: red\n---\n").contains("unknown key"));
    assert!(err("---\nname: x\nsends_to: our Jira\nauto: true\n---\n").contains("without anyone approving"));
    assert!(err("---\nname: x\nauto: sometimes\n---\n").contains("true or false"));
}

/// Runs the shipped summary report for real on the made-up demo call, with
/// OpenCode or the agent in `HEX_MEETINGS_AGENT` (`claude`, `pi`, ...), and the
/// model in `HEX_MEETINGS_MODEL` if set. Costs
/// a model request, so it only runs when asked:
/// `cargo test -- --ignored summary_report`.
#[test]
#[ignore]
fn summary_report_runs_through_an_agent() {
    let report = builtin();
    let d = crate::demo::sales_detail();
    let transcript = crate::hex::transcript_text(&d.entries);
    let agent = std::env::var("HEX_MEETINGS_AGENT").unwrap_or_else(|_| "opencode".into());
    let plan = report.plan(crate::agents::by_id(&agent), |_| std::env::var("HEX_MEETINGS_MODEL").ok()).unwrap();
    let argv = report.argv(&plan.command, &Vars { transcript: &transcript, ..vars("") });
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ended = crate::runner::run(&argv, &stop);
    println!("{}", ended.output);
    assert_eq!(ended.outcome, crate::activity::Outcome::Done, "{}", ended.output);
    assert!(ended.output.to_lowercase().contains("northwind"));
    assert!(ended.output.contains("- [ ]"), "action items shown on the call are to-dos");
}

/// The Summary sent to Slack for real, on the made-up sales call: the reply
/// must use Slack's formatting, not Markdown. `cargo test -- --ignored slack_summary`.
#[test]
#[ignore]
fn slack_summary_reply_uses_slack_formatting() {
    let mut report = builtin().clone();
    report.to = Some(crate::places::To::Place(crate::places::Place::Slack));
    let transcript = crate::hex::transcript_text(&crate::demo::sales_detail().entries);
    let agent = std::env::var("HEX_MEETINGS_AGENT").unwrap_or_else(|_| "opencode".into());
    let plan = report.plan(crate::agents::by_id(&agent), |_| std::env::var("HEX_MEETINGS_MODEL").ok()).unwrap();
    let argv = report.argv(&plan.command, &Vars { transcript: &transcript, ..vars("") });
    let ended = crate::runner::run(&argv, &std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    println!("{}", ended.output);
    assert_eq!(ended.outcome, crate::activity::Outcome::Done, "{}", ended.output);
    assert!(!ended.output.contains("**"), "Slack bold is *one* asterisk");
    assert!(!ended.output.lines().any(|l| l.trim_start().starts_with('#')), "Slack has no # headings");
    assert!(!ended.output.contains("- [ ]"), "Slack shows Markdown to-dos as text");
}

#[test]
fn a_new_folder_has_no_reports_but_the_builtin_summary_is_always_there() {
    let dir = std::env::temp_dir().join(format!("hex-meetings-reports-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (reports, errors) = load(&dir);
    assert!(reports.is_empty() && errors.is_empty());
    assert!(!dir.exists(), "loading writes nothing");
    assert_eq!(builtin().id, BUILTIN_ID);
    assert!(builtin().to.is_none() && builtin().sends_to.is_none(), "the summary shows on the call");
}

/// The support report for real, on a made-up call: the agent's reply must be
/// a list of issues Hex Meetings can post. `cargo test -- --ignored issues_report`.
#[test]
#[ignore]
fn issues_report_reply_parses() {
    let reports = crate::demo::reports();
    let report = reports.iter().find(|r| r.id == "support-issues").unwrap();
    // A made-up support call.
    let transcript = "[00:02] Them: Our CSV export stops after about a minute when we export more than 50,000 rows.\n\
        [00:15] You: Does it show an error?\n\
        [00:18] Them: Just a timeout. And the dates in the file come out as month first, but we're in Germany.\n\
        [00:31] You: Got it, I'll file both.\n";
    let agent = std::env::var("HEX_MEETINGS_AGENT").unwrap_or_else(|_| "opencode".into());
    let plan = report.plan(crate::agents::by_id(&agent), |_| std::env::var("HEX_MEETINGS_MODEL").ok()).unwrap();
    assert!(!plan.tools, "an agent that only writes needs no tools");
    let argv = report.argv(&plan.command, &Vars { transcript, ..vars("") });
    let ended = crate::runner::run(&argv, &std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    println!("{}", ended.output);
    let issues = crate::places::parse_issues(&ended.output).unwrap();
    println!("{issues:#?}");
    assert_eq!(issues.len(), 2, "two problems were reported");
}

/// The Notion report for real, on the made-up sales call: the agent's
/// Markdown must become Notion blocks. `cargo test -- --ignored notion_report`.
#[test]
#[ignore]
fn notion_report_reply_becomes_blocks() {
    let reports = crate::demo::reports();
    let report = reports.iter().find(|r| r.id == "notion-sales-call").unwrap();
    let transcript = crate::hex::transcript_text(&crate::demo::sales_detail().entries);
    let agent = std::env::var("HEX_MEETINGS_AGENT").unwrap_or_else(|_| "opencode".into());
    let plan = report.plan(crate::agents::by_id(&agent), |_| std::env::var("HEX_MEETINGS_MODEL").ok()).unwrap();
    assert!(!plan.tools);
    let argv = report.argv(&plan.command, &Vars { transcript: &transcript, ..vars("") });
    let ended = crate::runner::run(&argv, &std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    println!("{}", ended.output);
    let blocks = crate::places::notion::blocks(&ended.output);
    let kinds: Vec<&str> = blocks.iter().map(|b| b["type"].as_str().unwrap()).collect();
    println!("{kinds:?}");
    assert!(kinds.contains(&"to_do"), "next steps should be to-dos");
    assert!(kinds.iter().any(|k| k.starts_with("heading")), "sections should be headings");
}

#[test]
fn the_prompt_is_one_argument_and_the_transcript_stays_as_spoken() {
    let report = parse("t", Path::new("t.md"), "---\nname: T\n---\nSum up").unwrap();
    let plan = report.plan(crate::agents::by_id("claude"), no_model).unwrap();
    let argv = report.argv(&plan.command, &vars("[00:00] Them: hi; rm -rf ~ {{title}}\n"));
    assert_eq!(argv[0], "claude");
    // The whole prompt is the last argument; nothing goes through a shell.
    assert_eq!(argv.len(), plan.command.len());
    assert!(argv.last().unwrap().ends_with("[00:00] Them: hi; rm -rf ~ {{title}}\n"));
}
