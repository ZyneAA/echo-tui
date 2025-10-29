use std::io::{self, stdout};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ratatui::{
    Frame,
    crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

use crate::config::Config;

pub mod event;
pub mod widget;

pub struct Canvas {
    config: Config,
}

impl Canvas {
    pub fn init(config: Config) -> Self {
        Canvas { config }
    }

    pub async fn render(&mut self, exit: Arc<AtomicBool>) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;

        let mut terminal = ratatui::init();
        let exit = exit.load(Ordering::Relaxed);
        while !exit {
            if let Err(e) = terminal.draw(|frame| self.draw(frame)) {
                eprintln!("{e}");
            };
            if event::handle_events().await.unwrap() == true {
                break;
            };
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}
