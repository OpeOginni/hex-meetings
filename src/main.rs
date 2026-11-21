//! Hex Meetings: an unofficial companion for Hex's Meetings (beta). It
//! reads the calls Hex recorded through Hex's public command line, shows
//! them, and hands a transcript to whatever agent the person already uses,
//! through reports they control. Not affiliated with Hex or Anomaly.
//!
//! `cargo run` opens the window. `cargo run -- --snap out.png --scene demo`
//! renders a scene headless (`--list-scenes` lists them).

mod activity;
mod agents;
mod app;
mod demo;
mod hex;
mod paths;
mod places;
mod reports;
mod runner;
mod settings;
mod speech;
mod time;
mod ui;

use app::{App, HexStatus, Page};
use rune::AppSpec;

/// Rune's headless flags: scenes render demo data and start no workers.
fn headless() -> bool {
    std::env::args().any(|a| {
        matches!(a.as_str(), "--snap" | "--tree" | "--trace" | "--frames" | "--strip" | "--video" | "--script" | "--list-scenes")
    })
}

fn init() -> App {
    if headless() || std::env::var_os("HEX_MEETINGS_DEMO").is_some() {
        return demo::app();
    }
    App::new()
}

type Scene = (&'static str, fn(&mut App));

const SCENES: &[Scene] = &[
    ("demo", |_| {}),
    ("confirm", |s| s.armed = Some("notion-sales-call".into())),
    ("running", |s| {
        s.running = Some(app::Running {
            meeting_id: "demo-sales".into(),
            report: "Sales call to Notion".into(),
            agent: Some("opencode"),
            started: std::time::Instant::now() - std::time::Duration::from_secs(12),
            stop: Default::default(),
        })
    }),
    ("result", |s| s.activity.insert(0, demo::summary_run())),
    ("saved", |s| {
        let run = demo::summary_run();
        s.saved = Some((run.at_ms, "/Users/demo/Documents/Meeting summaries/2026-11-21 Northwind intro call - Summary and action items.md".into()));
        s.activity.insert(0, run);
    }),
    ("deleting", |s| {
        s.activity.insert(0, demo::summary_run());
        s.deleting = Some("demo-sales".into());
    }),
    ("setup", |s| s.page = Page::Setup),
    ("no-reports", |s| s.reports.clear()),
    ("model-menu", |s| {
        s.page = Page::Setup;
        s.model_menu_open = true;
    }),
    ("model-failed", |s| {
        s.page = Page::Setup;
        s.model_lists.insert("opencode", Err("OpenCode took too long to list its models.".into()));
    }),
    ("model-claude", |s| {
        s.page = Page::Setup;
        s.settings.agent = Some("claude".into());
        s.model_menu_open = true;
    }),
    ("model-typed", |s| {
        s.page = Page::Setup;
        s.settings.models.insert("opencode".into(), "my-provider/custom-model".into());
        s.model_edit = rune::TextEdit::new("my-provider/custom-model");
    }),
    ("about", |s| s.page = Page::About),
    ("activity-empty", |s| {
        s.page = Page::Activity;
        s.activity.clear();
    }),
    ("reports-empty", |s| {
        s.page = Page::Reports;
        s.reports.clear();
    }),
    ("search-empty", |s| s.search = rune::TextEdit::new("budget review")),
    ("connect-notion", |s| {
        s.page = Page::Setup;
        s.start_connect(places::Place::Notion);
    }),
    ("edit-agent-tools", |s| {
        s.page = Page::Reports;
        s.edit_report("customer-feedback");
        if let Some(d) = s.editing.as_mut() {
            d.sends_to = Some(String::new());
            d.place = rune::TextEdit::new("Our product feedback Notion");
            d.agent = agents::by_id("claude");
        }
    }),
    ("forced-dark", |s| {
        s.page = Page::Setup;
        s.settings.appearance = Some("dark".into());
    }),
    ("post-slack", |s| s.activity.insert(0, demo::slack_run())),
    ("post-slack-connected", |s| {
        s.activity.insert(0, demo::slack_run());
        s.settings.connections.push(places::Connection { place: places::Place::Slack, target: "#meeting-notes".into() });
    }),
    ("posted", |s| {
        let mut run = demo::slack_run();
        run.delivery = Some(activity::Delivery::Posted { to: "Slack (#meeting-notes)".into(), links: vec![] });
        s.activity.insert(0, run);
    }),
    ("connect-github", |s| {
        s.page = Page::Setup;
        s.settings.connections.push(places::Connection { place: places::Place::Slack, target: "#meeting-notes".into() });
        s.start_connect(places::Place::GitHub);
        s.form_target = rune::TextEdit::new("fabrikam/exporter");
    }),
    ("setup-german", |s| {
        s.page = Page::Setup;
        s.settings.language = Some("de".into());
    }),
    ("language-menu", |s| {
        s.page = Page::Setup;
        s.language_open = true;
    }),
    ("places-tested", |s| {
        s.page = Page::Setup;
        for (place, target) in [(places::Place::Slack, "#meeting-notes"), (places::Place::Notion, "https://www.notion.so/acme/Sales-calls-0123456789abcdef0123456789abcdef"), (places::Place::GitHub, "fabrikam/exporter")] {
            s.settings.connections.push(places::Connection { place, target: target.into() });
        }
        s.checks.insert(places::Place::Slack, Ok("The webhook works.".into()));
        s.checks.insert(places::Place::Notion, Err("Notion can't see that page. Share it with your integration.".into()));
        s.checking = Some(places::Place::GitHub);
    }),
    ("failed", |s| {
        let mut run = demo::summary_run();
        run.outcome = activity::Outcome::Failed;
        run.output = "opencode failed (exit 1).\n\nNo provider is configured.".into();
        s.activity.insert(0, run);
    }),
    ("summary-to", |s| {
        s.settings.connections.push(places::Connection { place: places::Place::Slack, target: "#meeting-notes".into() });
        s.settings.connections.push(places::Connection { place: places::Place::Notion, target: String::new() });
        s.quick.agent = agents::by_id("claude");
        s.quick.menu = Some("to");
    }),
    ("summary-agent", |s| s.quick.menu = Some("agent")),
    ("summary-set", |s| {
        s.settings.connections.push(places::Connection { place: places::Place::Slack, target: "#meeting-notes".into() });
        s.quick.agent = agents::by_id("claude");
        s.quick.to = Some(places::To::Place(places::Place::Slack));
        s.quick.focus = rune::TextEdit::new("Pricing and the EU data question");
    }),
    ("reports", |s| s.page = Page::Reports),
    ("edit-report", |s| {
        s.page = Page::Reports;
        s.edit_report("support-issues");
    }),
    ("edit-report-menu", |s| {
        s.page = Page::Reports;
        s.edit_report("customer-feedback");
        s.settings.connections.push(places::Connection { place: places::Place::Slack, target: "#meeting-notes".into() });
        s.settings.connections.push(places::Connection { place: places::Place::GitHub, target: "fabrikam/exporter".into() });
        if let Some(d) = s.editing.as_mut() {
            d.menu = Some("to");
        }
    }),
    ("edit-agent-menu", |s| {
        s.page = Page::Reports;
        s.edit_report("customer-feedback");
        if let Some(d) = s.editing.as_mut() {
            d.menu = Some("agent");
        }
    }),
    ("activity", |s| {
        s.page = Page::Activity;
        s.open_run = s.activity.first().map(|r| r.at_ms);
    }),
    ("empty", |s| {
        s.meetings.clear();
        s.selected = None;
        s.detail = None;
        s.activity.clear();
        s.meetings_on = Some(false);
        s.settings.language = Some("de".into());
        s.reports.clear();
    }),
    ("no-hex", |s| {
        s.hex = Some(HexStatus::Missing);
        s.meetings.clear();
        s.installed.clear();
    }),
];

fn main() {
    if !headless() {
        paths::adopt_login_path();
    }
    // The Dock icon, also when run with `cargo run` (no app bundle).
    if !headless()
        && let Err(e) = rune::set_app_icon(include_bytes!("../assets/icon.png").to_vec())
    {
        eprintln!("hex-meetings: couldn't set the Dock icon: {e}");
    }
    rune::run(AppSpec::<App> {
        scenes: SCENES,
        fonts: rune_theme_opencode::FONTS,
        families: rune_theme_opencode::FAMILIES,
        min_size: Some((820.0, 540.0)),
        setup: Some(|s, waker| s.start(waker)),
        poll: Some(|s, _, tx| s.poll(tx)),
        ..AppSpec::new("Hex Meetings", (1120.0, 740.0), init, ui::view)
    });
}
