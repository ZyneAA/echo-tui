use std::io;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use super::EchoCanvas;
use crate::awdio::skip;

impl EchoCanvas {
    pub async fn handle_events(&mut self, evt: Event) -> io::Result<()> {
        let exit = match evt {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };

        Ok(exit)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => self.state.exit = true,
            KeyCode::Right => self.next_tab(),
            KeyCode::Left => self.previous_tab(),
            KeyCode::Char('P') => {
                let mut state = self.audio_state.lock().unwrap();
                state.is_pause = !state.is_pause;
            }
            KeyCode::Char('K') => {
                let mut audio = self.audio_state.lock().unwrap();
                audio.volume += 0.1;
            }
            KeyCode::Char('J') => {
                let mut audio = self.audio_state.lock().unwrap();
                audio.volume = (audio.volume - 0.1).max(0.0);
            }
            KeyCode::Char('H') => {
                skip(self.audio_state.clone(), -10.0);
            }
            KeyCode::Char('L') => {
                skip(self.audio_state.clone(), 10.0);
            }
            _ => {
                let key = key_event.code.to_string();
                self.state.append_input(&key);
            }
        }
    }

    pub fn next_tab(&mut self) {
        self.state.selected_tab = self.state.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.state.selected_tab = self.state.selected_tab.previous();
    }
}
