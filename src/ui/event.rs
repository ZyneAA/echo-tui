use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use super::Canvas;

impl Canvas {
    pub async fn handle_events(&mut self) -> io::Result<()> {
        let exit = match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {},
        };

        Ok(exit)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.state.exit = true,
            KeyCode::Char('l') => self.next_tab(),
            KeyCode::Char('h') => self.previous_tab(),
            _ => {},
        }
    }

    pub fn next_tab(&mut self) {
        self.state.selected_tab = self.state.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.state.selected_tab = self.state.selected_tab.previous();
    }
}

