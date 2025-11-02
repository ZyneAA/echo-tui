use std::io::{self, stdout};

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
use crate::app::State;

pub mod canvas;
pub mod components;
pub mod event;

pub struct EchoCanvas {
    state: State,
    config: Config,
    fft_fake: Vec<(String, u64)>,
}

impl EchoCanvas {
    pub fn init(state: State, config: Config) -> Self {
        EchoCanvas {
            state,
            config,
            fft_fake: vec![],
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
        let mut fake_timestamp_ticker: Interval = time::interval(Duration::from_millis(1000));

        tokio::spawn(async move {
            loop {
                if let Ok(event) = tokio::task::spawn_blocking(|| crossterm::event::read()).await {
                    if let Ok(evt) = event {
                        let _ = event_tx.send(evt);
                    }
                }
            }
        });

        self.fft_fake = (0..175)
            .map(|i| (format!("{i}"), rand::random_range(1..5)))
            .collect();

        while !self.state.exit {
            for (_, val) in self.fft_fake.iter_mut() {
                let change = rand::random_range(0..5);
                if rand::random_bool(0.5) {
                    *val = val.saturating_add(change);
                } else {
                    *val = val.saturating_sub(change);
                }
                *val = (*val).max(1).min(30);
            }

            tokio::select! {
                _ = ticker.tick() => {
                    // Does nothing now, only use for refrashing the UI
                }

                _ = fake_timestamp_ticker.tick() => {
                    self.caluate_current_timestamp();
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

    fn caluate_current_timestamp(&mut self) {
        let total_ms = self.state.animations.timestamp.1.as_millis();
        let current_ms = self.state.animations.timestamp.0.as_millis();

        let position: usize = if total_ms == 0 {
            // Song unloaded
            0
        } else if total_ms == current_ms {
            self.state.animations.timestamp.0 += Duration::from_millis(1000);
            49
        } else {
            let percentage_raw = (current_ms * 50) / total_ms;
            self.state.animations.timestamp.0 += Duration::from_millis(1000);
            (percentage_raw as usize).min(50)
        };

        for i in 0..position {
            self.state.animations.animation_timestamp.vals[i] =
                self.config.animations["animations"].timestamp_bar.clone();
        }
        self.state.animations.animation_timestamp.vals[position] =
            self.config.animations["animations"].timestamp.clone();

        self.state.animations.timestamp_location = position;

        if current_ms >= total_ms {
            self.state.animations.timestamp.0 = Duration::from_millis(0);
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}
