use chrono::Local;
use std::{
    io::stdout,
    sync::{mpsc::Receiver, Arc, Mutex},
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

use crate::{app::{LogLevel, Report}, awdio::AudioPlayer};
use crate::config::Config;
use crate::result::EchoResult;
use crate::{app::State, awdio::AudioData};

pub mod canvas;
pub mod components;
pub mod event;

pub struct EchoCanvas {
    state: State,
    config: Config,
    audio_player: AudioPlayer,
    audio_state: Option<Arc<Mutex<AudioData>>>,
    report_rx: Receiver<Report>
}

impl EchoCanvas {
    pub fn init(
        state: State,
        config: Config,
        audio_state: Option<Arc<Mutex<AudioData>>>,
        audio_player: AudioPlayer,
        report_rx: Receiver<Report>
    ) -> Self {
        EchoCanvas {
            state,
            config,
            audio_player,
            audio_state,
            report_rx
        }
    }

    pub async fn paint(&mut self) -> EchoResult<()> {
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
                    // refresh ui
                }

                _ = timestamp_ticker.tick() => {
                    self.state.uptime += Duration::from_millis(1000);
                    self.state.uptime_readable = self.format_uptime();
                    self.current_time();
                }

                _ = amimation_ticker.tick() => {
                    self.update_animations_on_tick();
                }

                Some(evt) = event_rx.recv() => {
                    match self.handle_events(evt).await {
                        Ok(()) => {},
                        Err(e) => {
                            let reporter = self.state.report_tx.clone();
                            reporter.send(Report {
                                log: Some(e),
                                level: LogLevel::ERR
                            }).ok();
                        }
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

    fn format_uptime(&mut self) -> String {
        let total_secs = self.state.uptime.as_secs();

        let days = total_secs / 86_400;
        let hours = (total_secs % 86_400) / 3_600;
        let minutes = (total_secs % 3_600) / 60;
        let seconds = total_secs % 60;

        match (days, hours, minutes) {
            (0, 0, 0) => format!("{:02}s", seconds),
            (0, 0, _) => format!("{:02}m {:02}s", minutes, seconds),
            (0, _, _) => format!("{:02}h {:02}m {:02}s", hours, minutes, seconds),
            (_, _, _) => format!("{}d {:02}h {:02}m", days, hours, minutes),
        }
    }

    fn current_time(&mut self) {
        let now = Local::now();
        self.state.current_clock = now.format(" // %H:%M:%S // ").to_string();
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}
