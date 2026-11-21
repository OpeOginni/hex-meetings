//! Meetings: the calls Hex recorded, grouped by day, beside the open one:
//! its details, Hex's own summary, the recipe bar, the last recipe result,
//! and the transcript as speaker turns.

use rune::{El, ScrollSpec, col, el, icon, row, spacer, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Markdown, Spinner, TextInput};
use rune_theme_opencode::icons as oc;

use super::{Tone, header, notice, outcome_tone, tag};
use crate::app::{App, HexStatus};
use crate::hex::{self, Meeting};

const LIST_W: f32 = 300.0;

pub fn page(s: &App, k: &Kit, hair: f32) -> El {
    if s.hex == Some(HexStatus::Missing) {
        return empty(
            k,
            "Hex isn't installed",
            "Hex Meetings reads the calls Hex records. Install Hex from anoma.ly/hex, turn on Labs › Meetings, and record a call.",
        );
    }
    row()
        .key("meetings")
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(list(s, k))
        .child(el().w(hair).h_full().fill(k.c.border_weak))
        .child(detail(s, k))
}

fn empty(k: &Kit, title: &str, body: &str) -> El {
    col()
        .key("empty")
        .w_full()
        .h_full()
        .center()
        .gap(8.0)
        .padding(40.0)
        .window_drag()
        .child(icon(&oc::WAVEFORM, 28.0).color(k.c.icon_faint))
        .child(text(title).font(super::HEADING).color(k.c.text_strong))
        .child(text(body).font(k.ty.ui).color(k.c.text_muted).wrap().max_w(380.0).align(rune::Align::Center))
}

fn status_tag(k: &Kit, m: &Meeting) -> Option<El> {
    match m.status.as_str() {
        "complete" => None,
        "recording" | "starting" => Some(tag(k, "Recording", Tone::Busy)),
        "transcribing" => Some(tag(k, "Transcribing", Tone::Busy)),
        "interrupted" => Some(tag(k, "Interrupted", Tone::Quiet)),
        other => Some(tag(k, &capitalize(other), Tone::Bad)),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn list(s: &App, k: &Kit) -> El {
    let c = &k.c;
    let search = TextInput::new(&s.search)
        .placeholder("Search titles and apps")
        .leading_icon(&oc::MAGNIFYING_GLASS)
        .width(Some(LIST_W - 24.0))
        .on_edit(|s: &mut App| &mut s.search)
        .build(k)
        .key("search");
    let today = crate::time::today();
    let mut rows = col().key("rows").w_full().gap(1.0).pb(12.0);
    let visible = s.visible_meetings();
    let mut last_day = String::new();
    for m in &visible {
        let day = crate::time::day_label(m.timestamp_ms, today);
        if day != last_day {
            rows = rows.child(
                text(day.clone())
                    .key(format!("day-{day}"))
                    .role("heading")
                    .level(3)
                    .font(k.ty.small_medium)
                    .color(c.text_faint)
                    .px(10.0)
                    .pt(if last_day.is_empty() { 4.0 } else { 16.0 })
                    .pb(4.0),
            );
            last_day = day;
        }
        let selected = s.selected.as_deref() == Some(m.id.as_str());
        let id = m.id.clone();
        let meta = [m.application.clone(), Some(hex::duration(m.duration_ms))]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
        let mut title_row = row()
            .gap(6.0)
            .center_y()
            .child(text(m.title.clone()).font(k.ty.ui_medium).color(c.text_base).truncate().shrink().min_w(0.0));
        if let Some(t) = status_tag(k, m) {
            title_row = title_row.child(t);
        }
        rows = rows.child(
            row()
                .key(format!("meeting-{}", m.id))
                .role("button")
                .label(m.title.clone())
                .checked(selected)
                .w_full()
                .px(10.0)
                .py(7.0)
                .gap(8.0)
                .radius(6.0)
                .fill(if selected { c.layer_03 } else { rune::Color::TRANSPARENT })
                .hover(if selected { rune::Color::TRANSPARENT } else { c.hover })
                .on_press(move |s: &mut App| s.select(Some(id.clone())))
                .child(
                    col()
                        .grow(1.0)
                        .shrink()
                        .min_w(0.0)
                        .gap(2.0)
                        .child(title_row)
                        .child(text(meta).font(k.ty.small).color(c.text_muted).truncate()),
                )
                .child(text(crate::time::clock(m.timestamp_ms)).font(k.ty.small).color(c.text_faint)),
        );
    }
    let body = if !s.meetings_loaded {
        col().w_full().pt(24.0).center().child(Spinner::new().build(k))
    } else if let Some(e) = &s.list_error {
        col().w_full().px(10.0).pt(8.0).child(text(e.clone()).font(k.ty.small).color(c.danger.fg).wrap())
    } else if s.meetings.is_empty() {
        col().w_full().px(10.0).pt(8.0).child(
            text("No meetings yet. Turn on Labs › Meetings in Hex and record a call; it shows up here when it ends.")
                .font(k.ty.small)
                .color(c.text_muted)
                .wrap(),
        )
    } else if visible.is_empty() {
        col().w_full().px(10.0).pt(8.0).child(text("No meeting matches.").font(k.ty.small).color(c.text_muted))
    } else {
        rows
    };
    col()
        .key("list")
        .w(LIST_W)
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base)
        .child(row().w_full().h(40.0).window_drag())
        .child(row().px(12.0).pb(10.0).child(search))
        .child(col().key("list-scroll").w_full().grow(1.0).min_h(0.0).px(8.0).child(body).scroll(ScrollSpec { stick_bottom: false }))
}

fn detail(s: &App, k: &Kit) -> El {
    let shell = |content: El| {
        col()
            .key("detail")
            .grow(1.0)
            .shrink()
            .min_w(0.0)
            .h_full()
            .min_h(0.0)
            .child(row().w_full().h(40.0).window_drag())
            .child(col().key("detail-scroll").w_full().grow(1.0).min_h(0.0).px(32.0).pb(32.0).child(content).scroll(ScrollSpec { stick_bottom: false }))
    };
    let Some(m) = s.selected_meeting() else {
        if s.meetings.is_empty() {
            return shell(col());
        }
        return shell(col().pt(40.0).child(text("Choose a meeting.").font(k.ty.ui).color(k.c.text_muted)));
    };
    let c = &k.c;
    let mut meta = vec![crate::time::long(m.timestamp_ms), hex::duration(m.duration_ms)];
    if let Some(app) = &m.application {
        meta.insert(0, app.clone());
    }
    let mut body = col().key(format!("detail-{}", m.id)).w_full().gap(20.0);
    let copy = s.detail.as_ref().map(|d| hex::transcript_text(&d.entries)).unwrap_or_default();
    body = body.child(header(
        k,
        &m.title,
        &meta.join(" · "),
        vec![
            IconButton::new(&oc::OUTLINE_COPY)
                .variant(IconButtonVariant::Ghost)
                .label("Copy transcript")
                .help("Copy transcript")
                .disabled(copy.is_empty())
                .on_press_tx(move |_: &mut App, tx| tx.copy(copy.clone()))
                .build(k)
                .key("copy-transcript"),
        ],
    ));
    if let Some(n) = &s.notice {
        body = body.child(notice(k, n, Some(|s: &mut App| s.notice = None)));
    }
    for w in &m.warnings {
        body = body.child(text(format!("Hex: {w}")).font(k.ty.small).color(c.text_faint).wrap());
    }
    if let Some(notes) = s.detail.as_ref().and_then(|d| d.notes.as_ref()) {
        body = body.child(
            col()
                .key("hex-summary")
                .w_full()
                .gap(4.0)
                .child(text("Hex's summary").font(k.ty.small_medium).color(c.text_faint))
                .child(text(notes.summary.clone()).font(k.ty.body).color(c.text_base).wrap().selectable(true)),
        );
    }
    body = body.child(recipe_bar(s, k, m));
    if let Some(run) = s.last_run(&m.id) {
        body = body.child(result(k, run));
    }
    body = body.child(transcript(s, k));
    shell(body)
}

/// Recipes as buttons; a press asks where the call goes before anything runs.
fn recipe_bar(s: &App, k: &Kit, m: &Meeting) -> El {
    let c = &k.c;
    let card = col()
        .key("recipe-bar")
        .w_full()
        .gap(10.0)
        .padding(14.0)
        .radius(k.radius.lg)
        .fill(c.surface_raised_base)
        .border(1.0, c.border_weak);
    if let Some(r) = &s.running {
        let secs = r.started.elapsed().as_secs();
        let here = r.meeting_id == m.id;
        let label = if here {
            format!("Running {}… {}", r.recipe, hex::clock(secs * 1000))
        } else {
            format!("Running {} on another meeting… {}", r.recipe, hex::clock(secs * 1000))
        };
        return card.child(
            row()
                .gap(10.0)
                .center_y()
                .child(Spinner::new().size(14.0).build(k))
                .child(text(label).font(k.ty.ui).color(c.text_base).grow(1.0))
                .child(Button::new("Stop").variant(ButtonVariant::Neutral).on_press(|s: &mut App| s.stop_run()).build(k).key("stop")),
        );
    }
    if let Some(recipe) = s.armed.as_deref().and_then(|id| s.recipe(id)) {
        return card
            .child(text(format!("Run “{}” on this call?", recipe.name)).font(k.ty.ui_medium).color(c.text_strong))
            .child(
                row()
                    .gap(6.0)
                    .center_y()
                    .child(icon(&oc::SHIELD, 14.0).color(c.icon_muted))
                    .child(text(format!("The transcript goes to: {}", recipe.sends_to)).font(k.ty.ui).color(c.text_muted).wrap().shrink().min_w(0.0)),
            )
            .child(
                text(format!("Runs {}", recipe.command.first().map(String::as_str).unwrap_or("?")))
                    .font(k.ty.small)
                    .color(c.text_faint),
            )
            .child(
                row()
                    .gap(8.0)
                    .child(spacer())
                    .child(Button::new("Cancel").variant(ButtonVariant::Ghost).on_press(|s: &mut App| s.armed = None).build(k).key("cancel"))
                    .child(
                        Button::new("Send")
                            .variant(ButtonVariant::Contrast)
                            .disabled(s.detail.is_none())
                            .on_press(|s: &mut App| s.run_armed())
                            .build(k)
                            .key("send"),
                    ),
            );
    }
    let mut buttons = row().key("recipes").w_full().gap(8.0).flex_wrap();
    for r in &s.recipes {
        let id = r.id.clone();
        buttons = buttons.child(
            Button::new(r.name.clone())
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Small)
                .help(r.description.clone())
                .disabled(s.detail.is_none())
                .on_press(move |s: &mut App| s.armed = Some(id.clone()))
                .build(k)
                .key(format!("recipe-{}", r.id)),
        );
    }
    let hint = if s.recipes.is_empty() {
        "No recipes. Add one in the Recipes page."
    } else {
        "Hand this call to your own agent. You'll see where it goes before it's sent."
    };
    card.child(text("Send to").font(k.ty.small_medium).color(c.text_faint))
        .child(buttons)
        .child(text(hint).font(k.ty.small).color(c.text_faint).wrap())
}

fn result(k: &Kit, run: &crate::activity::Run) -> El {
    let c = &k.c;
    let output = run.output.clone();
    let when = crate::time::long(run.at_ms);
    col()
        .key("last-run")
        .w_full()
        .gap(10.0)
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(text(run.recipe.clone()).font(k.ty.ui_medium).color(c.text_strong))
                .child(tag(k, run.outcome.label(), outcome_tone(run.outcome)))
                .child(text(when).font(k.ty.small).color(c.text_faint).grow(1.0))
                .child(
                    IconButton::new(&oc::OUTLINE_COPY)
                        .variant(IconButtonVariant::Ghost)
                        .label("Copy result")
                        .help("Copy result")
                        .on_press_tx(move |_: &mut App, tx| tx.copy(output.clone()))
                        .build(k)
                        .key("copy-result"),
                ),
        )
        .child(
            col()
                .w_full()
                .padding(14.0)
                .radius(k.radius.lg)
                .fill(c.layer_01)
                .child(Markdown::new(if run.output.is_empty() { "(no output)".to_string() } else { run.output.clone() }).selectable(true).build(k)),
        )
}

fn transcript(s: &App, k: &Kit) -> El {
    let c = &k.c;
    let mut out = col().key("transcript").w_full().gap(14.0).child(text("Transcript").font(k.ty.small_medium).color(c.text_faint));
    if let Some(e) = &s.detail_error {
        return out.child(text(e.clone()).font(k.ty.ui).color(c.danger.fg).wrap());
    }
    let Some(d) = &s.detail else {
        return out.child(row().child(Spinner::new().size(14.0).build(k)));
    };
    if d.entries.is_empty() {
        return out.child(text("No transcript yet.").font(k.ty.ui).color(c.text_muted));
    }
    for (i, e) in d.entries.iter().enumerate() {
        let mine = e.source == "microphone";
        out = out.child(
            row()
                .key(format!("turn-{i}"))
                .w_full()
                .gap(14.0)
                .items_start()
                .child(
                    col()
                        .w(56.0)
                        .gap(1.0)
                        .child(text(e.speaker()).font(k.ty.small_medium).color(if mine { c.text_accent } else { c.text_base }))
                        .child(text(hex::clock(e.start_ms)).font(k.ty.text_xsmall).color(c.text_faint)),
                )
                .child(text(e.text.trim().to_string()).font(k.ty.body).color(c.text_base).wrap().selectable(true).grow(1.0).shrink().min_w(0.0)),
        );
    }
    out
}
