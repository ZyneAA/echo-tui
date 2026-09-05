use tokio::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app::{EchoSubTab, SelectedTab},
    awdio::metadata::Metadata,
    event::echo::sub_events,
    result::EchoResult,
    ui::EchoCanvas,
};

pub async fn handle_echo_key_event(canvas: &mut EchoCanvas, key_event: KeyEvent) -> EchoResult<()> {
    if canvas
        .state
        .echo_tab_state
        .is_echo_metadata_buffer_being_filled
    {
        return sub_events::handle_echo_metadata_key_event(canvas, key_event).await;
    }
    if canvas
        .state
        .echo_tab_state
        .is_echo_search_buffer_being_filled
    {
        return sub_events::handle_echo_search_key_event(canvas, key_event);
    } else if canvas
        .state
        .echo_tab_state
        .is_echo_download_buffer_being_filled
    {
        return sub_events::handle_echo_download_key_event(canvas, key_event).await;
    } else if canvas
        .state
        .echo_tab_state
        .is_echo_import_buffer_being_filled
    {
        return sub_events::handle_echo_import_key_enent(canvas, key_event).await;
    }

    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('I'), _) | (KeyCode::Char('i'), KeyModifiers::SHIFT) => {
            canvas.state.echo_tab_state.prev_sub_state = EchoSubTab::IMPORT;
            canvas.state.switch_echo_subtab('I');
        }
        (KeyCode::Char('D'), _) | (KeyCode::Char('d'), KeyModifiers::SHIFT) => {
            canvas.state.echo_tab_state.prev_sub_state = EchoSubTab::DOWNLOAD;
            canvas.state.switch_echo_subtab('D');
            sub_events::reload_download_history(canvas).await;
        }
        (KeyCode::Char('S'), _) | (KeyCode::Char('s'), KeyModifiers::SHIFT) => {
            canvas.state.echo_tab_state.prev_sub_state = EchoSubTab::SEARCH;
            canvas.state.switch_echo_subtab('S');
        }
        (KeyCode::Char('M'), _) | (KeyCode::Char('m'), KeyModifiers::SHIFT) => {
            canvas.state.switch_echo_subtab('M');
        }

        (KeyCode::Char('|'), KeyModifiers::NONE) => {
            let pool = canvas.db_connection_pool.clone();
            let song_path = canvas.all_paths.songs.clone();

            tokio::spawn(async move {
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
                    if stem.parse::<i64>().is_ok() {
                        continue;
                    }

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
                        db_title,
                        tag.artist,
                        tag.album,
                        "PENDING"
                    )
                    .execute(&pool)
                    .await
                    {
                        Ok(res) => res.last_insert_rowid(),
                        Err(e) => {
                            eprintln!("db insert error: {:?}", e);
                            continue;
                        }
                    };

                    let new_file_name = format!("{}.mp3", id);
                    let new_path = song_path.join(&new_file_name);

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
        }

        (KeyCode::Char('f'), _) => {
            // in Echo>Search, f opens the filter picker instead of FFT toggle
            if canvas.state.selected_tab == SelectedTab::Echo
                && canvas.state.echo_tab_state.echo_subtab == EchoSubTab::SEARCH
            {
                canvas.state.echo_tab_state.search_filter_selecting = true;
            } else {
                let mut ok = canvas.audio_player.state.lock().unwrap();
                ok.enable_fft_compute = !ok.enable_fft_compute;
            }
        }
        (KeyCode::Char('P') | KeyCode::Char('p'), _) => canvas.toggle_pause()?,
        (KeyCode::Char('K') | KeyCode::Char('k'), _) => canvas.adjust_volume(0.1)?,
        (KeyCode::Char('J') | KeyCode::Char('j'), _) => canvas.adjust_volume(-0.1)?,
        (KeyCode::Char('h'), _) => canvas.skip_audio(-1.0)?,
        (KeyCode::Char('l'), _) => canvas.skip_audio(1.0)?,

        (KeyCode::Char('i'), _) => match canvas.state.echo_tab_state.echo_subtab {
            EchoSubTab::SEARCH => {
                // i -> straight into query input (filter defaults to ALL)
                canvas
                    .state
                    .echo_tab_state
                    .is_echo_search_buffer_being_filled = true;
                return Ok(());
            }
            EchoSubTab::IMPORT => {
                canvas
                    .state
                    .echo_tab_state
                    .is_echo_import_buffer_being_filled = true;

                return Ok(());
            }

            EchoSubTab::METADATA => {
                canvas.state.echo_tab_state.metadata_buffer.clear();
                canvas
                    .state
                    .echo_tab_state
                    .is_echo_metadata_buffer_being_filled = true
            }
            EchoSubTab::DOWNLOAD => {
                canvas
                    .state
                    .echo_tab_state
                    .is_echo_download_buffer_being_filled = true
            }
        },

        _ => match canvas.state.echo_tab_state.echo_subtab {
            EchoSubTab::SEARCH => {
                return sub_events::handle_echo_search_key_event(canvas, key_event);
            }
            EchoSubTab::IMPORT => {
                return sub_events::handle_echo_import_key_enent(canvas, key_event).await;
            }
            EchoSubTab::METADATA => {
                return sub_events::handle_echo_metadata_key_event(canvas, key_event).await;
            }
            EchoSubTab::DOWNLOAD => {
                return sub_events::handle_echo_download_key_event(canvas, key_event).await;
            }
        },
    }

    Ok(())
}
