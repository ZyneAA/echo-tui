# Progress — Where We Left Off

Updated: 2026-09-01

## Done (uncommitted)

### 1. Search sub-tab: delete song from DB (`d` key)
- `d` in Echo → Search shows bottom-bar prompt `DELETE SONG FROM DB? (y/n)`. `y` deletes, any other key cancels.
- Deletes DB row by `file_path` (`db::repository::Repository::delete_song_by_path`, added in `src/db/repository.rs`). `Song` has no DB id → path is the key.
- Removed from `local_songs` in place, `selected_song_pos` clamped.
- If deleted song is the `active_track`, playback stopped by replacing player with `AudioPlayer::bad()` + `audio_state = None` (stream drop = stop). No file deletion from disk — DB row only.
- Files: `src/app.rs` (`is_confirm_delete`), `src/event/echo/sub_events.rs`, `src/db/repository.rs`, `src/ui/components/tabs/echo.rs`.

### 2. Import sub-tab: folder/file picker + confirm prompt
- Path buffer `Enter`:
  - **Single file** (`.mp3`) → confirm prompt directly.
  - **Directory** → lists all `.mp3` files in a selectable table in the (previously empty) lower import pane. Bottom hint bar: `w/s nav · SPACE select · a all · ENTER import · ESC cancel`.
  - Bad path → error via `report_tx`, buffer cleared.
- Batch mode keys: `w`/`s` nav, `SPACE` toggle, `a` select all/none, `Enter` → prompt `IMPORT N FILE(S)? (y/n)`, `y` imports selected inline, `Esc` clears list.
- Old inline import loop refactored into `async fn import_file(pool, old_path, songs_dir)` in `sub_events.rs`. Awaited inline (not spawned) so `local_songs` reloads from DB right after → new imports visible in Search immediately (fixes old bug where they only appeared after restart).
- Import errors now go to `report_tx` (`LogLevel::ERR`) instead of `eprintln!`.
- Stale-path bug fixed: `import_buffer` cleared after use.
- New state in `EchoTabState` (`src/app.rs`): `import_file_list`, `import_file_selected_pos`, `import_selected: Vec<bool>`, `is_confirm_import`.

### 3. Import fixes: title + destination
- **Title "Unknown" bug**: `Metadata::from_path` (`src/awdio/metadata.rs:51`) defaults missing tags to `"Unknown"`, so the old `is_empty()` check never fired. `import_file` now falls back to the file stem when title is empty or `"Unknown"`.
- **Destination**: imported files are **copied** to `~/Library/Application Support/echo/songs/{id}.mp3` (`canvas.all_paths.songs`) instead of renamed in the source folder. Source file untouched.

### 4. Metadata edit buffer fix
- **Bug**: while a metadata edit field was open, keystrokes hit global handlers first (`main_events.rs`) — `i p k j h l f` never typed into the buffer and instead toggled pause/volume/seek/FFT. Metadata buffer lacked the early-return that Search/Import buffers had (existed as commented stub).
- **Fix**: enabled the metadata early-return in `handle_echo_key_event`; `i`-open in METADATA now clears `state.buffer` (stale-text bug).
- **Bug**: METADATA pane always showed "NO SONGS FOUND." — `is_zero_local_song` initialized `true` and only ever set `true` (never reset when songs exist; search table ignores the flag, so it looked fine). Fix: `app.rs start()` now assigns `is_zero_local_song = local_songs.is_empty()`.

### 5. Metadata sub-tab UI fixes
- FILE PATH input block no longer REVERSED-highlighted in METADATA subtab (`block.rs` — METADATA now shares SEARCH's bold style).
- Edit buffer unified on `echo_tab_state.metadata_buffer` (was writing `state.buffer` while render displayed the never-written `metadata_buffer`). Metadata pane bottom title now shows `INSERT:` badge (accent bg) while editing + live typed chars.
- Numeric fields (YEAR/TRACK/TOTAL/DISC) always blank in metadata pane — display used `toml::to_string(&u32)` which errors on scalars → `unwrap_or_default()` = `""`. Fix: plain `.to_string()`; dropped unused `toml::to_string` import.

### 6. Search sub-tab: live filtering
- Search now live-filters the song table as you type (was a no-op before). Matches title/artist/album/genre, case-insensitive.
- Filter keybinds (shown as input-block title): `T:title` `A:artist` `AL:album` `G:genre`; no prefix = all fields; empty query = unfiltered.
- `w`/`s` navigate within matched rows; `Esc` clears query + filter; `Backspace` works in search input now. Delete + import-reload recompute the filter.
- Input block title no longer echoes typed chars (was showing filename-hint of query); SEARCH shows `FILTER | T:title A:artist AL:album G:genre` instead of `FILE PATH | hint`.
- Zero search matches (non-empty query) → table replaced with centered "NO MATCHES."; no songs shown.
- New state: `EchoTabState.search_matched: Vec<usize>` (indices into `local_songs`).
- Flow rework: `i` → straight into query input (typing, INSERT MODE badge bottom-right); `f` → filter picker menu (ALL/TITLE/ARTIST/ALBUM/GENRE via `w/s`+`Enter` or direct keybinds `t/a/l/g`). Default ALL. Title shows `FILTER: ALL · TITLE · ...` with active filter bold+reversed. `Esc` on list resets filter to ALL. New state: `search_filter: SearchFilter`, `search_filter_selecting`, `search_filter_pos`. Note: `f` = FFT toggle everywhere except Echo>Search.

### 7. Exit confirmation
- `Esc` in free mode (no active buffer/sub-input) → centered bottom prompt `EXIT? (y/n)`. `y`/`Enter` exits, `n`/`Esc` cancels. `State.is_confirm_exit`.

### 8. Download sub-tab (Echo → DOWNLOAD)
- Migration `migrations/20260905000000_create_downloads.sql`: `downloads` table (url, source default 'youtube', status, file_path, started_at, finished_at). NOTE: applied manually to dev DB (`sqlite3 ... < migration`) for sqlx compile-time checks — runtime DBs migrate automatically.
- Two panes: **PROGRESSES** (in-flight: percent, speed, start time, url, source — updated live by worker via `Arc<Mutex<Vec<DownloadProgress>>>`) + **HISTORY** (finished/failed from DB, newest first, 10/page, `n` next / `b` prev, `+` in title = more pages). `w`/`s` switch pane focus (highlighted title).
- `i` opens URL input (INSERT MODE badge), `Enter` queues. yt-dlp spawned with piped stdout (`--newline --progress`) — `download_mp3_with_progress` in `download.rs` parses %/speed per line. Source hardcoded youtube (only source for now).
- On finish: row moved to history (`finish_download`), file auto-imported into library via `import_file` (now `pub(crate)`), Report on success/failure.
- New state in `EchoTabState`: `is_echo_download_buffer_being_filled`, `download_buffer`, `download_pane`, `download_progress`, `download_history`, `download_history_page`, `download_history_has_more`. History loads on subtab entry + page change.
- Skipped: auto-refresh of history pane while a download finishes (re-enter subtab or change page to see it), pause/cancel of running downloads, sources other than youtube.
- Download indexing: worker imports via `import_file` (DB title = yt-dlp title tag or video-name stem; file renamed/copied to `songs/{id}.mp3`), original video-titled file removed after copy. `f` on DOWNLOAD input no longer REVERSED-highlighted (added to bold arm in `block.rs`).

## Known issues / preexisting (not touched)
- `src/main.rs` line ~21: stray `c` breaks `cargo build` per AGENTS.md (note: current builds pass, may already be fixed — verify before relying on AGENTS.md note).
- `local_songs` loaded with `LIMIT 0..10` (`app.rs:346`, `get_songs_from_db(pool, 0, 10)`) — import reload only shows first 10 songs.
- `Metadata::from_path` "Unknown" defaults also apply to artist/album/genre in DB rows.
- Duplicate import routine on `|` key (`main_events.rs:50-137`) does NOT use new `import_file` (no title fix, renames in place) — candidate for consolidation.
- `Report` display only shows `INFO` level (`ui/components.rs:174-187`); ERR/WARN silently dropped.
- Clippy warnings preexist (`current_song`, `enable_fft_compute`, unused fns, acronym names).

## Verification status
- `cargo build` ✅, `cargo clippy` ✅ (no new warnings), `cargo fmt` applied.
- NOT manually tested yet: run `cargo run` →
  - Search: `d` → `y` deletes; playing track stops if active.
  - Import: `i` → folder path → select → `Enter` → `y` → files copied to songs dir, titles from filename when tags missing, Search list refreshes.
- No test suite exists (per AGENTS.md).

## Possible next steps
- Consolidate `|` bulk import with `import_file`.
- Confirm prompts styled/positioned more prominently.
- Pagination for song list (load all songs, not first 10).
