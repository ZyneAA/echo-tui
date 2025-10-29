use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

pub async fn handle_events() -> io::Result<bool> {
    let exit = match event::read()? {
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
            handle_key_event(key_event)
        }
        _ => false,
    };

    Ok(exit)
}

fn handle_key_event(key_event: KeyEvent) -> bool {
    match key_event.code {
        KeyCode::Char('q') => true,
        _ => false,
    }
}
