//! Meetings: the calls Hex recorded, grouped by day, beside the open one:
//! its details, Hex's own summary, the report bar, the last report result,
//! what to do with the call afterwards, and the transcript as speaker turns.

use rune::{El, ScrollSpec, col, el, icon, row, spacer, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Markdown, Spinner, TextInput};
use rune_theme_opencode::icons as oc;

use super::{Tone, notice, outcome_tone, tag};
use crate::app::{App, HexStatus};
use crate::hex::{self, Meeting};

const LIST_W: f32 = 300.0;

pub fn page(s: &App, k: &Kit, hair: f32, layer: &mut Vec<El>) -> El {
    if s.hex == Some(HexStatus::Missing) || (s.meetings_loaded && s.meetings.is_empty() && s.list_error.is_none()) {
        return super::start::page(s, k, hair, layer);
    }
    row()
        .key("meetings")
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(list(s, k))
        .child(el().w(hair).h_full().fill(k.c.border_weak))
        .child(detail(s, k, layer))
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
    let mut search = TextInput::new(&s.search)
        .placeholder("Search titles and apps")
        .leading_icon(&oc::MAGNIFYING_GLASS)
        .width(Some(LIST_W - 24.0))
        .on_edit(|s: &mut App| &mut s.search);
    if !s.search.text.is_empty() {
        search = search.clear_button().on_action(|s: &mut App, _| s.search = rune::TextEdit::new(""));
    }
    let search = search
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
                        .child(
                            row()
                                .gap(5.0)
                                .center_y()
                                .children(m.application.as_deref().map(|app| super::mark(k, super::logos::call_app(app).unwrap_or((&super::logos::CALL, None)), 12.0)))
                                .child(text(meta).font(k.ty.small).color(c.text_muted).truncate().shrink().min_w(0.0)),
                        ),
                )
                .child(text(crate::time::clock(m.timestamp_ms)).font(k.ty.small).color(c.text_faint)),
        );
    }
    let body = if !s.meetings_loaded {
        col().w_full().pt(24.0).center().child(Spinner::new().build(k))
    } else if let Some(e) = &s.list_error {
        col().w_full().px(10.0).pt(8.0).child(text(e.clone()).font(k.ty.small).color(c.danger.fg).wrap())
    } else if visible.is_empty() {
        super::empty_small(
            k,
            "search-empty",
            &oc::MAGNIFYING_GLASS,
            &format!("No meeting matches \u{201c}{}\u{201d}", s.search.text.trim()),
            Some(
                Button::new("Clear search")
                    .variant(ButtonVariant::Neutral)
                    .control_size(ControlSize::Small)
                    .on_press(|s: &mut App| s.search = rune::TextEdit::new(""))
                    .build(k)
                    .key("clear-search"),
            ),
        )
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

fn detail(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let shell = |content: El| {
        col()
            .key("detail")
            .grow(1.0)
            .shrink()
            .min_w(0.0)
            .h_full()
            .min_h(0.0)
            .child(row().w_full().h(40.0).window_drag())
            .child(col().key("detail-scroll").w_full().grow(1.0).min_h(0.0).px(32.0).pt(4.0).pb(32.0).child(content).scroll(ScrollSpec { stick_bottom: false }))
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
    let app_logo = m.application.as_deref().and_then(super::logos::call_app).map(|mk| super::mark(k, mk, 16.0));
    body = body.child(super::header_with(
        k,
        &m.title,
        app_logo,
        &meta.join(" · "),
        vec![
            Button::new("Copy transcript")
                .icon(&oc::OUTLINE_COPY)
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Small)
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
    body = body.child(report_bar(s, k, m, layer));
    if let Some(run) = s.last_run(&m.id) {
        body = body.child(result(s, k, run));
        if run.outcome == crate::activity::Outcome::Done && !s.is_settled(&m.id) {
            body = body.child(afterwards(s, k, m));
        }
    }
    body = body.child(transcript(s, k));
    shell(body)
}

/// The Summary, set up for this call, and the saved reports as buttons; a
/// report asks where the call goes before anything runs.
fn report_bar(s: &App, k: &Kit, m: &Meeting, layer: &mut Vec<El>) -> El {
    let c = &k.c;
    let card = col()
        .key("report-bar")
        .w_full()
        .gap(10.0)
        .padding(14.0)
        .radius(k.radius.lg)
        .fill(c.surface_raised_base)
        .border(1.0, c.border_weak);
    if let Some(r) = &s.running {
        let secs = r.started.elapsed().as_secs();
        let agent = r.agent;
        let elsewhere = (r.meeting_id != m.id).then(|| text("on another call").font(k.ty.small).color(c.text_faint));
        return card.child(
            row()
                .gap(10.0)
                .center_y()
                .child(Spinner::new().size(14.0).build(k))
                .children(agent.and_then(|id| super::agent_logo(k, id, 16.0)))
                .child(text(r.report.clone()).font(k.ty.ui_medium).color(c.text_base))
                .children(elsewhere)
                .child(spacer())
                .child(text(hex::clock(secs * 1000)).font(k.ty.small).color(c.text_faint))
                .child(Button::new("Stop").variant(ButtonVariant::Neutral).on_press(|s: &mut App| s.stop_run()).build(k).key("stop")),
        );
    }
    if let Some(report) = s.armed.as_deref().and_then(|id| s.report(id)) {
        let plan = s.plan(report);
        let mut card = card.child(text(format!("Run “{}” on this call?", report.name)).font(k.ty.ui_medium).color(c.text_strong));
        card = match &plan {
            Ok(plan) => {
                let mut runs = format!("Runs {}", plan.runs);
                if plan.tools {
                    runs.push_str(", which may use its connected tools without asking you");
                }
                card.child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(icon(&oc::SHIELD, 14.0).color(c.icon_muted))
                        .child(text(format!("The transcript goes to {}.", lower_first(&plan.sends_to))).font(k.ty.ui).color(c.text_muted).wrap().shrink().min_w(0.0)),
                )
                .child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .children(plan.agent.and_then(|id| super::agent_logo(k, id, 14.0)))
                        .child(text(runs).font(k.ty.small).color(c.text_faint).wrap().shrink().min_w(0.0)),
                )
            }
            Err(e) => card.child(text(e.clone()).font(k.ty.ui).color(c.danger.fg).wrap()),
        };
        card = match report.to.filter(|to| s.connection(*to).is_none()) {
            Some(to) => card.child(
                row()
                    .gap(6.0)
                    .center_y()
                    .child(icon(&oc::WARNING, 14.0).color(c.warning.icon))
                    .child(text(format!("{} isn't connected yet. Connect it in Setup before posting.", to.name())).font(k.ty.small).color(c.text_muted).wrap().shrink().min_w(0.0)),
            ),
            None => card,
        };
        return card.child(
            row()
                .gap(8.0)
                .child(spacer())
                .child(Button::new("Cancel").variant(ButtonVariant::Neutral).on_press(|s: &mut App| s.armed = None).build(k).key("cancel"))
                .child(
                    Button::new("Run")
                        .variant(ButtonVariant::Submit)
                        .disabled(s.detail.is_none() || plan.is_err())
                        .on_press(|s: &mut App| s.run_armed())
                        .build(k)
                        .key("send"),
                ),
        );
    }
    let mut buttons = row().key("reports").w_full().gap(8.0).flex_wrap();
    for r in &s.reports {
        let id = r.id.clone();
        buttons = buttons.child(
            Button::new(r.name.clone())
                .icon(super::reports::report_mark(s, r).0)
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Small)
                .help(r.description.clone())
                .disabled(s.detail.is_none())
                .on_press(move |s: &mut App| s.armed = Some(id.clone()))
                .build(k)
                .key(format!("report-{}", r.id)),
        );
    }
    buttons = buttons.child(
        Button::new("New report\u{2026}")
            .icon(&oc::PLUS_SMALL)
            .variant(ButtonVariant::Neutral)
            .control_size(ControlSize::Small)
            .help("Make a report: what to look for, and where it goes")
            .on_press(|s: &mut App| {
                s.go(crate::app::Page::Reports);
                s.new_report();
            })
            .build(k)
            .key("new-report-from-call"),
    );
    // Only a new user needs telling what a report is for.
    let hint = s.reports.is_empty().then(|| text("Save a focus and where it goes as a report, to run it on any call.").font(k.ty.small).color(c.text_faint).wrap());
    card.child(quick(s, k, layer))
        .child(el().w_full().h(1.0).my(4.0).fill(c.border_weak))
        .child(text("Reports").font(k.ty.small_medium).color(c.text_faint))
        .child(buttons)
        .children(hint)
}

fn quick_toggle(s: &mut App, menu: &'static str) {
    s.quick.menu = if s.quick.menu == Some(menu) { None } else { Some(menu) };
}

/// The Summary for this call: which agent writes it, where it goes, and
/// anything to look out for. Where the transcript goes shows beside the
/// button that sends it.
fn quick(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let c = &k.c;
    let q = &s.quick;
    let report = s.quick_report();
    let plan = s.plan(&report);
    let agent = super::reports::agent_menu(s, k, "quick-agent", q.agent, q.menu == Some("agent"), |s| quick_toggle(s, "agent"), |s| s.quick.menu = None, |s, a| {
        s.quick.agent = a;
        s.quick.menu = None;
    })
    .build(k, layer);
    let label = q.to.map_or("On the call".to_string(), |to| to.name().to_string());
    let to = super::reports::places_menu(
        s,
        k,
        "quick-to",
        super::reports::report_mark(s, &report),
        label,
        q.to,
        true,
        None,
        q.menu == Some("to"),
        |s| quick_toggle(s, "to"),
        |s| s.quick.menu = None,
        |s, to| {
            s.quick.to = to;
            s.quick.menu = None;
        },
    )
    .item(super::reports::connect_item(k))
    .build(k, layer);
    let focus = TextInput::new(&q.focus)
        .placeholder("Anything to look out for? (optional)")
        .width(None)
        .on_edit(|s: &mut App| &mut s.quick.focus)
        .build(k)
        .key("quick-focus");
    let route = match &plan {
        Ok(plan) => row()
            .gap(6.0)
            .center_y()
            .grow(1.0)
            .shrink()
            .min_w(0.0)
            .child(icon(&oc::SHIELD, 14.0).color(c.icon_muted))
            .child(text(format!("The transcript goes to {}.", lower_first(&plan.sends_to))).font(k.ty.small).color(c.text_muted).wrap().shrink().min_w(0.0)),
        Err(e) => row().grow(1.0).shrink().min_w(0.0).child(text(e.clone()).font(k.ty.small).color(c.danger.fg).wrap()),
    };
    col()
        .key("quick")
        .w_full()
        .gap(10.0)
        .child(text("Summary").font(k.ty.ui_medium).color(c.text_strong))
        .child(row().gap(8.0).flex_wrap().center_y().child(agent).child(to))
        .child(focus)
        .child(
            row().w_full().gap(12.0).center_y().child(route).child(
                Button::new("Summarize")
                    .variant(ButtonVariant::Submit)
                    .disabled(s.detail.is_none() || plan.is_err())
                    .on_press(App::summarize)
                    .build(k)
                    .key("summarize"),
            ),
        )
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_lowercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn result(s: &App, k: &Kit, run: &crate::activity::Run) -> El {
    let c = &k.c;
    let output = run.output.clone();
    let when = crate::time::long(run.at_ms);
    let at = run.at_ms;
    let mut out = col()
        .key("last-run")
        .w_full()
        .gap(10.0)
        .child(
            row()
                .gap(8.0)
                .center_y()
                .children(crate::agents::AGENTS.iter().find(|a| a.program == run.program).and_then(|a| super::agent_logo(k, a.id, 16.0)))
                .child(text(run.report.clone()).font(k.ty.ui_medium).color(c.text_strong))
                .child(if run.awaits_post() { tag(k, "Ready to post", Tone::Busy) } else { tag(k, run.outcome.label(), outcome_tone(run.outcome)) })
                .child(text(when).font(k.ty.small).color(c.text_faint).grow(1.0))
                .children(matches!(run.outcome, crate::activity::Outcome::Failed | crate::activity::Outcome::Stopped).then(|| {
                    Button::new("Try again")
                        .icon(&oc::REFRESH)
                        .variant(ButtonVariant::Submit)
                        .control_size(ControlSize::Small)
                        .on_press(move |s: &mut App| s.retry(at))
                        .build(k)
                        .key("retry-run")
                }))
                // A failed run's text is the error, not a result to keep.
                .children((run.outcome != crate::activity::Outcome::Failed).then(|| {
                    Button::new("Save")
                        .icon(&oc::DOWNLOAD)
                        .variant(ButtonVariant::Neutral)
                        .control_size(ControlSize::Small)
                        .help(format!("Save as Markdown in {}", crate::paths::tilde(&crate::paths::summaries_dir())))
                        .disabled(run.output.is_empty())
                        .on_press(move |s: &mut App| s.save_result(at))
                        .build(k)
                        .key("save-result")
                }))
                .children((run.outcome != crate::activity::Outcome::Failed).then(|| {
                    Button::new("Copy")
                        .icon(&oc::OUTLINE_COPY)
                        .variant(ButtonVariant::Neutral)
                        .control_size(ControlSize::Small)
                        .on_press_tx(move |_: &mut App, tx| tx.copy(output.clone()))
                        .build(k)
                        .key("copy-result")
                })),
        )
        .child(match delivery(s, k, run) {
            // A result to post shows as what will be posted, not as the raw reply.
            Some(panel) => panel,
            None => col()
                .w_full()
                .padding(14.0)
                .radius(k.radius.lg)
                .fill(c.layer_01)
                .child(Markdown::new(if run.output.is_empty() { "(no output)".to_string() } else { run.output.clone() }).selectable(true).build(k)),
        });
    if let Some((_, path)) = s.saved.as_ref().filter(|(saved_at, _)| *saved_at == at) {
        let file = path.clone();
        out = out.child(
            row()
                .key("saved")
                .gap(8.0)
                .center_y()
                .child(icon(&oc::CHECK_SMALL, 14.0).color(c.success.icon))
                .child(
                    text(format!("Saved to {}", path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default()))
                        .font(k.ty.small)
                        .color(c.text_muted)
                        .help(crate::paths::tilde(path))
                        .truncate()
                        .shrink()
                        .min_w(0.0),
                )
                .child(
                    Button::new("Show in Finder")
                        .icon(&oc::FOLDER)
                        .variant(ButtonVariant::Neutral)
                        .control_size(ControlSize::Small)
                        .on_press(move |_: &mut App| crate::app::reveal_file(&file))
                        .build(k)
                        .key("show-saved"),
                ),
        );
    }
    out
}

/// Slack's own formatting (*bold*, _italic_) as Markdown, so the preview
/// reads as the message will in Slack.
fn slack_to_markdown(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        // Slack's *bold* is Markdown's **bold**; an odd * is left as typed.
        let line = if line.matches('*').count() % 2 == 0 { line.replace('*', "**") } else { line.to_string() };
        let line = line.strip_prefix("\u{2022} ").map_or(line.clone(), |rest| format!("- {rest}"));
        out.push_str(&line);
        // Each Slack line is its own line.
        out.push_str("  \n");
    }
    out
}

#[cfg(test)]
#[test]
fn slack_formatting_reads_as_markdown() {
    assert_eq!(slack_to_markdown("*Call* (34 min)\n\u{2022} You: send *it*\n2 * 3"), "**Call** (34 min)  \n- You: send **it**  \n2 * 3  \n");
}

/// What a result will post, as it will read there: the Slack message, the
/// Notion page, or the issues.
fn preview(k: &Kit, to: crate::places::To, output: &str) -> El {
    use crate::places::{Place, To, parse_issues};
    let c = &k.c;
    match to {
        To::Place(Place::Slack) => Markdown::new(slack_to_markdown(output.trim())).selectable(true).build(k),
        To::Place(Place::Notion) => Markdown::new(output.trim().to_string()).selectable(true).build(k),
        _ => match parse_issues(output) {
            Ok(issues) if issues.is_empty() => text("The agent found nothing to file.").font(k.ty.ui).color(c.text_muted),
            Ok(issues) => {
                let mut list = col().w_full().gap(10.0);
                for issue in issues {
                    list = list.child(
                        col()
                            .w_full()
                            .gap(2.0)
                            .child(text(issue.title).font(k.ty.ui_medium).color(c.text_base).wrap())
                            .child(text(issue.body).font(k.ty.small.lh(18.0)).color(c.text_muted).wrap().selectable(true)),
                    );
                }
                list
            }
            Err(e) => text(e).font(k.ty.ui).color(c.danger.fg).wrap(),
        },
    }
}

/// A finished result that Hex Meetings posts: what it will post, then Post /
/// Don't post, or what happened. None for a result that only shows on the call.
fn delivery(s: &App, k: &Kit, run: &crate::activity::Run) -> Option<El> {
    use crate::activity::{Delivery, Outcome};
    use crate::places::To;
    let c = &k.c;
    let to = run.to.as_deref().and_then(To::parse).filter(|_| run.outcome == Outcome::Done)?;
    let at = run.at_ms;
    let small = |label: &str, key: &str, variant: ButtonVariant, f: fn(&mut App, i64)| {
        Button::new(label.to_string()).variant(variant).control_size(ControlSize::Small).on_press(move |s: &mut App| f(s, at)).build(k).key(key.to_string())
    };
    let place_logo = s.connection(to).map(|c| super::logos::for_place(c.place)).or(match to {
        To::Place(p) => Some(super::logos::for_place(p)),
        To::Issues => None,
    });
    let status = |glyph: Option<El>, words: String| {
        row().gap(8.0).center_y().grow(1.0).shrink().min_w(0.0).children(glyph).child(text(words).font(k.ty.small).color(c.text_muted).wrap().shrink().min_w(0.0))
    };
    let logo = || place_logo.map(|m| super::mark(k, m, 14.0));
    let bar = match &run.delivery {
        Some(Delivery::Posted { to, links }) => {
            let mut bar = row().w_full().gap(8.0).center_y().child(status(Some(icon(&oc::CHECK_SMALL, 14.0).color(c.success.icon)), format!("Posted to {to}")));
            for (i, link) in links.iter().enumerate() {
                let url = link.clone();
                let label = if links.len() == 1 { "Open".to_string() } else { format!("Open {}", i + 1) };
                bar = bar.child(
                    Button::new(label)
                        .icon(&oc::ARROW_UP_RIGHT)
                        .variant(ButtonVariant::Neutral)
                        .control_size(ControlSize::Small)
                        .help(link.clone())
                        .on_press(move |_: &mut App| crate::app::reveal(&url))
                        .build(k)
                        .key(format!("open-link-{i}")),
                );
            }
            bar
        }
        Some(Delivery::Skipped) => row().w_full().child(status(None, "Not posted.".into())),
        Some(Delivery::Failed { to, error }) => row()
            .w_full()
            .gap(8.0)
            .center_y()
            .child(status(Some(icon(&oc::WARNING, 14.0).color(c.warning.icon)), format!("Couldn't post to {to}. {error}")))
            .child(small("Don't post", "skip-post", ButtonVariant::Neutral, App::skip_post))
            .child(small("Try again", "retry-post", ButtonVariant::Submit, App::retry_post)),
        None if s.posting == Some(at) => row().w_full().gap(8.0).center_y().child(Spinner::new().size(14.0).build(k)).child(status(None, "Posting\u{2026}".into())),
        None => match s.connection(to) {
            Some(conn) => row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(status(logo(), format!("Goes to {}", conn.label())))
                .child(small("Don't post", "skip-post", ButtonVariant::Neutral, App::skip_post))
                .child(small("Post", "post", ButtonVariant::Submit, App::post_result)),
            None => row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(status(logo(), format!("Connect {} to post this.", to.name())))
                .child(small("Don't post", "skip-post", ButtonVariant::Neutral, App::skip_post))
                .child(
                    Button::new("Open Setup")
                        .variant(ButtonVariant::Submit)
                        .control_size(ControlSize::Small)
                        .on_press(|s: &mut App| s.go(crate::app::Page::Setup))
                        .build(k)
                        .key("open-setup"),
                ),
        },
    };
    Some(col().key("delivery").w_full().gap(14.0).padding(14.0).radius(k.radius.lg).fill(c.layer_01).child(preview(k, to, &run.output)).child(bar))
}

/// After a result: keep the call in Hex until its own time is up, or delete it in Hex now.
fn afterwards(s: &App, k: &Kit, m: &Meeting) -> El {
    let c = &k.c;
    let small = |label: &str, key: &str, f: Box<dyn Fn(&mut App)>| {
        Button::new(label.to_string()).variant(ButtonVariant::Neutral).control_size(ControlSize::Small).on_press(f).build(k).key(key.to_string())
    };
    let (keep_id, del_id) = (m.id.clone(), m.id.clone());
    let keep = move |s: &mut App| s.keep_call(&keep_id);
    let line = if s.deleting.as_deref() == Some(m.id.as_str()) {
        "Hex is open at Meetings: delete this call there.".to_string()
    } else {
        match &s.overview {
            Some(Ok(o)) => format!("Done with this call? {}", hex::retention_sentence(o.retention())),
            _ => "Done with this call? Hex keeps it as its settings say.".to_string(),
        }
    };
    let mut actions = row().gap(8.0);
    actions = if s.deleting.as_deref() == Some(m.id.as_str()) {
        actions.child(small("Cancel", "keep", Box::new(keep)))
    } else {
        actions.child(small("Keep", "keep", Box::new(keep))).child(small("Delete in Hex\u{2026}", "delete-in-hex", Box::new(move |s: &mut App| s.delete_in_hex(&del_id))))
    };
    row()
        .key("afterwards")
        .w_full()
        .gap(12.0)
        .px(14.0)
        .py(10.0)
        .center_y()
        .radius(k.radius.lg)
        .border(1.0, c.border_weak)
        .child(icon(&oc::ARCHIVE, 16.0).color(c.icon_muted))
        .child(text(line).font(k.ty.small.lh(18.0)).color(c.text_muted).wrap().grow(1.0).shrink().min_w(0.0))
        .child(actions)
}

/// Who's speaking: your microphone, or the people on the call, shown with
/// the call app's logo when Hex knows it.
fn avatar(k: &Kit, mine: bool, app: Option<&str>) -> El {
    let c = &k.c;
    let dot = el().size(28.0, 28.0).radius(14.0).center();
    if mine {
        return dot.fill(c.info.bg).child(icon(&oc::WAVEFORM, 14.0).color(c.info.icon));
    }
    match app.and_then(super::logos::call_app) {
        Some(logo) => dot.fill(c.bg_base).border(1.0, c.border_weak).child(super::mark(k, logo, 15.0)),
        None => dot.fill(c.layer_02).child(icon(&oc::SPEECH_BUBBLE, 14.0).color(c.icon_muted)),
    }
}

fn transcript(s: &App, k: &Kit) -> El {
    let c = &k.c;
    let mut out = col().key("transcript").w_full().gap(20.0).child(text("Transcript").font(k.ty.small_medium).color(c.text_faint));
    if let Some(e) = &s.detail_error {
        return out.child(text(e.clone()).font(k.ty.ui).color(c.danger.fg).wrap());
    }
    let Some(d) = &s.detail else {
        return out.child(row().child(Spinner::new().size(14.0).build(k)));
    };
    if d.entries.is_empty() {
        let still = d.meeting.in_progress || matches!(d.meeting.status.as_str(), "recording" | "starting" | "transcribing");
        let line = if still { "Hex is still transcribing this call. It shows up here as it does." } else { "Hex didn't hear anything in this call." };
        return out.child(super::empty_small(k, "transcript-empty", &oc::WAVEFORM, line, None));
    }
    // One block per change of speaker: the avatar and name once, then each
    // thing they said as its own paragraph (its time shows on hover).
    let app = s.selected_meeting().and_then(|m| m.application.as_deref());
    for (i, turns) in d.entries.chunk_by(|a, b| a.source == b.source).enumerate() {
        let first = &turns[0];
        let mine = first.source == "microphone";
        let mut said = col().grow(1.0).shrink().min_w(0.0).gap(6.0).child(
            row()
                .gap(8.0)
                .center_y()
                .child(text(first.speaker()).font(k.ty.ui_medium).color(if mine { c.text_accent } else { c.text_strong }))
                .child(text(hex::clock(first.start_ms)).font(k.ty.text_xsmall).color(c.text_faint)),
        );
        for e in turns {
            said = said.child(text(e.text.trim().to_string()).font(k.ty.body).color(c.text_base).wrap().selectable(true).help(hex::clock(e.start_ms)));
        }
        out = out.child(row().key(format!("turn-{i}")).w_full().gap(12.0).items_start().child(avatar(k, mine, app)).child(said));
    }
    out
}
