//! About: what Hex Meetings is, and the credit owed to Hex and Anomaly,
//! whose app does the recording and transcribing this one builds on.

use rune::{El, col, icon, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};
use rune_theme_opencode::icons as oc;

use super::{Tone, header, logos, tag};
use crate::app::{self, App};

/// What this app is, and the credit owed to Hex and Anomaly.
fn card(k: &Kit) -> El {
    let c = &k.c;
    let link = |label: &str, key: &str, url: &'static str| {
        Button::new(label.to_string())
            .icon(&oc::ARROW_UP_RIGHT)
            .variant(ButtonVariant::Neutral)
            .control_size(ControlSize::Small)
            .help(url)
            .on_press(move |_: &mut App| app::reveal(url))
            .build(k)
            .key(key.to_string())
    };
    let para = |words: &str| text(words.to_string()).font(k.ty.ui).color(c.text_muted).wrap();
    col()
        .key("about")
        .w_full()
        .gap(14.0)
        .padding(18.0)
        .radius(k.radius.lg)
        .fill(c.surface_raised_base)
        .border(1.0, c.border_weak)
        .child(
            row()
                .gap(14.0)
                .center_y()
                .child(icon(&logos::APP, 56.0).a11y_hidden())
                .child(
                    col()
                        .gap(2.0)
                        .child(text("Hex Meetings").font(super::HEADING).color(c.text_strong))
                        .child(text(format!("Version {}. An unofficial companion for Hex.", env!("CARGO_PKG_VERSION"))).font(k.ty.small).color(c.text_muted)),
                ),
        )
        .child(para(
            "Hex does the real work: it records your calls, transcribes them on your Mac with its own speech models, and keeps them. \
             Hex Meetings only reads those meetings through Hex's public command line and hands them to the agent you choose.",
        ))
        .child(para("Thank you to Anomaly for building Hex, and Rune, the toolkit this app is made with, in the open."))
        .child(
            text("Not made, endorsed or supported by Anomaly. Hex and the other names and logos shown here belong to their owners.")
                .font(k.ty.small)
                .color(c.text_faint)
                .wrap(),
        )
        .child(
            row()
                .gap(8.0)
                .flex_wrap()
                .child(link("Hex", "about-hex", "https://anoma.ly/hex"))
                .child(link("Hex on GitHub", "about-hex-github", "https://github.com/anomalyco/hex"))
                .child(link("Rune", "about-rune", "https://github.com/anomalyco/rune")),
        )
}

/// The call apps Hex 3.4 offers to record, as Hex itself lists them. Hex's
/// command line doesn't print this list, so it's copied here; check it when
/// Hex updates.
const CALL_APPS: &[&str] = &["Zoom", "Google Meet", "Microsoft Teams", "Slack huddles", "FaceTime", "WhatsApp", "Discord", "Gather"];
/// With Labs › More call apps.
const MORE_CALL_APPS: &[&str] = &["Signal", "Telegram", "WeChat", "LINE", "Webex", "Element", "Phone"];

/// An app as a small chip: its logo (or a plain call icon) and its name.
fn chip(k: &Kit, name: &str) -> El {
    let c = &k.c;
    let logo = match logos::call_app(name) {
        Some(m) => super::mark(k, m, 16.0),
        None => icon(&logos::CALL, 16.0).color(c.icon_muted),
    };
    row()
        .key(format!("app-{name}"))
        .gap(8.0)
        .h(28.0)
        .pr(14.0)
        .center_y()
        .child(logo)
        .child(text(name.to_string()).font(k.ty.ui).color(c.text_base))
}

fn chips(k: &Kit, names: &[&str]) -> El {
    row().w_full().gap(8.0).flex_wrap().children(names.iter().map(|n| chip(k, n)))
}

/// Which calls Hex offers to record, so nobody has to open Hex to find out.
fn calls(s: &App, k: &Kit) -> El {
    let c = &k.c;
    let more = s.overview.as_ref().and_then(|o| o.as_ref().ok()).and_then(|o| o.more_call_apps());
    let more_tag = match more {
        Some(true) => Some(tag(k, "On", Tone::Good)),
        Some(false) => Some(tag(k, "Off", Tone::Quiet)),
        None => None,
    };
    let heading = |words: &str| text(words.to_string()).font(k.ty.small_medium).color(c.text_faint);
    let hex_found = matches!(s.hex, Some(crate::app::HexStatus::Found { .. }));
    let button = |label: &str, key: &str, help: &str, f: fn(&mut App)| {
        Button::new(label.to_string())
            .icon(&oc::ARROW_UP_RIGHT)
            .variant(ButtonVariant::Neutral)
            .control_size(ControlSize::Small)
            .help(help.to_string())
            .disabled(!hex_found)
            .on_press(f)
            .build(k)
            .key(key.to_string())
    };
    col()
        .key("calls")
        .w_full()
        .gap(12.0)
        .padding(18.0)
        .radius(k.radius.lg)
        .fill(c.surface_raised_base)
        .border(1.0, c.border_weak)
        .child(
            row()
                .w_full()
                .gap(12.0)
                .items_start()
                .child(
                    col()
                        .grow(1.0)
                        .shrink()
                        .min_w(0.0)
                        .gap(2.0)
                        .child(text("Calls Hex records").font(super::HEADING).color(c.text_strong))
                        .child(
                            text("Hex offers to record calls in these apps. Recorded calls show up in Meetings.")
                                .font(k.ty.small)
                                .color(c.text_muted)
                                .wrap(),
                        ),
                )
                .child(button("Choose apps in Hex", "choose-apps", "Ask, always record, or never offer, for each app", |s| s.open_in_hex(false))),
        )
        .child(heading("Always"))
        .child(chips(k, CALL_APPS))
        .child(row().gap(8.0).center_y().mt(4.0).child(heading("With Labs \u{203a} More call apps")).children(more_tag))
        .child(chips(k, MORE_CALL_APPS).opacity(if more == Some(false) { 0.5 } else { 1.0 }))
        .child(
            text("Hex also names calls on more websites, such as Google Voice, Messenger, GroupMe, and Teams and FaceTime on the web.")
                .font(k.ty.small.lh(18.0))
                .color(c.text_faint)
                .wrap(),
        )
}

pub fn page(s: &App, k: &Kit) -> El {
    super::page_shell(
        "about",
        header(k, "About", "Hex Meetings, the app it's built on, and the calls it records.", vec![]),
        col().w_full().gap(20.0).child(card(k)).child(calls(s, k)),
    )
}
