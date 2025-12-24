use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use super::EchoCanvas;
use crate::app::LogLevel;
use crate::result::{EchoError, EchoResult};
use crate::{
    app::SelectedTab,
    awdio::{AudioPlayer, skip},
};

impl EchoCanvas {
    pub async fn handle_events(&mut self, evt: Event) -> EchoResult<()> {
        let exit = match evt {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => Ok(()),
        };

        exit
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> EchoResult<()> {
        match key_event.code {
            KeyCode::Esc => self.state.exit = true,
            KeyCode::Right => self.next_tab(),
            KeyCode::Left => self.previous_tab(),

            KeyCode::Char('w') => self.state.previous_local_song(),
            KeyCode::Char('s') => self.state.next_local_song(),

            KeyCode::Enter => match self.state.selected_tab {
                SelectedTab::Echo => {
                    match self.state.local_songs.get(self.state.selected_song_pos) {
                        Some(v) => {
                            let reporter = self.state.report.clone();
                            let audio_player = match AudioPlayer::new(&v.path) {
                                Ok(player) => player,
                                Err(e) => {
                                    reporter
                                        .lock()
                                        .map_err(|e| EchoError::LockPoisoned(e.to_string()))?
                                        .log = e.to_string().to_uppercase();
                                    reporter
                                        .lock()
                                        .map_err(|e| EchoError::LockPoisoned(e.to_string()))?
                                        .level = LogLevel::ERR;
                                    AudioPlayer::bad()
                                }
                            };
                            self.audio_player = audio_player;

                            let mut audio_state = Some(self.audio_player.state.clone());
                            if let Err(_) = self.audio_player.play() {
                                audio_state = None
                            }
                            self.audio_state = audio_state
                        }
                        None => {}
                    }
                }
                _ => {}
            },

            KeyCode::Char('P')
            | KeyCode::Char('K')
            | KeyCode::Char('J')
            | KeyCode::Char('h')
            | KeyCode::Char('l')
            | KeyCode::Char('H')
            | KeyCode::Char('L') => {
                if let Some(audio_arc_mutex) = &self.audio_state {
                    match key_event.code {
                        KeyCode::Char('P') => {
                            let mut state = audio_arc_mutex
                                .lock()
                                .map_err(|e| EchoError::LockPoisoned(e.to_string()))?;
                            state.is_pause = !state.is_pause;
                        }
                        KeyCode::Char('K') => {
                            let mut audio = audio_arc_mutex
                                .lock()
                                .map_err(|e| EchoError::LockPoisoned(e.to_string()))?;
                            audio.volume = (audio.volume + 0.1).min(1.0);
                        }
                        KeyCode::Char('J') => {
                            let mut audio = audio_arc_mutex
                                .lock()
                                .map_err(|e| EchoError::LockPoisoned(e.to_string()))?;
                            audio.volume = (audio.volume - 0.1).max(0.0);
                        }
                        KeyCode::Char('h') => {
                            skip(audio_arc_mutex.clone(), -1.0)
                                .map_err(|e| EchoError::LockPoisoned(e.to_string()))?;
                        }
                        KeyCode::Char('l') => {
                            skip(audio_arc_mutex.clone(), 1.0)
                                .map_err(|e| EchoError::LockPoisoned(e.to_string()))?;
                        }
                        KeyCode::Char('H') => {
                            skip(audio_arc_mutex.clone(), -10.0)
                                .map_err(|e| EchoError::LockPoisoned(e.to_string()))?;
                        }
                        KeyCode::Char('L') => {
                            skip(audio_arc_mutex.clone(), 10.0)
                                .map_err(|e| EchoError::LockPoisoned(e.to_string()))?;
                        }
                        _ => unreachable!(),
                    }
                }
            }
            _ => {
                let key = key_event.code.to_string();
                self.state.append_input(&key);
            }
        }

        Ok(())
    }

    pub fn next_tab(&mut self) {
        self.state.selected_tab = self.state.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.state.selected_tab = self.state.selected_tab.previous();
    }
}
