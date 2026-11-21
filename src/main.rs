//! Hex Meetings: an unofficial companion for Hex's Meetings (beta). It
//! reads the calls Hex recorded through Hex's public command line, shows
//! them, and hands a transcript to whatever agent the person already uses,
//! through recipes they control. Not affiliated with Hex or Anomaly.
//!
//! `cargo run` opens the window. `cargo run -- --snap out.png --scene demo`
//! renders a scene headless (`--list-scenes` lists them).

mod activity;
mod app;
mod demo;
mod hex;
mod paths;
mod recipes;
mod runner;
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
            recipe: "Sales call to Notion".into(),
            started: std::time::Instant::now() - std::time::Duration::from_secs(12),
            stop: Default::default(),
        })
    }),
    ("result", |s| s.activity.insert(0, demo::summary_run())),
    ("recipes", |s| s.page = Page::Recipes),
    ("activity", |s| {
        s.page = Page::Activity;
        s.open_run = s.activity.first().map(|r| r.at_ms);
    }),
    ("empty", |s| {
        s.meetings.clear();
        s.selected = None;
        s.detail = None;
        s.activity.clear();
    }),
    ("no-hex", |s| s.hex = Some(HexStatus::Missing)),
];

fn main() {
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
