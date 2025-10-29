use std::io::{self, stdout};

use ratatui::{
    Frame,
    crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

use crate::app;
use crate::config::Config;

pub mod canvas;
pub mod components;
pub mod event;

pub struct Canvas {
    state: app::State,
    config: Config,
}

impl Canvas {
    pub fn init(state: app::State, config: Config) -> Self {
        Canvas { state, config }
    }

    pub async fn paint(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;

        let mut terminal = ratatui::init();

        while !self.state.exit {
            if let Err(e) = terminal.draw(|frame| self.draw(frame)) {
                eprintln!("{}", e);
            };

            if let Err(e) = self.handle_events().await {
                eprintln!("{}", e);
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
