# Echo TUI

A fast, terminal-native music player for local MP3 libraries, built in Rust. Echo combines a full-featured TUI with an embedded SQLite library, YouTube downloads, and real-time audio visualization — all without leaving your terminal.

## Features

- **Terminal-first experience** — built with [ratatui](https://github.com/ratatui/ratatui) and crossterm; no GUI required.
- **Local library management** — import MP3 folders, browse/search songs, and store rich metadata in a SQLite database (auto-created and migrated on first run).
- **Inline metadata editor** — edit title, artist, album, year, genre, track/disc numbers, and album artist directly from the UI. Changes are written back to both the file tags and the database.
- **Playlists** — create, delete, rename-free ordered lists; add or remove songs and play straight from a playlist.
- **YouTube downloads** — download audio as MP3 with `yt-dlp`, auto-fill metadata from the video, and index the result into your library.
- **Real-time FFT visualization** — live spectral analysis rendered in the UI, toggleable on the fly.
- **Audio playback engine** — custom pipeline built on `cpal` and `symphonia` with background decode, seek, pause/resume, and volume control.
- **Customizable theme** — colors and animation characters configured via a TOML file.

## Requirements

- **Rust** toolchain (edition 2024). See <https://rustup.rs>.
- **[yt-dlp](https://github.com/yt-dlp/yt-dlp)** in your `PATH` — required only for the YouTube download feature.
- **SQLite** — bundled and managed by `sqlx`; no manual setup needed.

## Getting Started

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run
```

The first launch creates your application data directory, runs the database migrations, and creates a default config file. On macOS these live under `~/Library/Application Support/echo/`; on Linux under `~/.local/share/echo/` and `~/.config/echo/`.

### Development commands

```bash
cargo fmt                # Format code
cargo lint               # Clippy strict checks (-D warnings)
cargo precommit          # Format + lint
```

Logging is active in debug builds only and writes to `logs/dev.log`. Release builds are silent.

## Usage

Navigate between tabs with the **Left**/**Right** arrow keys. **Esc** cancels the current input or exits the app.

### Echo tab — local library

| Keys | Action |
| --- | --- |
| `Shift+S`, `Shift+I`, `Shift+D`, `Shift+M` | Switch to Search / Import / Download / Metadata sub-tabs |
| `i` | Start typing in the active sub-tab |
| `w` / `s` | Navigate songs |
| `Enter` | Play the selected song |
| `p` | Pause / resume |
| `k` / `j` | Volume up / down |
| `h` / `l` | Seek backwards / forwards 1s |
| `f` | Toggle FFT visualization |
| `|` | Bulk-import every MP3 in the songs directory |

### Playlist tab

| Keys | Action |
| --- | --- |
| `n` | Create a new playlist |
| `w` / `s` | Navigate playlists / songs |
| `Enter` | Open a playlist (or play a song inside it) |
| `a` | Add the current library song to the selected playlist |
| `d` | Delete the selected playlist |
| `r` | Remove the selected song from the playlist |
| `R` | Refresh playlists from the database |
| `Backspace` | Back to the playlist list |

### Download tab

| Keys | Action |
| --- | --- |
| `d` | Start a new download (paste a YouTube URL) |
| `Enter` | Confirm the URL and begin downloading |
| `Esc` | Cancel the current download input |

## Configuration

UI colors and animation characters are configured in `echo.toml`. The file is created automatically on first run and defaults are applied for any missing fields.

Platform-specific config paths:

- **macOS:** `~/Library/Application Support/echo/config/echo.toml`
- **Linux:** `~/.config/echo/config/echo.toml`

```toml
[colors]
bg = "#0b0b0f"
fg = "#c8c8c8"
accent = "#7aa2f7"
primary = "#3b3b3b"
success = "#9ece6a"
error = "#f7768e"
warning = "#e0af68"
info = "#7dcfff"
title = "#ffffff"
border = "#565f89"

[animations]
spinner = ["/", "-", "\\", "|"]
hpulse = ["| ⎟ ⎜", "⎜ | ⎜", "⎟ ⎢ |"]
dot = 3
timestamp = "☐"
timestamp_bar = "▲"
```

## How It Works

- **Entry flow:** `main.rs` → `ignite::engine()` (resolves paths, config, and DB) → `app::start()` → `ui::EchoCanvas::paint()`.
- **Event loop:** a tokio-based loop in `ui.rs` multiplexes three tickers (100ms refresh, 200ms animation, 1000ms clock/uptime) with keyboard/mouse events from a background crossterm task.
- **Audio engine:** `AudioPlayer` decodes files with `symphonia`, streams them through `cpal`, and runs a background `rustfft` thread for spectral analysis. Shared state lives in `Arc<Mutex<AudioData>>`.
- **Database:** `sqlx` with automatic migrations; WAL journal mode. Schema lives in `migrations/`.

See [`AGENTS.md`](AGENTS.md) for detailed architecture notes and [`PLAN.md`](PLAN.md) for the download-queue roadmap.

## Getting Help

- Open an issue at <https://github.com/ZyneAA/echo-tui/issues> for bugs or feature requests.
- See the project roadmap in [`PLAN.md`](PLAN.md).

## Contributing

Contributions are welcome. Please follow these steps:

1. Fork the repository and create a feature branch.
2. Run `cargo fmt` and `cargo lint` before submitting — the linter runs with `-D warnings`.
3. Keep commits small and focused; one logical change per commit.
4. Open a pull request describing your change and how you tested it.

Note: the project currently has no automated test suite, so manual verification of changes is expected.

## Maintainers

Maintained by the author of [ZyneAA/echo-tui](https://github.com/ZyneAA/echo-tui). This is an early-stage (v0.1.0) project — expect active development and a rough edge or two.
