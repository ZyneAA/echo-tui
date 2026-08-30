use chrono::Local;
use sqlx::SqlitePool;
use std::{
    io::stdout,
    sync::{Arc, Mutex, mpsc::Receiver},
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

use crate::result::EchoResult;
use crate::{app::State, awdio::AudioData};
use crate::{
    app::{LogLevel, Report},
    awdio::AudioPlayer,
};
use crate::{config::UiConfig, ignite::Paths};

pub mod actions;
pub mod components;
pub mod layout;

pub struct EchoCanvas {
    pub state: State,
    pub ui_config: UiConfig,
    pub db_connection_pool: SqlitePool,
    pub audio_player: AudioPlayer,
    pub audio_state: Option<Arc<Mutex<AudioData>>>,
    pub report_rx: Receiver<Report>,
    pub all_paths: Paths,
}

impl EchoCanvas {
    pub fn init(
        state: State,
        config: UiConfig,
        db_connection: SqlitePool,
        audio_state: Option<Arc<Mutex<AudioData>>>,
        audio_player: AudioPlayer,
        report_rx: Receiver<Report>,
        all_paths: Paths,
    ) -> Self {
        EchoCanvas {
            state,
            ui_config: config,
            db_connection_pool: db_connection,
            audio_player,
            audio_state,
            report_rx,
            all_paths,
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

        // ── Main event loop ─────────────────────────────────────────────
        while !self.state.exit {
            // Drain any queued reports from background tasks
            while let Ok(report) = self.report_rx.try_recv() {
                self.state.current_report = Some(report);
            }

            tokio::select! {
                // 100ms tick — general UI refresh (currently a no-op placeholder)
                _ = ticker.tick() => {
                    // refresh ui
                }

                // 1s tick — clock + uptime display
                _ = timestamp_ticker.tick() => {
                    self.state.uptime += Duration::from_millis(1000);
                    self.state.uptime_readable = self.format_uptime();
                    self.current_time();
                }

                // 200ms tick — advance spinner, pulse, blink animations
                _ = amimation_ticker.tick() => {
                    self.update_animations_on_tick();
                }

                // Keyboard/mouse event from crossterm background task
                Some(evt) = event_rx.recv() => {
                    match self.handle_events(evt).await {
                        Ok(()) => {},
                        Err(e) => {
                            let reporter = self.state.report_tx.clone();
                            reporter.send(Report {
                                log: Some(e.to_string()),
                                report: Some(e),
                                level: LogLevel::ERR
                            }).ok();
                        }
                    }
                }
            }

            // Re-render the full terminal frame
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

        self.state.animations.is_blink = !self.state.animations.is_blink;
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
