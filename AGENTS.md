# AGENTS.md - Echo TUI Music Player

## Project Overview
Rust TUI music player with SQLite backend, YouTube downloads, and custom audio processing.

## Development Commands
```bash
cargo fmt          # Format code
cargo lint         # Clippy strict checks (-D warnings)
cargo precommit    # Format + lint (alias for fmt && lint)
cargo build        # Build the TUI app (NOTE: src/main.rs has a stray `c` on line 21 that breaks compilation)
cargo run          # Run the application
```

## Architecture Notes

### Entry Flow
`main.rs` → `ignite::engine()` → `app::start()` → `ui::EchoCanvas::paint()`

### Main Event Loop
The TUI event loop in `ui.rs::paint()` multiplexes:
- 100ms ticker: UI refresh (currently placeholder)
- 200ms ticker: Animation updates (spinner, pulse, blink)
- 1000ms ticker: Clock + uptime display
- Event channel: Keyboard/mouse input handling

### Key State Management
Complex state in `app.rs` with:
- Tab navigation (Echo, Playlist, Download, Misc)
- Animation state with multiple frame counters
- Audio player state with mutex-wrapped Arc
- Report system for background task communication

## Database & Storage

### Database Path
- Runtime path is resolved via `directories::ProjectDirs` in `ignite.rs` (platform-specific, e.g. `~/Library/Application Support/echo/data/music.db` on macOS)
- Auto-created on first run via sqlx migrations
- The `.env` file at repo root is hardcoded to the developer's macOS path and is only for `sqlx` compile-time checks

### Schema
- `songs`: Track metadata with file paths
- `playlists`: Playlist names and IDs
- `playlist_songs`: Junction table with ordering

### Migration
Single migration: `migrations/202603290000_init_playlists.sql`
- Auto-run on DB initialization
- Uses WAL journal mode

## External Dependencies

### Required Tools
- `yt-dlp`: YouTube MP3 downloads (must be in PATH)
- SQLite: Database backend

### Key Libraries
- `ratatui`: TUI framework with all-widgets
- `crossterm`: Terminal input/output handling
- `sqlx`: Async SQLite with migrations
- `tokio`: Async runtime (full features)
- `cpal`: Audio playback backend
- `symphonia`: Audio decoding with all-codecs

## Important Conventions

### Error Handling
- Custom `EchoResult<T>` type in `src/result.rs`
- Comprehensive error variants for all failure modes
- Background task errors sent via report system

### Animation System
- 3 independent animation sequences with different tick rates
- Frame index increment logic in `increment_frame_index()`
- State stored in `AnimationState` struct

### Audio Architecture
- `AudioPlayer` wraps cpal backend
- `AudioData` state protected by Arc<Mutex<>>
- Audio controls via `with_audio_state()` helper

### Widget Implementation
- `EchoCanvas` implements ratatui `Widget` trait via `ui/layout.rs`
- Custom rendering in `draw()` method

## Development Quirks

### No Test Suite
- Project has no unit or integration tests
- All target artifacts are build outputs, not tests

### Logging
- Only active in debug builds (`cfg!(debug_assertions)`)
- Writes to `logs/dev.log` with rolling file appender
- Silent in release builds

### Configuration
- UI config path is resolved via `directories::ProjectDirs` (platform-specific, e.g. `~/Library/Application Support/echo/config/echo.toml` on macOS)
- Color themes and animation configs via TOML
- Defaults applied for missing fields

### File Structure
- Monorepo structure but single binary
- Modular organization by feature (db, audio, ui, etc.)
- No workspace or multiple packages

## Common Pitfalls

1. **src/main.rs is broken**: `main.rs` line 21 contains a stray `c` that prevents compilation. Must be fixed before `cargo build` or `cargo run` will succeed.
2. **Missing yt-dlp**: Download functionality fails silently without tool
3. **Database path**: Platform-dependent via `directories::ProjectDirs`, not a fixed `~/.config/...` path
4. **Async complexity**: Multiple tokio tasks require careful error handling
5. **Animation state**: Complex interdependent animation tickers
6. **Audio locks**: Mutex can poison, requires error handling