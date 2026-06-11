# Implementation Roadmap: Download Tab + Metadata Form + Queue

## Overview
YouTube download with metadata form, queue manager (5 concurrent), pause/resume, priority reorder.

## Phase 1: Download Manager Core (`src/download/manager.rs`)

### Task 1.1: Job Struct
```rust
pub struct DownloadJob {
    pub id: u32,
    pub url: String,
    pub metadata: Metadata,
    pub status: JobStatus,
}

pub enum JobStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed(String),
}
```

### Task 1.2: Queue Struct
```rust
pub struct DownloadQueue {
    pub jobs: Vec<DownloadJob>,
    pub max_running: usize,
    pub command_rx: UnboundedReceiver<QueueCommand>,
    pub report_tx: Sender<Report>,
}
```

### Task 1.3: Commands
```rust
pub enum QueueCommand {
    AddJob(DownloadJob),
    PauseJob(u32),
    ResumeJob(u32),
    CancelJob(u32),
    RemoveJob(u32),
    MoveUp(u32),
    MoveDown(u32),
}
```

### Task 1.4: Run Loop
- Poll commands via `select!`
- Maintain `running_count`
- Start new jobs when `running_count < 5`
- Spawn yt-dlp with `Command::spawn()`

### Task 1.5: Start Job
- Spawn yt-dlp child
- Monitor `child.wait()`
- On success: rename, insert DB, report
- On failure: report ERR
- On pause: `child.kill().await`, mark Paused

### Task 1.6: Create `download/mod.rs`
Expose manager + original functions.

### Task 1.7: Cargo.toml
Add `uuid` or `rand` if needed (prefer `u32` counter).

## Phase 2: YouTube Metadata Fetch

### Task 2.1: `fetch_youtube_info(url)`
- Run `yt-dlp --print title --print uploader --no-download <url>`
- Return `(title, uploader)`

### Task 2.2: `download_with_child(url, path, metadata)`
- Return `(Child, impl Future)` for pause/resume
- yt-dlp `--continue` handles partial files

## Phase 3: State & Form (`src/app.rs`)

### Task 3.1: `DownloadMetadataForm`
```rust
pub struct DownloadMetadataForm {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub genre: String,
    pub track_number: String,
    pub total_tracks: String,
    pub disc_number: String,
    pub total_discs: String,
    pub album_artist: String,
}
```

### Task 3.2: Default impl
All fields = `String::new()`.

### Task 3.3: Update `DownloadState`
```rust
pub enum DownloadState {
    Idle,
    InputUrl,
    FetchingMetadata,
    InputMetadata,
    ViewQueue,
}
```

### Task 3.4: Add to `State`
```rust
pub download_metadata_form: DownloadMetadataForm,
pub download_metadata_selected_pos: usize,
pub download_queue: DownloadQueue,
pub download_queue_selected_pos: usize,
pub download_queue_command_tx: UnboundedSender<QueueCommand>,
```

### Task 3.5: `State::new()`
- Create channel
- Initialize queue
- Spawn queue worker

### Task 3.6: Helpers
```rust
pub fn get_download_metadata_field_mut(&mut self, pos: usize) -> &mut String
pub fn next_download_metadata_field(&mut self)
pub fn prev_download_metadata_field(&mut self)
pub fn clear_download_metadata_form(&mut self)
```

## Phase 4: Download Tab UI (`src/ui/components/tabs/download.rs`)

### Task 4.1-4.2: Create file + dispatcher
Match `download_state` -> render fn.

### Task 4.3: Idle
"Press `d` to add download"

### Task 4.4: InputUrl
Input box + URL buffer.

### Task 4.5: FetchingMetadata
Spinner + "Fetching YouTube info..."

### Task 4.6: InputMetadata
- 10 rows: `[FIELD_NAME] value`
- Selected: accent border + highlight
- Hint: "↑↓ navigate | Enter confirm next field | Enter last field confirm all | Esc cancel"

### Task 4.7: ViewQueue
- Format: `[P] Title - Artist` (P=Pending, R=Running, D=Done, X=Paused, F=Failed)
- Selected: accent highlight
- Hint: "w/s navigate | p pause/resume | x cancel | d new download | a up priority | D down priority"

### Task 4.8: Update `tabs/mod.rs`
`pub mod download;`

### Task 4.9: Wire `components.rs`
Add `SelectedTab::Download` branch.

## Phase 5: Event Handling (`src/event.rs`)

### Task 5.1: Idle
`Char('d')` -> `InputUrl`

### Task 5.2: InputUrl
- `Enter`: if valid, spawn `fetch_youtube_info`, set `FetchingMetadata`
- `Char(c)`: append url_buffer
- `Backspace`: pop url_buffer
- `Esc`: clear, go `Idle`

### Task 5.3: FetchingMetadata -> InputMetadata
- On success: pre-fill form (title=video title, artist=uploader)
- On fail: report ERR, go `Idle`

### Task 5.4: InputMetadata
- `Up`/`Down`: move `download_metadata_selected_pos` (wrap 0-9)
- `Char(c)`: append current field
- `Backspace`: pop current field
- `Enter`: if pos < 9, next field. If pos == 9, parse numerics -> build `Metadata` -> send `AddJob` -> go `ViewQueue`
- `Esc`: clear, go `Idle`

### Task 5.5: ViewQueue
- `w`/`s`: move `download_queue_selected_pos`
- `p`: Running -> `PauseJob(id)`, Paused -> `ResumeJob(id)`
- `x`: Running -> `CancelJob(id)`, else -> `RemoveJob(id)`
- `d`: go `InputUrl`
- `a`: `MoveUp(id)`
- `D`: `MoveDown(id)`
- `Esc`: go `Idle`

### Task 5.6: FetchingMetadata key handling
Only `Esc` to cancel.

## Phase 6: Auto-refresh Library (`src/ui.rs`)

### Task 6.1: In `paint()` loop
After report drain:
- Check if report contains "Downloaded:"
- Call `Library::get_songs_from_db(pool, 0, 10).await`
- Update `state.local_songs`

### Task 6.2: Add `refresh_local_songs()` to `EchoCanvas`

## Phase 7: Report Rendering Fix (`src/ui/components.rs`)

### Task 7.1: Update report block
```rust
match level {
    LogLevel::INFO => success color,
    LogLevel::ERR => error color,
    LogLevel::WARN => warning color,
}
```

### Task 7.2: All levels render in same tab area

## Phase 8: Download Task Integration

### Task 8.1: Update `download.rs`
Accept `Metadata` parameter. Write tags after download.

### Task 8.2: Queue `start_job`
- Download to temp
- `db::insert_song` with metadata
- Rename to `{id}.mp3`
- `db::update_song_path`
- `metadata.update_file(new_path)`
- Report "Downloaded: {title} (id={id})"

## Phase 9: Build & Test

### Task 9.1: `cargo build`
Fix compilation errors.

### Task 9.2: Valid URL
-> form pre-fill -> edit -> download -> success

### Task 9.3: Invalid URL
-> error

### Task 9.4: Pause/resume
Running download

### Task 9.5: Queue 8 items
5 running, 3 pending

### Task 9.6: Priority
Move up/down

### Task 9.7: Cancel/remove

### Task 9.8: Library refresh
After download

---

## Notes

- `DownloadQueue` needs `active_children: HashMap<u32, Child>` to track running processes for pause/resume
- `State` needs `UnboundedSender<QueueCommand>` for event.rs to send commands to the queue
- `u32` counter for job IDs, not ULID
- yt-dlp `--continue` handles partial file resume on restart
- All numeric fields parse as u32, default 0 on empty
