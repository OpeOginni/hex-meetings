//! Recipes: what each one does, where it sends a call, and the command it
//! runs. Recipes are edited as files, so this page shows them and opens
//! the folder rather than growing an editor.

use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

use super::header;
use crate::app::{self, App};
use crate::paths;

pub fn page(s: &App, k: &Kit) -> El {
    let c = &k.c;
    let mut list = col().key("recipe-list").w_full().gap(12.0);
    for e in &s.recipe_errors {
        list = list.child(super::notice(k, e, None));
    }
    for r in &s.recipes {
        list = list.child(
            col()
                .key(format!("recipe-card-{}", r.id))
                .w_full()
                .gap(8.0)
                .padding(16.0)
                .radius(k.radius.lg)
                .fill(c.surface_raised_base)
                .border(1.0, c.border_weak)
                .child(text(r.name.clone()).font(k.ty.ui_medium).color(c.text_strong))
                .child(text(r.description.clone()).font(k.ty.ui).color(c.text_muted).wrap())
                .child(
                    row()
                        .gap(6.0)
                        .child(text("Sends to").font(k.ty.small_medium).color(c.text_faint).w(64.0))
                        .child(text(r.sends_to.clone()).font(k.ty.small).color(c.text_base).wrap().grow(1.0).shrink().min_w(0.0)),
                )
                .child(
                    row()
                        .gap(6.0)
                        .child(text("Runs").font(k.ty.small_medium).color(c.text_faint).w(64.0))
                        .child(
                            text(r.command_line())
                                .font(k.ty.mono)
                                .color(c.text_base)
                                .wrap_anywhere()
                                .selectable(true)
                                .grow(1.0)
                                .shrink()
                                .min_w(0.0),
                        ),
                ),
        );
    }
    if s.recipes.is_empty() && s.recipe_errors.is_empty() {
        list = list.child(text("No recipes yet. Add a .md file to the recipes folder.").font(k.ty.ui).color(c.text_muted));
    }
    let actions = vec![
        Button::new("Open folder")
            .variant(ButtonVariant::Neutral)
            .on_press(|s: &mut App| {
                if !s.offline {
                    app::reveal(&paths::recipes_dir());
                }
            })
            .build(k)
            .key("open-recipes"),
        Button::new("Reload").variant(ButtonVariant::Ghost).on_press(|s: &mut App| s.reload_recipes()).build(k).key("reload-recipes"),
    ];
    let help = col()
        .key("recipe-help")
        .w_full()
        .gap(6.0)
        .padding(16.0)
        .radius(k.radius.lg)
        .fill(c.layer_01)
        .child(text("Writing a recipe").font(k.ty.ui_medium).color(c.text_strong))
        .child(
            text(
                "A recipe is a Markdown file with name, description, command and sends_to at the top, then the prompt. \
                 The prompt can use {{title}}, {{date}}, {{duration}}, {{app}}, {{summary}} and {{transcript}}. \
                 {prompt} in the command is where the filled-in prompt goes. Any tool with a command line works: \
                 opencode run, claude -p, or your own script. Its own settings decide the model and where results go.",
            )
            .font(k.ty.ui)
            .color(c.text_muted)
            .wrap(),
        );
    col()
        .key("recipes-page")
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(row().w_full().h(40.0).window_drag())
        .child(
            col()
                .key("recipes-scroll")
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
                        .child(header(k, "Recipes", "Each recipe hands a call to a tool you already use. Edit the files to change them.", actions))
                        .child(list)
                        .child(help),
                )
                .scroll(ScrollSpec { stick_bottom: false }),
        )
}
