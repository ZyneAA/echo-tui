use crate::app::{DownloadState, LogLevel, PlaylistSubTab, Report};
use crate::awdio::AudioPlayer;
use crate::awdio::metadata::Metadata;
use crate::awdio::song::Song;
use crate::db;
use crate::download;
use crate::result::{EchoReport, EchoResult};
use crate::ui::EchoCanvas;
use crate::{app::SelectedTab, awdio::AudioData, awdio::skip};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

mod echo;

impl EchoCanvas {
    pub async fn handle_events(&mut self, evt: Event) -> EchoResult<()> {
        let exit = match evt {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event).await
            }
            _ => Ok(()),
        };

        exit
    }

    async fn handle_key_event(&mut self, key_event: KeyEvent) -> EchoResult<()> {
        match key_event.code {
            KeyCode::Esc => {
                // If we're in an input mode, cancel it; otherwise exit
                match self.state.selected_tab {
                    SelectedTab::Download => {
                        if matches!(self.state.download_state, DownloadState::InputUrl) {
                            self.state.download_state = DownloadState::Idle;
                            self.state.download_url_buffer.clear();
                            return Ok(());
                        }
                    }
                    SelectedTab::Playlist => {
                        if matches!(self.state.playlist_subtab, PlaylistSubTab::InputName) {
                            self.state.playlist_subtab = PlaylistSubTab::List;
                            self.state.playlist_name_buffer.clear();
                            return Ok(());
                        }
                        if matches!(self.state.playlist_subtab, PlaylistSubTab::Songs) {
                            self.state.playlist_subtab = PlaylistSubTab::List;
                            self.state.playlist_songs.clear();
                            self.state.selected_playlist_song_idx = 0;
                            return Ok(());
                        }
                    }
                    SelectedTab::Echo => {
                        if self.is_any_echo_buffer_active() {
                            self.deavtivate_all_echo_buffer();
                            return Ok(());
                        }
                    }
                    _ => {}
                }

                self.state.exit = true;
                return Ok(());
            }
            KeyCode::Right => {
                self.next_tab();
                return Ok(());
            }
            KeyCode::Left => {
                self.previous_tab();
                return Ok(());
            }
            _ => {}
        }

        match self.state.selected_tab {
            SelectedTab::Echo => echo::main_events::handle_echo_key_event(self, key_event).await?,
            SelectedTab::Download => self.handle_download_key_event(key_event).await?,
            SelectedTab::Playlist => self.handle_playlist_key_event(key_event).await?,
            _ => {}
        }
        Ok(())
    }

    // ── Download tab ─────────────────────────────────────────────

    async fn handle_download_key_event(&mut self, key_event: KeyEvent) -> EchoResult<()> {
        match &self.state.download_state {
            DownloadState::InputUrl => match key_event.code {
                KeyCode::Enter => {
                    if self.state.download_url_buffer.trim().is_empty() {
                        self.state.download_state = DownloadState::Idle;
                        return Ok(());
                    }

                    let url = self.state.download_url_buffer.clone();
                    let songs_dir = self.all_paths.songs.clone();
                    let pool = self.db_connection_pool.clone();
                    let reporter = self.state.report_tx.clone();

                    self.state.download_state = DownloadState::Downloading;

                    tokio::spawn(async move {
                        let _ = reporter.send(Report {
                            log: Some(format!("Downloading: {}", url)),
                            report: None,
                            level: LogLevel::INFO,
                        });

                        match download::download_mp3(&url, &songs_dir).await {
                            Ok(downloaded_path) => {
                                let path_str = downloaded_path.to_str().unwrap_or("").to_string();

                                // Read metadata from the downloaded file
                                let metadata = Metadata::from_path(&path_str).unwrap_or_default();

                                // Insert into DB
                                match db::insert_song(&pool, &metadata, &path_str).await {
                                    Ok(id) => {
                                        // Rename to {id}.mp3
                                        let new_name = format!("{}.mp3", id);
                                        let new_path = songs_dir.join(&new_name);

                                        if let Err(e) =
                                            tokio::fs::rename(&downloaded_path, &new_path).await
                                        {
                                            let _ = reporter.send(Report {
                                                log: Some(format!("Rename error: {}", e)),
                                                report: None,
                                                level: LogLevel::ERR,
                                            });
                                            return;
                                        }

                                        if let Some(new_path_str) = new_path.to_str() {
                                            // Update file_path in DB
                                            let _ =
                                                db::update_song_path(&pool, id, new_path_str).await;

                                            // Update tags in the renamed file
                                            let _ = metadata.update_file(new_path_str);
                                        }

                                        let _ = reporter.send(Report {
                                            log: Some(format!(
                                                "Downloaded: {} (id={})",
                                                metadata.title, id
                                            )),
                                            report: None,
                                            level: LogLevel::INFO,
                                        });
                                    }
                                    Err(e) => {
                                        let _ = reporter.send(Report {
                                            log: Some(format!("DB insert error: {}", e)),
                                            report: None,
                                            level: LogLevel::ERR,
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = reporter.send(Report {
                                    log: Some(format!("Download failed: {}", e)),
                                    report: None,
                                    level: LogLevel::ERR,
                                });
                            }
                        }
                    });

                    self.state.download_url_buffer.clear();
                    self.state.download_state = DownloadState::Idle;
                }
                KeyCode::Char(c) => {
                    self.state.download_url_buffer.push(c);
                }
                KeyCode::Backspace => {
                    self.state.download_url_buffer.pop();
                }
                _ => {}
            },
            _ => match key_event.code {
                KeyCode::Char('d') => {
                    self.state.download_state = DownloadState::InputUrl;
                    self.state.download_url_buffer.clear();
                }
                _ => {}
            },
        }
        Ok(())
    }

    // ── Playlist tab ─────────────────────────────────────────────

    async fn handle_playlist_key_event(&mut self, key_event: KeyEvent) -> EchoResult<()> {
        match &self.state.playlist_subtab {
            PlaylistSubTab::InputName => match key_event.code {
                KeyCode::Enter => {
                    let name = self.state.playlist_name_buffer.trim().to_string();
                    if !name.is_empty() {
                        let pool = self.db_connection_pool.clone();
                        let reporter = self.state.report_tx.clone();
                        match db::create_playlist(&pool, &name).await {
                            Ok(_id) => {
                                // Refresh playlists
                                if let Ok(pls) = db::get_all_playlists(&pool).await {
                                    self.state.playlists = pls;
                                }
                                let _ = reporter.send(Report {
                                    log: Some(format!("Created playlist: {}", name)),
                                    report: None,
                                    level: LogLevel::INFO,
                                });
                            }
                            Err(e) => {
                                let _ = reporter.send(Report {
                                    log: Some(format!("Create playlist error: {}", e)),
                                    report: None,
                                    level: LogLevel::ERR,
                                });
                            }
                        }
                    }
                    self.state.playlist_name_buffer.clear();
                    self.state.playlist_subtab = PlaylistSubTab::List;
                }
                KeyCode::Char(c) => {
                    self.state.playlist_name_buffer.push(c);
                }
                KeyCode::Backspace => {
                    self.state.playlist_name_buffer.pop();
                }
                _ => {}
            },
            PlaylistSubTab::List => match key_event.code {
                KeyCode::Char('n') => {
                    self.state.playlist_subtab = PlaylistSubTab::InputName;
                    self.state.playlist_name_buffer.clear();
                }
                KeyCode::Char('w') => {
                    self.state.selected_playlist_idx =
                        self.state.selected_playlist_idx.saturating_sub(1);
                }
                KeyCode::Char('s') => {
                    if !self.state.playlists.is_empty() {
                        self.state.selected_playlist_idx = (self.state.selected_playlist_idx + 1)
                            .min(self.state.playlists.len() - 1);
                    }
                }
                KeyCode::Char('d') => {
                    if let Some(playlist) =
                        self.state.playlists.get(self.state.selected_playlist_idx)
                    {
                        let pool = self.db_connection_pool.clone();
                        let pid = playlist.id;
                        let reporter = self.state.report_tx.clone();
                        let name = playlist.name.clone();
                        match db::delete_playlist(&pool, pid).await {
                            Ok(()) => {
                                if let Ok(pls) = db::get_all_playlists(&pool).await {
                                    self.state.playlists = pls;
                                }
                                if self.state.selected_playlist_idx > 0 {
                                    self.state.selected_playlist_idx -= 1;
                                }
                                let _ = reporter.send(Report {
                                    log: Some(format!("Deleted playlist: {}", name)),
                                    report: None,
                                    level: LogLevel::INFO,
                                });
                            }
                            Err(e) => {
                                let _ = reporter.send(Report {
                                    log: Some(format!("Delete error: {}", e)),
                                    report: None,
                                    level: LogLevel::ERR,
                                });
                            }
                        }
                    }
                }
                KeyCode::Char('a') => {
                    // Add current song from Echo tab to selected playlist
                    if let Some(playlist) =
                        self.state.playlists.get(self.state.selected_playlist_idx)
                    {
                        if let Some(song) = self.state.local_songs.get(self.state.selected_song_pos)
                        {
                            let pool = self.db_connection_pool.clone();
                            let pid = playlist.id;
                            let song_path = song.path.clone();
                            let reporter = self.state.report_tx.clone();
                            let song_title = song.metadata.title.clone();
                            let playlist_name = playlist.name.clone();
                            match db::add_song_to_playlist(&pool, pid, &song_path).await {
                                Ok(()) => {
                                    let _ = reporter.send(Report {
                                        log: Some(format!(
                                            "Added '{}' to '{}'",
                                            song_title, playlist_name
                                        )),
                                        report: None,
                                        level: LogLevel::INFO,
                                    });
                                }
                                Err(e) => {
                                    let _ = reporter.send(Report {
                                        log: Some(format!("Add song error: {}", e)),
                                        report: None,
                                        level: LogLevel::ERR,
                                    });
                                }
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    // Enter the playlist to view songs
                    if let Some(playlist) =
                        self.state.playlists.get(self.state.selected_playlist_idx)
                    {
                        let pool = self.db_connection_pool.clone();
                        let pid = playlist.id;
                        match db::get_playlist_song_paths(&pool, pid).await {
                            Ok(paths) => {
                                let songs: Vec<Song> = paths
                                    .iter()
                                    .filter_map(|p| {
                                        if std::path::Path::new(p).exists() {
                                            Some(Song::new(p.clone()))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                self.state.playlist_songs = songs;
                                self.state.selected_playlist_song_idx = 0;
                                self.state.playlist_subtab = PlaylistSubTab::Songs;
                            }
                            Err(e) => {
                                let reporter = self.state.report_tx.clone();
                                let _ = reporter.send(Report {
                                    log: Some(format!("Load songs error: {}", e)),
                                    report: None,
                                    level: LogLevel::ERR,
                                });
                            }
                        }
                    }
                }
                KeyCode::Char('R') => {
                    // Refresh playlists from DB
                    let pool = self.db_connection_pool.clone();
                    if let Ok(pls) = db::get_all_playlists(&pool).await {
                        self.state.playlists = pls;
                    }
                }
                _ => {}
            },
            PlaylistSubTab::Songs => match key_event.code {
                KeyCode::Char('w') => {
                    self.state.selected_playlist_song_idx =
                        self.state.selected_playlist_song_idx.saturating_sub(1);
                }
                KeyCode::Char('s') => {
                    if !self.state.playlist_songs.is_empty() {
                        self.state.selected_playlist_song_idx =
                            (self.state.selected_playlist_song_idx + 1)
                                .min(self.state.playlist_songs.len() - 1);
                    }
                }
                KeyCode::Char('r') => {
                    // Remove song from playlist
                    if let Some(playlist) =
                        self.state.playlists.get(self.state.selected_playlist_idx)
                    {
                        if let Some(song) = self
                            .state
                            .playlist_songs
                            .get(self.state.selected_playlist_song_idx)
                        {
                            let pool = self.db_connection_pool.clone();
                            let pid = playlist.id;
                            let song_path = song.path.clone();
                            let reporter = self.state.report_tx.clone();
                            match db::remove_song_from_playlist(&pool, pid, &song_path).await {
                                Ok(()) => {
                                    // Refresh
                                    if let Ok(paths) = db::get_playlist_song_paths(&pool, pid).await
                                    {
                                        self.state.playlist_songs = paths
                                            .iter()
                                            .filter_map(|p| {
                                                if std::path::Path::new(p).exists() {
                                                    Some(Song::new(p.clone()))
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();
                                    }
                                    if self.state.selected_playlist_song_idx > 0 {
                                        self.state.selected_playlist_song_idx -= 1;
                                    }
                                    let _ = reporter.send(Report {
                                        log: Some("Removed song from playlist".into()),
                                        report: None,
                                        level: LogLevel::INFO,
                                    });
                                }
                                Err(e) => {
                                    let _ = reporter.send(Report {
                                        log: Some(format!("Remove error: {}", e)),
                                        report: None,
                                        level: LogLevel::ERR,
                                    });
                                }
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    // Play selected song
                    if let Some(song) = self
                        .state
                        .playlist_songs
                        .get(self.state.selected_playlist_song_idx)
                    {
                        let reporter = self.state.report_tx.clone();
                        let audio_player = match AudioPlayer::new(&song.path) {
                            Ok(player) => player,
                            Err(e) => {
                                reporter
                                    .send(Report {
                                        log: Some(e.to_string()),
                                        report: Some(EchoReport::LockPoisoned(e.to_string())),
                                        level: LogLevel::ERR,
                                    })
                                    .ok();
                                AudioPlayer::bad()
                            }
                        };
                        self.state.active_track = song.clone();
                        self.audio_player = audio_player;
                        let mut audio_state = Some(self.audio_player.state.clone());
                        if let Err(_) = self.audio_player.play() {
                            audio_state = None;
                        }
                        self.audio_state = audio_state;
                    }
                }
                KeyCode::Backspace => {
                    self.state.playlist_subtab = PlaylistSubTab::List;
                    self.state.playlist_songs.clear();
                    self.state.selected_playlist_song_idx = 0;
                }
                _ => {}
            },
        }
        Ok(())
    }

    fn skip_audio(&mut self, amount: f64) -> EchoResult<()> {
        let _ = self.with_audio_state(|state| {
            let _ = skip(state, amount);
        });

        Ok(())
    }

    fn adjust_volume(&mut self, amount: f32) -> EchoResult<()> {
        self.with_audio_state(|state| state.volume = (state.volume + amount).clamp(0.0, 1.0))
    }

    fn toggle_pause(&mut self) -> EchoResult<()> {
        self.with_audio_state(|state| state.is_pause = !state.is_pause)
    }

    fn with_audio_state<F>(&self, f: F) -> EchoResult<()>
    where
        F: FnOnce(&mut AudioData),
    {
        if let Some(audio_arc_mutex) = &self.audio_state {
            let mut state = audio_arc_mutex
                .lock()
                .map_err(|e| EchoReport::LockPoisoned(e.to_string()))?;
            f(&mut state);
        }

        Ok(())
    }

    pub fn next_tab(&mut self) {
        self.state.selected_tab = self.state.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.state.selected_tab = self.state.selected_tab.previous();
    }

    pub fn is_any_echo_buffer_active(&self) -> bool {
        self.state.echo_tab_state.is_echo_search_buffer_being_filled
            || self.state.echo_tab_state.is_echo_import_buffer_being_filled
            || self
                .state
                .echo_tab_state
                .is_echo_metadata_buffer_being_filled
    }

    fn deavtivate_all_echo_buffer(&mut self) {
        let state = &mut self.state.echo_tab_state;
        state.is_echo_search_buffer_being_filled = false;
        state.is_echo_import_buffer_being_filled = false;
        state.is_echo_metadata_buffer_being_filled = false;
    }
}
