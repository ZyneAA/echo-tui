use std::path::Path;

use tokio::fs;

use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    app::{LogLevel, Report},
    awdio::{AudioPlayer, metadata::Metadata},
    db,
    result::{EchoReport, EchoResult},
    ui::EchoCanvas,
};

async fn import_file(
    pool: &sqlx::sqlite::SqlitePool,
    old_path: &Path,
    songs_dir: &Path,
) -> EchoResult<()> {
    let path_str = old_path.to_str().ok_or_else(|| {
        crate::result::EchoReport::InvalidMetadata("invalid path encoding".into())
    })?;

    let stem = old_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");

    let mut tag = Metadata::from_path(path_str)?;

    // Metadata::from_path defaults missing tags to "Unknown" -> fall back to file name
    let db_title = if tag.title.is_empty() || tag.title == "Unknown" {
        stem.to_string()
    } else {
        tag.title.clone()
    };

    let id = sqlx::query!(
        "INSERT INTO songs (title, artist, album, file_path) VALUES (?, ?, ?, ?)",
        db_title,
        tag.artist,
        tag.album,
        "PENDING"
    )
    .execute(pool)
    .await?
    .last_insert_rowid();

    // copy into the app's songs dir instead of renaming in the source folder
    let new_path = songs_dir.join(format!("{}.mp3", id));

    fs::copy(&old_path, &new_path).await?;

    tag.title = db_title.clone();
    if let Some(new_path_str) = new_path.to_str() {
        let _ = tag.update_file(new_path_str);
        let _ = sqlx::query!(
            "UPDATE songs SET file_path = ? WHERE id = ?",
            new_path_str,
            id
        )
        .execute(pool)
        .await;
    }

    Ok(())
}

fn report_err(reporter: &std::sync::mpsc::Sender<Report>, msg: String) {
    reporter
        .send(Report {
            log: Some(msg),
            report: None,
            level: LogLevel::ERR,
        })
        .ok();
}

pub async fn handle_echo_import_key_enent(
    canvas: &mut EchoCanvas,
    key_event: KeyEvent,
) -> EchoResult<()> {
    // import confirmation prompt (single file + batch)
    if canvas.state.echo_tab_state.is_confirm_import {
        canvas.state.echo_tab_state.is_confirm_import = false;
        if let KeyCode::Char('y') = key_event.code {
            let pool = canvas.db_connection_pool.clone();
            let reporter = canvas.state.report_tx.clone();
            let songs_dir = canvas.all_paths.songs.clone();

            let paths: Vec<String> = canvas
                .state
                .echo_tab_state
                .import_file_list
                .iter()
                .zip(&canvas.state.echo_tab_state.import_selected)
                .filter(|(_, sel)| **sel)
                .map(|(p, _)| p.clone())
                .collect();

            let mut imported = 0u32;
            let mut failed = 0u32;
            for p in &paths {
                match import_file(&pool, Path::new(p), &songs_dir).await {
                    Ok(_) => imported += 1,
                    Err(e) => {
                        failed += 1;
                        report_err(&reporter, format!("import failed {}: {}", p, e));
                    }
                }
            }

            // refresh search list from DB
            match db::repository::Repository::get_songs_from_db(&pool, 0, 10).await {
                Ok(songs) => {
                    canvas.state.local_songs = songs;
                    canvas.state.echo_tab_state.is_zero_local_song =
                        canvas.state.local_songs.is_empty();
                }
                Err(e) => report_err(&reporter, format!("reload error: {}", e)),
            }

            if failed == 0 {
                reporter
                    .send(Report {
                        log: Some(format!("IMPORTED {} FILE(S)", imported)),
                        report: None,
                        level: LogLevel::INFO,
                    })
                    .ok();
            }
        }

        canvas.state.echo_tab_state.import_file_list.clear();
        canvas.state.echo_tab_state.import_selected.clear();
        canvas.state.echo_tab_state.import_file_selected_pos = 0;
        canvas
            .state
            .echo_tab_state
            .import_buffer
            .lock()
            .await
            .clear();
        return Ok(());
    }

    // file list mode: navigate / select / confirm / cancel
    if !canvas.state.echo_tab_state.import_file_list.is_empty() {
        let list_state = &mut canvas.state.echo_tab_state;
        match key_event.code {
            KeyCode::Char('w') => {
                list_state.import_file_selected_pos = list_state
                    .import_file_selected_pos
                    .saturating_sub(1)
                    .min(list_state.import_file_list.len().saturating_sub(1));
            }
            KeyCode::Char('s') => {
                list_state.import_file_selected_pos = (list_state.import_file_selected_pos + 1)
                    .min(list_state.import_file_list.len().saturating_sub(1));
            }
            KeyCode::Char(' ') => {
                let pos = list_state.import_file_selected_pos;
                if let Some(sel) = list_state.import_selected.get_mut(pos) {
                    *sel = !*sel;
                }
            }
            KeyCode::Char('a') => {
                let any_off = list_state.import_selected.iter().any(|s| !*s);
                let val = any_off;
                for sel in list_state.import_selected.iter_mut() {
                    *sel = val;
                }
            }
            KeyCode::Enter => {
                if canvas
                    .state
                    .echo_tab_state
                    .import_selected
                    .iter()
                    .any(|s| *s)
                {
                    canvas.state.echo_tab_state.is_confirm_import = true;
                }
            }
            KeyCode::Esc => {
                let list_state = &mut canvas.state.echo_tab_state;
                list_state.import_file_list.clear();
                list_state.import_selected.clear();
                list_state.import_file_selected_pos = 0;
                list_state.is_echo_import_buffer_being_filled = false;
                list_state.import_buffer.lock().await.clear();
            }
            _ => {}
        }
        return Ok(());
    }

    if canvas
        .state
        .echo_tab_state
        .is_echo_import_buffer_being_filled
    {
        match key_event.code {
            KeyCode::Char(c) => {
                let mut guard = canvas.state.echo_tab_state.import_buffer.lock().await;
                guard.push(c);
            }
            KeyCode::Enter => {
                canvas
                    .state
                    .echo_tab_state
                    .is_echo_import_buffer_being_filled = false;

                let path_str = canvas
                    .state
                    .echo_tab_state
                    .import_buffer
                    .lock()
                    .await
                    .clone();
                let path = Path::new(&path_str);

                if path.is_file() {
                    // single file: straight to confirm
                    canvas.state.echo_tab_state.import_file_list = vec![
                        canvas
                            .state
                            .echo_tab_state
                            .import_buffer
                            .lock()
                            .await
                            .clone(),
                    ];
                    canvas.state.echo_tab_state.import_selected = vec![true];
                    canvas.state.echo_tab_state.is_confirm_import = true;
                } else if path.is_dir() {
                    // batch: list all mp3 files in folder
                    let mut files = Vec::new();
                    if let Ok(mut entries) = fs::read_dir(&path_str).await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            let p = entry.path();
                            if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("mp3")
                            {
                                if let Some(ps) = p.to_str() {
                                    files.push(ps.to_string());
                                }
                            }
                        }
                    }
                    files.sort();
                    let selected = vec![false; files.len()];
                    canvas.state.echo_tab_state.import_file_list = files;
                    canvas.state.echo_tab_state.import_selected = selected;
                    canvas.state.echo_tab_state.import_file_selected_pos = 0;
                    canvas
                        .state
                        .echo_tab_state
                        .import_buffer
                        .lock()
                        .await
                        .clear();
                } else {
                    report_err(
                        &canvas.state.report_tx,
                        format!("path not found: {}", path_str),
                    );
                    canvas
                        .state
                        .echo_tab_state
                        .import_buffer
                        .lock()
                        .await
                        .clear();
                }
            }
            KeyCode::Backspace => {
                let mut guard = canvas.state.echo_tab_state.import_buffer.lock().await;
                guard.pop();

                return Ok(());
            }
            KeyCode::Esc => {
                canvas
                    .state
                    .echo_tab_state
                    .is_echo_import_buffer_being_filled = false;
                canvas
                    .state
                    .echo_tab_state
                    .import_buffer
                    .lock()
                    .await
                    .clear();
                return Ok(());
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn handle_echo_search_key_event(
    canvas: &mut EchoCanvas,
    key_event: KeyEvent,
) -> EchoResult<()> {
    if canvas
        .state
        .echo_tab_state
        .is_echo_search_buffer_being_filled
    {
        match key_event.code {
            KeyCode::Char(c) => {
                canvas.state.echo_tab_state.search_buffer.push(c);
                return Ok(());
            }
            KeyCode::Enter => {
                canvas
                    .state
                    .echo_tab_state
                    .is_echo_search_buffer_being_filled = false;
                return Ok(());
            }
            _ => {}
        }
    }

    // delete confirmation prompt
    if canvas.state.echo_tab_state.is_confirm_delete {
        canvas.state.echo_tab_state.is_confirm_delete = false;
        if let KeyCode::Char('y') = key_event.code {
            if let Some(song) = canvas
                .state
                .local_songs
                .get(canvas.state.selected_song_pos)
                .cloned()
            {
                // stop playback if the selected song is the active one
                if canvas.state.active_track.path == song.path {
                    canvas.audio_player = AudioPlayer::bad();
                    canvas.audio_state = None;
                }

                let pos = canvas.state.selected_song_pos;
                canvas.state.local_songs.remove(pos);
                canvas.state.selected_song_pos = canvas
                    .state
                    .selected_song_pos
                    .min(canvas.state.local_songs.len().saturating_sub(1));

                let pool = canvas.db_connection_pool.clone();
                let reporter = canvas.state.report_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        db::repository::Repository::delete_song_by_path(&pool, &song.path).await
                    {
                        reporter
                            .send(Report {
                                log: Some(format!("DB delete error: {}", e)),
                                report: None,
                                level: LogLevel::ERR,
                            })
                            .ok();
                    }
                });
            }
        }
        return Ok(());
    }

    match key_event.code {
        KeyCode::Esc => {
            canvas
                .state
                .echo_tab_state
                .is_echo_search_buffer_being_filled = false;
        }
        KeyCode::Char('d') => {
            if !canvas.state.local_songs.is_empty() {
                canvas.state.echo_tab_state.is_confirm_delete = true;
            }
        }
        KeyCode::Char('w') => {
            if canvas.state.local_songs.len() == 0 {
                return Ok(());
            }
            canvas.state.previous_local_song()
        }
        KeyCode::Char('s') => {
            if canvas.state.local_songs.len() == 0 {
                return Ok(());
            }
            canvas.state.next_local_song()
        }
        KeyCode::Enter => match canvas.state.local_songs.get(canvas.state.selected_song_pos) {
            Some(v) => {
                let reporter = canvas.state.report_tx.clone();
                let audio_player = match AudioPlayer::new(&v.path) {
                    Ok(player) => player,
                    Err(e) => {
                        reporter
                            .send(Report {
                                log: Some(e.to_string()),
                                report: Some(crate::result::EchoReport::LockPoisoned(
                                    e.to_string(),
                                )),
                                level: LogLevel::ERR,
                            })
                            .ok();
                        AudioPlayer::bad()
                    }
                };
                canvas.state.active_track =
                    canvas.state.local_songs[canvas.state.selected_song_pos].to_owned();
                canvas.audio_player = audio_player;

                let mut audio_state = Some(canvas.audio_player.state.clone());
                if let Err(_) = canvas.audio_player.play() {
                    audio_state = None
                }
                canvas.audio_state = audio_state
            }
            None => {}
        },
        _ => {}
    }
    Ok(())
}

pub async fn handle_echo_metadata_key_event(
    canvas: &mut EchoCanvas,
    key_event: KeyEvent,
) -> EchoResult<()> {
    if canvas
        .state
        .echo_tab_state
        .is_echo_metadata_buffer_being_filled
    {
        match key_event.code {
            KeyCode::Enter => {
                let selected_song = &mut canvas.state.local_songs[canvas.state.selected_song_pos];
                match canvas.state.echo_tab_state.echo_metadata_selected_pos {
                    0 => {
                        selected_song.metadata.title = canvas.state.buffer.clone();
                    }
                    1 => {
                        selected_song.metadata.artist = canvas.state.buffer.clone();
                    }
                    2 => {
                        selected_song.metadata.album = canvas.state.buffer.clone();
                    }
                    3 => {
                        selected_song.metadata.year = canvas
                            .state
                            .buffer
                            .parse::<u32>()
                            .unwrap_or(selected_song.metadata.year);
                    }
                    4 => {
                        selected_song.metadata.genre = canvas.state.buffer.clone();
                    }
                    5 => {
                        selected_song.metadata.track_number = canvas
                            .state
                            .buffer
                            .parse::<u32>()
                            .unwrap_or(selected_song.metadata.track_number);
                    }
                    6 => {
                        selected_song.metadata.total_tracks = canvas
                            .state
                            .buffer
                            .parse::<u32>()
                            .unwrap_or(selected_song.metadata.total_tracks);
                    }
                    7 => {
                        selected_song.metadata.disc_number = canvas
                            .state
                            .buffer
                            .parse::<u32>()
                            .unwrap_or(selected_song.metadata.disc_number);
                    }
                    8 => {
                        selected_song.metadata.total_discs = canvas
                            .state
                            .buffer
                            .parse::<u32>()
                            .unwrap_or(selected_song.metadata.total_discs);
                    }
                    _ => {}
                }

                let metadata_to_save = selected_song.metadata.clone();
                let path_to_save = selected_song.path.clone();
                let reporter = canvas.state.report_tx.clone();
                let pool = canvas.db_connection_pool.clone();
                tokio::spawn(async move {
                    if let Err(e) = metadata_to_save.update_file(&path_to_save) {
                        let _ = reporter.send(Report {
                            log: Some(e.to_string()),
                            report: Some(EchoReport::AudioTagError(e)),
                            level: LogLevel::ERR,
                        });
                    } else {
                        // Also sync to DB
                        if let Err(e) =
                            db::update_song_metadata(&pool, &path_to_save, &metadata_to_save).await
                        {
                            let _ = reporter.send(Report {
                                log: Some(format!("DB sync error: {}", e)),
                                report: None,
                                level: LogLevel::ERR,
                            });
                        } else {
                            let _ = reporter.send(Report {
                                log: Some("METADATA WRITTEN SUCCESS".into()),
                                report: None,
                                level: LogLevel::INFO,
                            });
                        }
                    }
                });

                canvas
                    .state
                    .echo_tab_state
                    .is_echo_metadata_buffer_being_filled = false;
                canvas.state.buffer = String::new();

                return Ok(());
            }
            KeyCode::Char(c) => {
                canvas.state.buffer.push(c);
                return Ok(());
            }
            KeyCode::Backspace => {
                canvas.state.buffer.pop();
                return Ok(());
            }
            _ => return Ok(()),
        }
    }

    match key_event.code {
        KeyCode::Char('w') => {
            canvas.state.echo_tab_state.echo_metadata_selected_pos = canvas
                .state
                .echo_tab_state
                .echo_metadata_selected_pos
                .saturating_sub(1)
        }
        KeyCode::Char('s') => {
            canvas.state.echo_tab_state.echo_metadata_selected_pos =
                (canvas.state.echo_tab_state.echo_metadata_selected_pos + 1).min(8)
        }
        KeyCode::Enter => {
            canvas
                .state
                .echo_tab_state
                .is_echo_metadata_buffer_being_filled = true;
            let selected_song = &canvas.state.local_songs[canvas.state.selected_song_pos];
            let metadata = &selected_song.metadata;

            match canvas.state.echo_tab_state.echo_metadata_selected_pos {
                0 => canvas.state.buffer = metadata.title.clone(),
                1 => canvas.state.buffer = metadata.artist.clone(),
                2 => canvas.state.buffer = metadata.album.clone(),
                3 => canvas.state.buffer = metadata.year.to_string(),
                4 => canvas.state.buffer = metadata.genre.clone(),
                5 => canvas.state.buffer = metadata.track_number.to_string(),
                6 => canvas.state.buffer = metadata.total_tracks.to_string(),
                7 => canvas.state.buffer = metadata.disc_number.to_string(),
                8 => canvas.state.buffer = metadata.total_discs.to_string(),
                _ => canvas.state.buffer = String::new(),
            }
        }
        _ => {}
    }

    Ok(())
}
