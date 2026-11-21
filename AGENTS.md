# Hex Meetings

An unofficial Rune desktop app that reads Hex's meetings through its public CLI and runs user-written reports (a name, a focus, a destination, an agent) on a transcript. Not affiliated with Hex or Anomaly. Say so in any user-facing text that could suggest otherwise.

## Layout

- `src/hex.rs`: the only code that talks to Hex: `hex meetings list|show --json` (JSON version 1), `hex settings get meetings`, `hex model list`, `hex settings set meetings.retention`, `hex labs on meetings`, and `hex window show meetings|models`. Never switch Hex's speech model: dictation uses it too. Don't read Hex's internal folders.
- `src/reports.rs`: the report format, the prompt the app builds around a report's focus (`Report::prompt`), and the one built-in report, Summary (`reports::builtin`). The app writes no reports for people; `examples/reports/*.md` are samples (and the demo's data). Edited in the app (`src/ui/reports.rs`) or by hand.
- `src/agents.rs`: the known agent CLIs (OpenCode, Claude Code, Codex, Cursor Agent, Pi) and their headless flags. Their marks are in `src/ui/logos.rs`.
- `src/settings.rs`: the default agent and its model, the summaries folder, and the calls already kept or deleted.
- `src/speech.rs`: whether Hex's speech model covers the calls' language, and which models would.
- `src/places.rs`: connected places (Slack, Notion, GitHub, Linear): the output format each needs, the Keychain, and the post. `src/places/notion.rs` turns Markdown into Notion blocks. Don't try to detect an agent's own connections; "Your agent's tools" (`sends_to`) leaves that to the agent.
- `src/runner.rs`: runs an agent's command on a worker thread, with no shell.
- `src/activity.rs`: the local run log.
- `src/app.rs`: state, events, workers (inbox + waker, drained in `poll`).
- `assets/icon.svg`: the app icon (master); `assets/icon.png` is its 1024 px render, used for the Dock and the README. The in-app copy is `logos::APP`.
- `src/ui/`: views. `src/demo.rs`: made-up data for scenes and tests.

## Rules

- Use rustup's toolchain (`rust-toolchain.toml`); Homebrew's older `cargo` can't build Rune.
- Nothing blocks the UI thread: Hex calls and agent runs happen on threads that push `Event`s.
- The app never calls a model. Reports run the user's own agent. The only web requests are posts the person approves, to places they connected (`src/places.rs`: Slack webhook, Notion, GitHub, Linear). Tokens live in the Keychain and never reach an agent.
- Show where a transcript goes before every run the person starts, and preview a result before posting it. Automatic runs (`auto: true`) are opted into per report, in the editor that shows where it goes; they never post by themselves and can't use `sends_to`.
- Check UI changes headless: `cargo run -- --snap out.png --scene <name>` (see `SCENES` in `src/main.rs`), in both `--theme light` and `--theme dark`.
- `cargo test` must pass. Demo data stays fictional.
