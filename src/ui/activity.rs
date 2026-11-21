//! Activity: every recipe run, newest first: which call, which recipe,
//! where it went, and what came back. Kept on this computer only.

use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Markdown};

use super::{header, outcome_tone, tag};
use crate::app::App;

pub fn page(s: &App, k: &Kit) -> El {
    let c = &k.c;
    let mut list = col().key("activity-list").w_full().gap(8.0);
    if let Some(n) = &s.notice {
        list = list.child(super::notice(k, n, Some(|s: &mut App| s.notice = None)));
    }
    if s.activity.is_empty() {
        list = list.child(text("Nothing sent yet. Runs appear here with where each call went.").font(k.ty.ui).color(c.text_muted));
    }
    for run in &s.activity {
        let at = run.at_ms;
        let open = s.open_run == Some(at);
        let mut card = col()
            .key(format!("run-{at}"))
            .w_full()
            .gap(6.0)
            .padding(14.0)
            .radius(k.radius.lg)
            .fill(c.surface_raised_base)
            .border(1.0, c.border_weak)
            .hover(c.surface_raised_hover)
            .role("button")
            .label(format!("{} on {}", run.recipe, run.meeting_title))
            .on_press(move |s: &mut App| s.open_run = if s.open_run == Some(at) { None } else { Some(at) })
            .child(
                row()
                    .gap(8.0)
                    .center_y()
                    .child(text(run.meeting_title.clone()).font(k.ty.ui_medium).color(c.text_strong).truncate().shrink().min_w(0.0))
                    .child(text("→").font(k.ty.ui).color(c.text_faint))
                    .child(text(run.recipe.clone()).font(k.ty.ui).color(c.text_base).truncate().shrink().min_w(0.0))
                    .child(tag(k, run.outcome.label(), outcome_tone(run.outcome)))
                    .child(rune::spacer())
                    .child(text(crate::time::long(run.at_ms)).font(k.ty.small).color(c.text_faint)),
            )
            .child(text(format!("Sent to: {} (ran {})", run.sends_to, run.program)).font(k.ty.small).color(c.text_muted).wrap());
        if open {
            card = card.child(
                col()
                    .w_full()
                    .mt(6.0)
                    .padding(12.0)
                    .radius(k.radius.md)
                    .fill(c.layer_01)
                    .child(Markdown::new(if run.output.is_empty() { "(no output)".to_string() } else { run.output.clone() }).selectable(true).build(k)),
            );
        }
        list = list.child(card);
    }
    let actions = vec![
        Button::new("Clear")
            .variant(ButtonVariant::Ghost)
            .disabled(s.activity.is_empty() || s.offline)
            .on_press(|s: &mut App| s.clear_activity())
            .build(k)
            .key("clear-activity"),
    ];
    col()
        .key("activity-page")
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(row().w_full().h(40.0).window_drag())
        .child(
            col()
                .key("activity-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .px(32.0)
                .pb(32.0)
                .child(
                    col()
                        .w_full()
                        .max_w(760.0)
                        .gap(20.0)
                        .child(header(k, "Activity", "Every call you've sent, where it went, and what came back. Stored only on this computer.", actions))
                        .child(list),
                )
                .scroll(ScrollSpec { stick_bottom: false }),
        )
}
