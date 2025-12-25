use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{
        Padding, Widget,
        block::Title,
        canvas::{Canvas, Points},
    },
};

use crate::{
    app::SelectedTab,
    awdio::{DurationInfo, song::Song},
    config::Config,
};

use super::EchoCanvas;
use super::components;
use crate::awdio::current_timestamp;

impl Widget for &EchoCanvas {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(10), Constraint::Percentage(90)])
            .split(area);

        let body_area = chunks[1];

        let header_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(chunks[0]);

        let song_name_area = header_area[0];
        let tab_area = header_area[2];
        let tab_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(tab_area);

        let (
            samples,
            sample_rate,
            channels,
            file_size,
            duration,
            host,
            is_pause,
            volume,
            total_samples_played,
            min_buffer_threshold,
            fft,
        ) = {
            match &self.audio_state {
                Some(v) => {
                    let audio = v.lock().unwrap();
                    (
                        audio.samples.len(),
                        audio.sample_rate,
                        audio.channels,
                        audio.file_size.clone(),
                        audio.duration.clone(),
                        audio.host.clone(),
                        audio.is_pause,
                        audio.volume,
                        audio.total_samples_played,
                        audio.min_buffer_threshold,
                        audio.fft_state.clone(),
                    )
                }
                None => (
                    0,
                    0,
                    0,
                    String::new(),
                    DurationInfo::default(),
                    String::new(),
                    false,
                    0.0,
                    0,
                    0,
                    Vec::new(),
                ),
            }
        };
        let max_samples = (duration.seconds * sample_rate as u64) as i64;
        let timestamp = current_timestamp(total_samples_played, sample_rate);
        let timestamp_percent = ((timestamp.1 / duration.seconds as f64) * 100.0)
            .ceil()
            .min(100.0);
        let position = (((timestamp_percent / 100.0) * 50.0) as usize).min(49);

        // Rendering starts here
        let text = vec![
            "Title: This is the day".into(),
            "Artist: Someone".into(),
            "Album: Something".into(),
        ];

        let is_playing_status = format!(
            " PLAYING: {} {} - ●  {}",
            !is_pause,
            self.config.animations["animations"].hpulse[self.state.animations.animation_hpulse.0],
            self.state.selected_song_pos
        );
        let title_block = components::bordered_block(
            Line::from(vec![Span::raw(is_playing_status)]),
            self.config.colors["colors"].border,
        )
        .title(Line::from(" | ").right_aligned())
        .padding(Padding::horizontal(1))
        .title_bottom(Line::from(format!(" SIZE: {} ", file_size)))
        .title_bottom(Line::from(format!(" CLK: {} ", duration.readable)).right_aligned())
        .title_style(Style::new().fg(self.config.colors["colors"].title));

        components::paragraph(text, title_block)
            .style(Style::default().fg(self.config.colors["colors"].fg))
            .render(song_name_area, buf);

        let timestamp_block =
            components::bordered_block(Line::default(), self.config.colors["colors"].border)
                .title_style(Style::new().fg(self.config.colors["colors"].title))
                .title(
                    Line::from(format!(" UPTIME: {} ", self.state.uptime_readable)).right_aligned(),
                )
                .title(Line::from(" ⟐  ").left_aligned())
                .title(Line::from(self.state.current_clock.clone()).centered())
                .title_bottom(Line::from(format!(" VOL: {:.1} ", volume)))
                .title_bottom(Line::from(format!(" HOST: {} ", host)).centered())
                .title_bottom(Line::from(" TICK: 100ms ").right_aligned());

        let mut anim = self.state.animations.animation_timestamp.borrow_mut();
        for i in 0..=anim.vals.len() - 1 {
            if i == position {
                anim.vals[position] = self.config.animations["animations"].timestamp.clone();
                continue;
            }
            anim.vals[i] = self.config.animations["animations"].timestamp_bar.clone();
        }
        drop(anim);

        let timestamp_bar: String = self
            .state
            .animations
            .animation_timestamp
            .borrow_mut()
            .vals
            .join("");
        let timestamp = format!(
            "{} ■ {} :|{:02}%|: {} ■ {}",
            timestamp.1 as u64, timestamp.0, timestamp_percent, duration.readable, duration.seconds
        );

        components::paragraph(
            vec![Line::from(timestamp_bar), Line::from(timestamp)],
            timestamp_block,
        )
        .style(Style::default().fg(self.config.colors["colors"].fg))
        .centered()
        .render(header_area[1], buf);

        let tab_block =
            components::bordered_block(Line::default(), self.config.colors["colors"].border)
                .title(" ● ")
                .title_style(Style::new().fg(self.config.colors["colors"].title));

        let spinner = self.config.animations["animations"].spinner.clone();

        components::tabs(
            self.state.selected_tab,
            tab_block,
            self.state.animations.animation_spinner.0,
            spinner,
            self.config.colors["colors"].fg,
            self.config.colors["colors"].accent,
        )
        .render(tab_area[0], buf);

        let report = self.report_rx.try_recv().unwrap_or_default();
        let report_log = match report.log {
            Some(e) => e.to_string(),
            None => "".into()
        };
        components::unbordered_block(Line::from(format!("{}", report_log)))
            .title_style(Style::default().fg(self.config.colors["colors"].error))
            .render(tab_area[1], buf);

        let config = &self.config;

        match self.state.selected_tab {
            SelectedTab::Echo => render_echo(
                body_area,
                buf,
                samples,
                sample_rate,
                channels,
                total_samples_played,
                max_samples as u64,
                min_buffer_threshold,
                fft,
                self.config.colors["colors"].fg,
                self.config.colors["colors"].title,
                self.config.colors["colors"].border,
                config,
                &self.state.local_songs,
                &self.state.selected_song_pos,
            ),
            SelectedTab::Playlist => render_playlist(body_area, buf),
            SelectedTab::Download => render_playlist(body_area, buf),
            SelectedTab::Misc => render_playlist(body_area, buf),
        }
    }
}

fn render_echo(
    area: Rect,
    buf: &mut Buffer,
    sample_buffer_size: usize,
    sample_rate: u32,
    channels: u16,
    total_samples_played: u64,
    max_samples: u64,
    min_buffer_threshold: usize,
    fft_state: Vec<f32>,
    low_color: Color,
    mid_color: Color,
    high_color: Color,
    config: &Config,
    songs: &Vec<Song>,
    selected_song_pos: &usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);
    let ttf_area = chunks[0];
    let body_area = chunks[1];

    let title_ttf = Line::from(" ▪︎ ");
    let ttf_block = components::bordered_block(title_ttf, low_color)
        .border_style(Style::new().fg(high_color))
        .title_style(Style::new().fg(mid_color));
    let fft_data: Vec<f64> = fft_state.iter().map(|value| *value as f64).collect();

    let inner_area = ttf_block.inner(ttf_area);
    let width = inner_area.width as f64;
    let height = (inner_area.height as f64) + 50.0;

    let middle = height / 2.0;

    let gradient_start = hex_to_rgb(&config.colors["colors"].fg.to_string()).unwrap_or((0, 0, 0));
    let gradient_mid = hex_to_rgb(&config.colors["colors"].title.to_string()).unwrap_or((0, 0, 0));
    let gradient_stop =
        hex_to_rgb(&config.colors["colors"].border.to_string()).unwrap_or((255, 255, 255));

    let gradient = gradient_steps(gradient_start, gradient_mid, gradient_stop, 32);

    let mut all_points = vec![];

    for (i_idx, i) in fft_data.iter().enumerate() {
        for j in 0..(*i as usize) {
            let upper = j as f64 + middle;
            let height_percent = upper / height;

            let level_idx = (height_percent.clamp(0.0, 1.0) * (gradient.len() - 1) as f64) as usize;

            all_points.push(((i_idx as f64), upper, gradient[level_idx]));
            all_points.push((
                (i_idx as f64),
                height - middle - j as f64,
                gradient[level_idx],
            ));
        }
    }

    Canvas::default()
        .block(
            ttf_block
                .title_bottom(Line::from(format!(
                    " ○ ○ SAMPLE_POS: {} / {} • ",
                    total_samples_played, max_samples
                )))
                .title_bottom(
                    Line::from(format!(
                        " • • MIN_BUF_THRESHOLD: {} ⋯ ",
                        min_buffer_threshold
                    ))
                    .right_aligned(),
                )
                .title(Title::from(format!(
                    " ■ SAMPLE_BUF: {} // SAMPLE_RATE: {} // BUS: {}X ",
                    sample_buffer_size, sample_rate, channels
                ))),
        )
        .x_bounds([0.0, width])
        .y_bounds([-50.0, height + 50.0])
        .paint(|ctx| {
            ctx.layer();

            for (x, y, color) in all_points.iter() {
                let (r, g, b) = color.clone();
                let main_color = Color::Rgb(r, g, b);
                let fade_color = Color::Rgb(r / 2, g / 2, b / 2);

                ctx.draw(&Points {
                    coords: &[(*x, *y)],
                    color: main_color,
                });
                ctx.draw(&Points {
                    coords: &[(x + 0.5, y + 0.5)],
                    color: fade_color,
                });
            }
        })
        .render(ttf_area, buf);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(body_area);
    let left_area = body[0];
    let right_area = body[1];

    let title_songs =
        Line::from(" SEARCH ").style(Style::default().fg(config.colors["colors"].title));
    let local_songs_block = components::bordered_block(
        title_songs,
        ratatui::style::Color::from(config.colors["colors"].border),
    );

    components::local_songs_table(
        songs,
        config.colors["colors"].fg,
        config.colors["colors"].bg,
        config.colors["colors"].accent,
        config.colors["colors"].title,
        selected_song_pos,
    )
    .block(local_songs_block)
    .render(left_area, buf);

    let info = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(right_area);
    let upper_area = info[0];
    let lower_area = info[1];

    let app_info = Line::from(" info ");
    components::bordered_block(app_info, ratatui::style::Color::Red).render(upper_area, buf);

    let metadata = Line::from(" metadata ");
    components::bordered_block(metadata, ratatui::style::Color::Red).render(lower_area, buf);
}

fn render_playlist(area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);
    let ttf_area = chunks[0];
    let body_area = chunks[1];

    let title_ttf = Line::from(" TTF ");
    components::bordered_block(title_ttf, ratatui::style::Color::Red).render(ttf_area, buf);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(body_area);
    let left_area = body[0];
    let right_area = body[1];

    let title_songs = Line::from(" Playlist ");
    components::bordered_block(title_songs, ratatui::style::Color::Red).render(left_area, buf);

    let title_metadata = Line::from(" metadata ");
    components::bordered_block(title_metadata, ratatui::style::Color::Red).render(right_area, buf);
}

fn hex_to_rgb(hex: &str) -> Option<(usize, usize, usize)> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);

    if hex.len() != 6 {
        return None;
    }

    let r = usize::from_str_radix(&hex[0..2], 16).ok()?;
    let g = usize::from_str_radix(&hex[2..4], 16).ok()?;
    let b = usize::from_str_radix(&hex[4..6], 16).ok()?;

    Some((r, g, b))
}

fn gradient_steps(
    start: (usize, usize, usize),
    mid: (usize, usize, usize),
    end: (usize, usize, usize),
    steps: usize,
) -> Vec<(u8, u8, u8)> {
    let mut result = Vec::new();
    let half = steps / 2;

    // first half: start → mid
    for i in 0..half {
        let t = i as f64 / (half - 1) as f64;
        let r = start.0 as f64 + (mid.0 as f64 - start.0 as f64) * t / 2.0;
        let g = start.1 as f64 + (mid.1 as f64 - start.1 as f64) * t / 2.0;
        let b = start.2 as f64 + (mid.2 as f64 - start.2 as f64) * t / 2.0;
        result.push((r as u8, g as u8, b as u8));
    }

    // second half: mid → end
    for i in 0..(steps - half) {
        let t = i as f64 / (steps - half - 1) as f64;
        let r = mid.0 as f64 + (end.0 as f64 - mid.0 as f64) * t / 2.0;
        let g = mid.1 as f64 + (end.1 as f64 - mid.1 as f64) * t / 2.0;
        let b = mid.2 as f64 + (end.2 as f64 - mid.2 as f64) * t / 2.0;
        result.push((r as u8, g as u8, b as u8));
    }

    result
}
