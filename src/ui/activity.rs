//! Activity: every report run, newest first: which call, which report,
//! where it went, and what came back. Kept on this computer only.

use rune::{El, col, icon, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Markdown};
use rune_theme_opencode::icons as oc;

use super::{group, header, logos, outcome_tone, tag, tile};
use crate::activity::Run;
use crate::app::App;

/// Where a run went: its report's place, else the agent that ran it.
fn run_mark(s: &App, run: &Run) -> logos::Mark {
    if let Some(r) = s.reports.iter().chain(std::iter::once(crate::reports::builtin())).find(|r| r.name == run.report) {
        return super::reports::report_mark(s, r);
    }
    crate::agents::AGENTS
        .iter()
        .find(|a| a.program == run.program)
        .and_then(|a| logos::agent(a.id))
        .unwrap_or((&oc::STATUS, None))
}

fn run_row(s: &App, k: &Kit, run: &Run) -> El {
    let c = &k.c;
    let at = run.at_ms;
    let open = s.open_run == Some(at);
    let head = row()
        .key(format!("run-{at}"))
        .role("button")
        .label(format!("{} on {}", run.report, run.meeting_title))
        .expanded(open)
        .w_full()
        .gap(12.0)
        .px(14.0)
        .py(10.0)
        .center_y()
        .hover(c.hover)
        .on_press(move |s: &mut App| s.open_run = if s.open_run == Some(at) { None } else { Some(at) })
        .child(tile(k, run_mark(s, run)))
        .child(
            col()
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(2.0)
                .child(text(run.meeting_title.clone()).font(k.ty.ui_medium).color(c.text_base).truncate())
                .child(text(run.report.clone()).font(k.ty.small).color(c.text_muted).truncate()),
        )
        .children(run.awaits_post().then(|| tag(k, "Not posted yet", super::Tone::Quiet)))
        .children(run.auto.then(|| tag(k, "Auto", super::Tone::Quiet)))
        .child(tag(k, run.outcome.label(), outcome_tone(run.outcome)))
        .child(text(crate::time::long(run.at_ms)).font(k.ty.small).color(c.text_faint))
        .child(icon(if open { &oc::OUTLINE_CHEVRON_DOWN } else { &oc::OUTLINE_CHEVRON_RIGHT }, 14.0).color(c.icon_faint));
    if !open {
        return head;
    }
    col().w_full().child(head).child(
        col()
            .key(format!("run-{at}-detail"))
            .w_full()
            .gap(8.0)
            .pl(58.0)
            .pr(14.0)
            .pb(14.0)
            .child(where_it_went(k, run))
            .child(
                col()
                    .w_full()
                    .padding(12.0)
                    .radius(k.radius.md)
                    .fill(c.bg_base)
                    .border(1.0, c.border_weak)
                    .child(Markdown::new(run.readable()).selectable(true).build(k)),
            ),
    )
}

/// One line on where a run went, with a button for each thing it made.
fn where_it_went(k: &Kit, run: &Run) -> El {
    use crate::activity::Delivery;
    let c = &k.c;
    let words = match &run.delivery {
        Some(Delivery::Posted { to, .. }) => format!("Posted to {to}"),
        Some(Delivery::Failed { to, error }) => format!("Couldn't post to {to}. {error}"),
        Some(Delivery::Skipped) => "Not posted".to_string(),
        None if run.awaits_post() => "Waiting to be posted, on the call's page".to_string(),
        None => upper_first(&run.sends_to),
    };
    let mut line = row().w_full().gap(8.0).center_y().child(text(words).font(k.ty.small).color(c.text_muted).wrap().grow(1.0).shrink().min_w(0.0));
    if matches!(run.outcome, crate::activity::Outcome::Failed | crate::activity::Outcome::Stopped) {
        let at = run.at_ms;
        line = line.child(
            Button::new("Try again")
                .icon(&oc::REFRESH)
                .variant(ButtonVariant::Neutral)
                .control_size(rune_kit::ControlSize::Small)
                .help("Opens the call with this report, ready to run")
                .on_press(move |s: &mut App| s.retry(at))
                .build(k)
                .key("run-retry"),
        );
    }
    if let Some(Delivery::Posted { links, .. }) = &run.delivery {
        for (i, link) in links.iter().enumerate() {
            let url = link.clone();
            let label = if links.len() == 1 { "Open".to_string() } else { format!("Open {}", i + 1) };
            line = line.child(
                Button::new(label)
                    .icon(&oc::ARROW_UP_RIGHT)
                    .variant(ButtonVariant::Neutral)
                    .control_size(rune_kit::ControlSize::Small)
                    .help(link.clone())
                    .on_press(move |_: &mut App| crate::app::reveal(&url))
                    .build(k)
                    .key(format!("run-link-{i}")),
            );
        }
    }
    line
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

pub fn page(s: &App, k: &Kit, hair: f32) -> El {
    let c = &k.c;
    let mut list = col().key("activity-list").w_full().gap(8.0);
    if let Some(n) = &s.notice {
        list = list.child(super::notice(k, n, Some(|s: &mut App| s.notice = None)));
    }
    if s.activity.is_empty() {
        // The route a run takes: your agent, then where it went.
        let agent = s.agent().and_then(|a| logos::agent(a.id)).map(|m| super::mark(k, m, 18.0));
        let route = row()
            .gap(6.0)
            .center_y()
            .children(agent)
            .child(icon(&oc::ARROW_RIGHT, 12.0).color(c.icon_faint))
            .child(super::mark(k, logos::for_place(crate::places::Place::Slack), 18.0));
        let go = Button::new("Go to Meetings").variant(ButtonVariant::Neutral).on_press(|s: &mut App| s.go(crate::app::Page::Meetings)).build(k).key("go-meetings");
        list = list.child(super::empty_state(
            k,
            "activity-empty",
            super::ghost_rows(k, route),
            "Nothing sent yet",
            "Run a report on a call and it shows up here.",
            vec![go],
        ));
    } else {
        list = list.child(group(k, "runs", hair, s.activity.iter().map(|r| run_row(s, k, r)).collect()));
    }
    // Clear asks once more, in red, since it can't be undone.
    let actions = if s.activity.is_empty() {
        vec![]
    } else if s.confirm_clear {
        vec![
            Button::new("Cancel").variant(ButtonVariant::Neutral).on_press(|s: &mut App| s.confirm_clear = false).build(k).key("cancel-clear"),
            super::destructive_button(k, "Clear all", "clear-activity", App::clear_activity),
        ]
    } else {
        vec![Button::new("Clear").variant(ButtonVariant::Neutral).on_press(|s: &mut App| s.clear_activity()).build(k).key("clear-activity")]
    };
    super::page_shell("activity", header(k, "Activity", "Where each call went, and what came back. Kept on this Mac.", actions), list)
}
