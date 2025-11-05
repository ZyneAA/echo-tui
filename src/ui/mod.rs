use std::{
    io::{self, stdout},
    sync::{Arc, Mutex},
};

use ratatui::{
    Frame,
    crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::{
    sync::mpsc::UnboundedSender,
    time::{self, Duration, Interval},
};

use crate::config::Config;
use crate::{app::State, awdio::AudioData};

pub mod canvas;
pub mod components;
pub mod event;

pub struct EchoCanvas {
    state: State,
    config: Config,
    audio_state: Arc<Mutex<AudioData>>,
}

impl EchoCanvas {
    pub fn init(state: State, config: Config, audio_state: Arc<Mutex<AudioData>>) -> Self {
        EchoCanvas {
            state,
            config,
            audio_state,
        }
    }

    pub async fn paint(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;

        let mut terminal = ratatui::init();

        let (event_tx, mut event_rx): (
            UnboundedSender<crossterm::event::Event>,
            UnboundedReceiver<crossterm::event::Event>,
        ) = tokio::sync::mpsc::unbounded_channel();

        let mut ticker: Interval = time::interval(Duration::from_millis(100));
        let mut amimation_ticker: Interval = time::interval(Duration::from_millis(200));
        let mut timestamp_ticker: Interval = time::interval(Duration::from_millis(1000));

        tokio::spawn(async move {
            loop {
                if let Ok(event) = tokio::task::spawn_blocking(|| crossterm::event::read()).await {
                    if let Ok(evt) = event {
                        let _ = event_tx.send(evt);
                    }
                }
            }
        });

        while !self.state.exit {
            tokio::select! {
                _ = ticker.tick() => {
                    self.state.uptime += Duration::from_millis(100);
                }

                _ = timestamp_ticker.tick() => {
                }

                _ = amimation_ticker.tick() => {
                    self.update_animations_on_tick();
                }

                Some(evt) = event_rx.recv() => {
                    match self.handle_events(evt).await {
                        Ok(()) => {},
                        Err(e) => eprintln!("{}", e)
                    }
                }
            }
            let _ = terminal.draw(|frame| self.draw(frame));
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        ratatui::restore();
        Ok(())
    }

    fn increment_frame_index(frame: &mut (usize, usize)) {
        // frame.0 is the current index
        // frame.1 is the maximum length (non-inclusive)

        if frame.0 < frame.1.saturating_sub(1) {
            frame.0 += 1;
        } else {
            frame.0 = 0;
        }
    }

    fn update_animations_on_tick(&mut self) {
        Self::increment_frame_index(&mut self.state.animations.animation_spinner);
        Self::increment_frame_index(&mut self.state.animations.animation_hpulse);
        Self::increment_frame_index(&mut self.state.animations.animation_dot);
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}
