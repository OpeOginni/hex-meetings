//! Get started: what's left before the first call shows up, in order.
//! Shown on the Meetings page until Hex has recorded a call. Each step
//! checks itself off from what the app can see: Hex found, Labs › Meetings
//! on, an agent installed.

use rune::{El, col, el, icon, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};
use rune_theme_opencode::icons as oc;

use super::{group, list_row};
use crate::app::{self, App, HexStatus, Page};
use crate::hex;

/// A step's number, or a check once it's done.
#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Done,
    /// The next step to do: as strong as its button.
    Next,
    Later,
}

/// A step's marker: a solid green check when done, a solid dark number for
/// the next step, an outlined number for the rest.
fn marker(k: &Kit, n: usize, mark: Mark) -> El {
    let c = &k.c;
    let dot = el().size(30.0, 30.0).radius(15.0).center();
    match mark {
        // One green in both themes, deep enough for a white check.
        Mark::Done => dot.fill(super::logos::srgb(0x2E9E5B)).child(icon(&oc::CHECK, 16.0).color(rune::Color::WHITE)),
        // The same fill as the step's primary button, in both themes.
        Mark::Next => dot.fill(c.bg_icon_button_contrast).child(text(n.to_string()).font(k.ty.ui_medium).color(c.submit_fg)),
        Mark::Later => dot.fill(c.bg_base).border(1.0, c.border_base).child(text(n.to_string()).font(k.ty.ui_medium).color(c.text_muted)),
    }
}

struct Step {
    title: &'static str,
    about: String,
    done: bool,
    /// Shown until the step is done.
    action: Option<Button>,
    /// Shown always, such as a setting the step depends on.
    control: Option<El>,
    /// Nice to do, never in the way: it doesn't become the next step.
    optional: bool,
}

pub fn page(s: &App, k: &Kit, hair: f32, layer: &mut Vec<El>) -> El {
    let hex_found = matches!(s.hex, Some(HexStatus::Found { .. }));
    let version = match &s.hex {
        Some(HexStatus::Found { version: Some(v), .. }) => format!("Hex {v} is installed."),
        _ => "Hex is installed.".into(),
    };
    let agent = s.agent();
    let (speech_fits, speech_about) = super::speech_status(s);
    let steps = [
        Step {
            title: "Install Hex",
            about: if hex_found { version } else { "For Apple silicon Macs on macOS 15 or later. This page updates once it's installed.".into() },
            done: hex_found,
            action: Some(Button::new("Download Hex").on_press(|_: &mut App| app::reveal(hex::DOWNLOAD_PAGE))),
            control: None,
            optional: false,
        },
        Step {
            title: "Turn on Meetings in Hex",
            about: "Hex then asks for the microphone and system audio.".into(),
            done: hex_found && s.meetings_on == Some(true),
            action: Some(
                Button::new(if s.turning_on { "Turning on…" } else { "Turn on" })
                    .disabled(!hex_found || s.turning_on)
                    .on_press(|s: &mut App| s.turn_on_meetings()),
            ),
            control: None,
            optional: false,
        },
        Step {
            title: "Match the speech model to your language",
            about: if hex_found { speech_about } else { "Hex's speech models show here once it's installed.".into() },
            done: hex_found && speech_fits,
            action: hex_found.then(|| Button::new("Choose in Hex").on_press(|s: &mut App| s.open_in_hex(true))),
            control: hex_found.then(|| super::language_select(s, k, layer)),
            optional: false,
        },
        Step {
            title: "Pick an agent",
            about: match agent {
                Some(a) => format!("{} writes your results.", a.name),
                None => "OpenCode, Claude Code, Codex, Cursor or Pi. Setup shows how to install one.".into(),
            },
            done: agent.is_some(),
            action: Some(Button::new("Open Setup").on_press(|s: &mut App| s.go(Page::Setup))),
            control: None,
            optional: false,
        },
        Step {
            title: "Make a report (optional)",
            about: "Every call gets a Summary. A report sends calls where your team works, such as Slack or Notion.".into(),
            done: !s.reports.is_empty(),
            action: Some(Button::new("New report").on_press(|s: &mut App| {
                s.go(Page::Reports);
                s.new_report();
            })),
            control: None,
            optional: true,
        },
        Step {
            title: "Record a call",
            about: "Join a call and Hex offers to record it. It shows up here when it ends.".into(),
            done: false,
            action: None,
            control: None,
            optional: false,
        },
    ];
    // The first step that isn't done gets the strong button.
    let current = steps.iter().position(|st| !st.done && !st.optional);
    let rows = steps
        .into_iter()
        .enumerate()
        .map(|(i, st)| {
            let action = st.action.filter(|_| !st.done).map(|b| {
                let variant = if Some(i) == current { ButtonVariant::Submit } else { ButtonVariant::Neutral };
                b.variant(variant).control_size(ControlSize::Small).build(k).key(format!("step-{i}-action"))
            });
            let trailing = st.control.into_iter().chain(action).collect();
            // The check is enough to show a step is done; fading the row would fade its control too.
            let mark = if st.done {
                Mark::Done
            } else if Some(i) == current {
                Mark::Next
            } else {
                Mark::Later
            };
            list_row(k, format!("step-{i}"), marker(k, i + 1, mark), st.title, Some(st.about), trailing)
        })
        .collect();
    let mut content = col().w_full().gap(20.0);
    if let Some(n) = &s.notice {
        content = content.child(super::notice(k, n, Some(|s: &mut App| s.notice = None)));
    }
    content = content.child(group(k, "steps", hair, rows));
    // The same frame as every other page, so titles line up across the app.
    super::page_shell("start", super::header(k, "Get started", "A few steps before your first call shows up here.", vec![]), content)
}
