use std::{path::Path, sync::Arc};

use tokio::fs;

use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    app::{LogLevel, Report},
    awdio::{AudioPlayer, metadata::Metadata},
    db,
    result::{EchoReport, EchoResult},
    ui::EchoCanvas,
};

pub async fn handle_echo_import_key_enent(
    canvas: &mut EchoCanvas,
    key_event: KeyEvent,
) -> EchoResult<()> {
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
                let pool = canvas.db_connection_pool.clone();

                let song_path_arc = Arc::clone(&canvas.state.echo_tab_state.import_buffer);

                tokio::spawn(async move {
                    let song_path = {
                        let guard = song_path_arc.lock().await;
                        guard.clone()
                    };

                    let mut entries = match fs::read_dir(&song_path).await {
                        Ok(e) => e,
                        Err(e) => {
                            eprintln!("read_dir error: {:?}", e);
                            return;
                        }
                    };

                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let old_path = entry.path();
                        if old_path.is_dir() {
                            continue;
                        }
                        if old_path.extension().and_then(|s| s.to_str()) != Some("mp3") {
                            continue;
                        }

                        let stem = old_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                        let path_str = match old_path.to_str() {
                            Some(p) => p,
                            None => {
                                eprintln!("invalid path encoding");
                                continue;
                            }
                        };

                        let mut tag = match Metadata::from_path(path_str) {
                            Ok(t) => t,
                            Err(e) => {
                                eprintln!("metadata error: {:?}", e);
                                continue;
                            }
                        };

                        let db_title = if tag.title.is_empty() {
                            stem
                        } else {
                            &tag.title
                        };

                        let id = match sqlx::query!(
                            "INSERT INTO songs (title, artist, album, file_path) VALUES (?, ?, ?, ?)",
                            db_title, tag.artist, tag.album, "PENDING"
                        )
                        .execute(&pool).await {
                            Ok(res) => res.last_insert_rowid(),
                            Err(e) => {
                                eprintln!("db insert error: {:?}", e);
                                continue;
                            }
                        };

                        let new_file_name = format!("{}{}.mp3", song_path, id);
                        let new_path = Path::new(&new_file_name);

                        if let Err(e) = fs::rename(&old_path, &new_path).await {
                            eprintln!("rename error: {:?}", e);
                            continue;
                        }

                        tag.title = db_title.to_string();
                        if let Some(new_path_str) = new_path.to_str() {
                            let _ = tag.update_file(new_path_str);
                            let _ = sqlx::query!(
                                "UPDATE songs SET file_path = ? WHERE id = ?",
                                new_path_str,
                                id
                            )
                            .execute(&pool)
                            .await;
                        }
                    }
                });

                return Ok(());
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

    match key_event.code {
        KeyCode::Esc => {
            canvas
                .state
                .echo_tab_state
                .is_echo_search_buffer_being_filled = false;
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
