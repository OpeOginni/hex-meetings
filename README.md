<p align="center"><img src="assets/icon.svg" width="88" alt=""></p>
<h1 align="center">Hex Meetings</h1>
<p align="center">Turn recorded calls into summaries, action items, and follow-ups with the AI agent you already use.</p>
<p align="center">An unofficial macOS companion for <a href="https://anoma.ly/hex/">Hex</a>. Not affiliated with Hex or Anomaly.</p>

![Hex Meetings showing a fictional sales call, per-call Summary controls, and the finished action items](docs/overview.png)

## How it works

1. **Record a call in Hex.** Open it in Hex Meetings to read the transcript and Hex's notes.
2. **Run a report.** Start with **Summary**, or write your own instructions for things like sales follow-ups, customer feedback, or bug reports. Use OpenCode, Claude Code, Codex, Cursor Agent, or Pi.
3. **Use the result.** Copy it, save it as Markdown, or preview it before posting to Slack, Notion, GitHub, or Linear.

See the [example reports](examples/reports/) for ideas.

## Install

You need an **Apple silicon Mac with macOS 15 or later** and **Hex 3.5 or later**. Hex has a Windows build, but its Meetings recording feature is macOS-only, so Hex Meetings is macOS-only too.

Install the latest release into `/Applications`:

```sh
curl -fsSL https://raw.githubusercontent.com/OpeOginni/hex-meetings/main/scripts/install.sh | sh
```

Or download `Hex-Meetings-macOS-arm64.dmg` from the [latest release](https://github.com/OpeOginni/hex-meetings/releases/latest) and drag the app into Applications. The app isn't notarized by Apple yet, so the first time you open a downloaded copy, macOS blocks it: open **System Settings › Privacy & Security** and click **Open Anyway**. The install command doesn't need this step.

To update, open **About** and press **Check for updates**, then **Update**. The app downloads the new version, checks it, and reopens.

Follow **Get started** in the app to enable Meetings in Hex and record your first call. To run reports, install one of the supported agent CLIs and select it in **Setup**.

## Where your data goes

Hex records and transcribes the call. Hex Meetings reads it through [Hex's public CLI](https://github.com/anomalyco/hex/blob/dev/docs/transcript-cli.md) and doesn't open Hex's internal files.

Before a manual run, the app shows where your transcript will go. Your agent's model settings determine how it processes the transcript. Automatic runs are opt-in and never post on their own. For built-in destinations, the app keeps credentials in the macOS Keychain and asks before posting. If you choose **Your agent's tools**, the agent handles delivery through its own connections. The app asks GitHub for a newer version only when you press **Check for updates** in **About**, or each time it opens if you turn that on there.

## Development

You need [Rust installed via rustup](https://rustup.rs/).

```sh
git clone https://github.com/OpeOginni/hex-meetings.git
cd hex-meetings
sh scripts/run-macos.sh                       # build and open the app
HEX_MEETINGS_DEMO=1 sh scripts/run-macos.sh   # with fictional calls, no Hex or agent needed
cargo test --locked
cargo run --locked -- --list-scenes
cargo run --locked -- --snap docs/overview.png --scene result --theme dark --size 1240x820
```

To release, bump `version` in `Cargo.toml`, run `cargo build` to update `Cargo.lock`, commit, then push a matching tag:

```sh
git tag v0.2.0 && git push origin v0.2.0
```

The [release workflow](.github/workflows/release.yml) tests, builds and packages the app (`scripts/package-macos.sh`), and publishes the zip and disk image. It signs and notarizes them when the signing secrets listed in the workflow are set.

Built with [Rune](https://github.com/anomalyco/rune). [MIT license](LICENSE). See [asset credits](docs/credits.md).
