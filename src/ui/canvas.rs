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

use crate::app::SelectedTab;

use super::EchoCanvas;
use super::components;

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
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(tab_area);

        // Rendering starts here
        let text = vec![
            "Title: This is the day".into(),
            "Artist: Someone".into(),
            "Album: Something".into(),
        ];

        let is_playing_status = format!(
            "status: playing {} {}",
            self.config.animations["animations"].hpulse[self.state.animations.animation_hpulse.0],
            self.state.animations.animation_timestamp.vals.len()
        );
        let title_block = components::bordered_block(
            Line::from(vec![Span::raw(is_playing_status)]),
            self.config.colors["colors"].border,
        )
        .title(Line::from(" | ").right_aligned())
        .padding(Padding::horizontal(1))
        .title_bottom("size: 12.3mb")
        .title_bottom(Line::from("length: 12:49").right_aligned())
        .title_style(Style::new().fg(self.config.colors["colors"].title));

        components::paragraph(text, title_block)
            .style(Style::default().fg(self.config.colors["colors"].fg))
            .render(song_name_area, buf);

        let timestamp_block =
            components::bordered_block(Line::default(), self.config.colors["colors"].border)
                .title_style(Style::new().fg(self.config.colors["colors"].title))
                .title(Line::from("uptime: 21290ms").right_aligned())
                .title(Line::from(" ⟐  ").left_aligned())
                .title(Line::from("12:21:43").centered())
                .title_bottom(Line::from("volume: 30"))
                .title_bottom(Line::from("using: macbook's speaker").centered())
                .title_bottom(Line::from("tick rate: 100ms").right_aligned());
        let timestamp_bar: String = self.state.animations.animation_timestamp.vals.join("");
        let total_length = self.state.animations.timestamp.1.as_millis();
        let current_timestamp = self.state.animations.timestamp.0.as_millis();
        let timestamp = format!("{}⧏ |⧐ {}", current_timestamp, total_length);

        let temp = self.state.animations.timestamp_location;
        let temp = make_gradient_bar(temp);

        components::paragraph(
            vec![Line::from(timestamp_bar), Line::from(timestamp), temp],
            timestamp_block,
        )
        .style(Style::default().fg(self.config.colors["colors"].fg))
        .centered()
        .render(header_area[1], buf);

        let tab_block =
            components::bordered_block(Line::default(), self.config.colors["colors"].border)
                .title(" ● ")
                .title_bottom(" ○ ○ ○ ")
                .title_style(Style::new().fg(self.config.colors["colors"].title))
                .border_type(ratatui::widgets::BorderType::Rounded);

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

        self.state.selected_tab.render_tabs(
            body_area,
            buf,
            self.fft_fake.clone(),
            self.config.colors["colors"].info,
            self.config.colors["colors"].title,
            self.config.colors["colors"].border,
        );
    }
}

impl SelectedTab {
    pub fn render_tabs(
        self,
        area: Rect,
        buf: &mut Buffer,
        fft_fake: Vec<(String, u64)>,
        low: Color,
        medium: Color,
        high: Color,
    ) {
        match self {
            Self::Echo => render_echo(area, buf, fft_fake, low, medium, high),
            Self::Playlist => render_playlist(area, buf),
            Self::Download => render_playlist(area, buf),
            Self::Misc => render_playlist(area, buf),
        }
    }
}

fn render_echo(
    area: Rect,
    buf: &mut Buffer,
    fft_fake: Vec<(String, u64)>,
    low_color: Color,
    medium_color: Color,
    high_color: Color,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);
    let ttf_area = chunks[0];
    let body_area = chunks[1];

    ///// CHNAGE HERE LATER /////
    let title_ttf = Line::from(" ▪︎ ");
    let ttf_block = components::bordered_block(title_ttf, low_color);
    let fft_fake: &Vec<(String, u64)> = &fft_fake;
    let fft_data: Vec<(&str, u64)> = fft_fake
        .iter()
        .map(|(label, value)| (label.as_str(), *value))
        .collect();

    let inner_area = ttf_block.inner(ttf_area);
    let width = inner_area.width as f64;
    let height = (inner_area.height as f64) + 50.0;

    let mut level_1 = Vec::new();
    let mut level_2 = Vec::new();
    let mut level_3 = Vec::new();
    let mut level_4 = Vec::new();
    let mut level_5 = Vec::new();

    let mut counter = 0.0;
    let middle = height / 2.0;

    let gradient_start = hex_to_rgb(&low_color.to_string()).unwrap_or((0, 0, 0));
    let gradient_stop = hex_to_rgb(&high_color.to_string()).unwrap_or((255, 255, 255));
    let gradient = gradient_steps(gradient_start, gradient_stop, 5);

    for (_, i) in fft_data {
        for j in 0..i {
            let upper = j as f64 + middle;
            let height_percent = upper / height;
            if height_percent > 0.4 && height_percent < 0.45 {
                level_2.push((counter, j as f64 + middle));
                level_2.push((counter, height - middle - j as f64));
            } else if height_percent > 0.45 && height_percent < 0.55 {
                level_3.push((counter, j as f64 + middle));
                level_3.push((counter, height - middle - j as f64));
            } else if height_percent > 0.56 && height_percent <= 0.8 {
                level_4.push((counter, j as f64 + middle));
                level_4.push((counter, height - middle - j as f64));
            } else if height_percent > 0.81 {
                level_5.push((counter, j as f64 + middle));
                level_5.push((counter, height - middle - j as f64));
            } else {
                level_1.push((counter, j as f64 + middle));
                level_1.push((counter, height - middle - j as f64));
            }
        }
        counter += 1.0;
    }

    let temp = format!(
        " {} {} {} {:?} ",
        width,
        height,
        low_color.to_string(),
        gradient_stop
    );
    let title = Title::from(temp);
    Canvas::default()
        .block(ttf_block.title(title))
        .x_bounds([0.0, width])
        .y_bounds([0.0, height])
        .paint(|ctx| {
            ctx.layer();
            ctx.draw(&Points {
                coords: &level_1,
                color: Color::Rgb(gradient[0].0, gradient[0].1, gradient[0].2),
            });
            ctx.draw(&Points {
                coords: &level_2,
                color: Color::Rgb(gradient[1].0, gradient[1].1, gradient[1].2),
            });
            ctx.draw(&Points {
                coords: &level_3,
                color: Color::Rgb(gradient[2].0, gradient[2].1, gradient[2].2),
            });
            ctx.draw(&Points {
                coords: &level_4,
                color: Color::Rgb(gradient[3].0, gradient[3].1, gradient[3].2),
            });
            ctx.draw(&Points {
                coords: &level_5,
                color: Color::Rgb(gradient[4].0, gradient[4].1, gradient[4].2),
            });
        })
        .render(ttf_area, buf);
    //////////////////////////////

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(body_area);
    let left_area = body[0];
    let right_area = body[1];

    let title_songs = Line::from("∥ songs ∥");
    components::bordered_block(title_songs, ratatui::style::Color::Red).render(left_area, buf);

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

fn make_gradient_bar(progress: usize) -> Line<'static> {
    let length = 50;
    let filled = length as usize * progress;

    // start and end colors (teal → magenta)
    let start = (0, 255, 200);
    let end = (255, 0, 150);

    let mut spans = Vec::new();
    for i in 0..length {
        // linear interpolation between start and end
        let t = i as f32 / (length - 1) as f32;
        let r = (start.0 as f32 + (end.0 as f32 - start.0 as f32) * t) as u8;
        let g = (start.1 as f32 + (end.1 as f32 - start.1 as f32) * t) as u8;
        let b = (start.2 as f32 + (end.2 as f32 - start.2 as f32) * t) as u8;

        let symbol = if i < filled { "+" } else { "·" }; // dot for unfilled part
        spans.push(Span::styled(
            symbol,
            Style::default().fg(Color::Rgb(r, g, b)),
        ));
    }

    Line::from(spans)
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
    end: (usize, usize, usize),
    steps: usize,
) -> Vec<(u8, u8, u8)> {
    let mut result = Vec::new();
    for i in 0..steps {
        let t = i as f64 / (steps - 1) as f64; // fraction 0.0 → 1.0
        let r = start.0 as f64 + (end.0 as f64 - start.0 as f64) * t;
        let g = start.1 as f64 + (end.1 as f64 - start.1 as f64) * t;
        let b = start.2 as f64 + (end.2 as f64 - start.2 as f64) * t;
        result.push((r as u8, g as u8, b as u8));
    }

    result
}
