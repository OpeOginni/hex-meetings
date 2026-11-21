# Hex Meetings

An unofficial Rune desktop app that reads Hex's meetings through its public CLI and runs user-written recipes (agent commands) on a transcript. Not affiliated with Hex or Anomaly. Say so in any user-facing text that could suggest otherwise.

## Layout

- `src/hex.rs`: the only code that talks to Hex, through `hex meetings list|show --json` (JSON version 1). Don't read Hex's internal folders.
- `src/recipes.rs` + `recipes/*.md`: the recipe format and the shipped examples.
- `src/runner.rs`: runs a recipe command on a worker thread, with no shell.
- `src/activity.rs`: the local run log.
- `src/app.rs`: state, events, workers (inbox + waker, drained in `poll`).
- `src/ui/`: views. `src/demo.rs`: made-up data for scenes and tests.

## Rules

- Use rustup's toolchain (`rust-toolchain.toml`); Homebrew's older `cargo` can't build Rune.
- Nothing blocks the UI thread: Hex calls and agent runs happen on threads that push `Event`s.
- The app itself never calls a model or a web service. Recipes run the user's own tools.
- Show where a transcript goes (`sends_to`) before every run.
- Check UI changes headless: `cargo run -- --snap out.png --scene <name>` (see `SCENES` in `src/main.rs`), in both `--theme light` and `--theme dark`.
- `cargo test` must pass. Demo data stays fictional.
