use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::{Cell, Paragraph, Row, Table, Widget};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{
        block::Title,
        canvas::{Canvas, Points},
    },
};
use toml::to_string;

use crate::app::EchoSubTab;
use crate::ui::components::shared;
use crate::{app::EchoTabState, awdio::song::Song, config::UiConfig};

pub fn render_echo(
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
    config: &UiConfig,
    songs: &Vec<Song>,
    selected_song_pos: &usize,
    current_song: &Song,
    echo_tab_state: &EchoTabState,
    songs_path: &PathBuf,
) {
    let info = config.colors["colors"].info;
    let title = config.colors["colors"].title;
    let bg = config.colors["colors"].bg;
    let buffer = &echo_tab_state.metadata_buffer;
    let import_buf = echo_tab_state.import_buffer.clone();

    let chunks = if echo_tab_state.is_fft_enable {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(0), Constraint::Percentage(100)])
            .split(area)
    };

    let ttf_area = chunks[0];
    let body_area = chunks[1];

    if echo_tab_state.is_fft_enable {
        let title_ttf = Line::from(" ▪︎ ");
        let ttf_block = shared::block::bordered_block(title_ttf, low_color)
            .border_style(Style::new().fg(high_color))
            .title_style(Style::new().fg(mid_color));
        let fft_data: Vec<f64> = fft_state.iter().map(|value| *value as f64).collect();

        let inner_area = ttf_block.inner(ttf_area);
        let width = inner_area.width as f64;
        let height = (inner_area.height as f64) + 50.0;

        let middle = height / 2.0;

        let gradient_start =
            hex_to_rgb(&config.colors["colors"].fg.to_string()).unwrap_or((0, 0, 0));
        let gradient_mid =
            hex_to_rgb(&config.colors["colors"].title.to_string()).unwrap_or((0, 0, 0));
        let gradient_stop =
            hex_to_rgb(&config.colors["colors"].border.to_string()).unwrap_or((255, 255, 255));

        let gradient = gradient_steps(gradient_start, gradient_mid, gradient_stop, 32);

        let mut all_points = vec![];

        for (i_idx, i) in fft_data.iter().enumerate() {
            for j in 0..(*i as usize) {
                let upper = j as f64 + middle;
                let height_percent = upper / height;

                let level_idx =
                    (height_percent.clamp(0.0, 1.0) * (gradient.len() - 1) as f64) as usize;

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
    }

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(body_area);
    let left_area = body[0];
    let right_area = body[1];

    let echo_main_title = match echo_tab_state.echo_subtab {
        EchoSubTab::IMPORT => echo_main_title_import(info, title, bg),
        EchoSubTab::SEARCH => echo_main_title_search(info, title, bg),
        EchoSubTab::DOWNLOAD => echo_main_title_download(info, title, bg),
        _ => echo_main_title_metadata(info, title, bg),
    };

    match echo_tab_state.echo_subtab {
        EchoSubTab::DOWNLOAD => {}
        EchoSubTab::IMPORT => {
            render_import_subtab(
                left_area,
                buf,
                echo_main_title.clone(),
                config,
                echo_tab_state.import_buffer.clone(),
                info,
                title,
                echo_tab_state,
            );
        }
        EchoSubTab::SEARCH => {
            render_search_subtab(
                left_area,
                buf,
                echo_main_title.clone(),
                config,
                &echo_tab_state.search_buffer,
                info,
                title,
                echo_tab_state,
                songs_path,
                songs,
                selected_song_pos,
            );
        }
        _ => match echo_tab_state.prev_sub_state {
            EchoSubTab::SEARCH => {
                render_search_subtab(
                    left_area,
                    buf,
                    echo_main_title.clone(),
                    config,
                    &echo_tab_state.search_buffer,
                    info,
                    title,
                    echo_tab_state,
                    songs_path,
                    songs,
                    selected_song_pos,
                );
            }
            EchoSubTab::IMPORT => {
                render_import_subtab(
                    left_area,
                    buf,
                    echo_main_title.clone(),
                    config,
                    import_buf,
                    info,
                    title,
                    echo_tab_state,
                );
            }
            _ => {}
        },
    }

    let info_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(right_area);
    let upper_area = info_layout[0];
    let lower_area = info_layout[1];

    let app_info =
        Line::from(" PLAYLISTS ").style(Style::default().fg(config.colors["colors"].title));
    shared::block::bordered_block(
        app_info,
        ratatui::style::Color::from(config.colors["colors"].border),
    )
    .render(upper_area, buf);

    let metadata_title = match echo_tab_state.echo_subtab {
        EchoSubTab::METADATA => Line::from(vec![
            Span::styled(" M", Style::default().fg(title).bg(info)),
            Span::styled("ETADATA ⌬ ·· ", Style::default().fg(title).bg(info)),
        ]),
        _ => Line::from(vec![
            Span::styled(
                " M",
                Style::default().fg(config.colors["colors"].info).bg(title),
            ),
            Span::styled("ETADATA ⌬ ·· ", Style::default().fg(bg).bg(title)),
        ]),
    };

    let metadata_block = shared::block::bordered_block(
        metadata_title,
        ratatui::style::Color::from(config.colors["colors"].border),
    )
    .title_bottom(Line::from(vec![
        Span::styled(
            " BUFF: ",
            Style::default().fg(config.colors["colors"].title),
        ),
        Span::styled(
            format!("{} ", buffer),
            Style::default().fg(config.colors["colors"].title),
        ),
    ]));

    if echo_tab_state.is_zero_local_song || songs.is_empty() {
        let empty_msg = Paragraph::new("NO SONGS FOUND.")
            .style(Style::default().fg(config.colors["colors"].fg))
            .centered()
            .block(metadata_block);

        empty_msg.render(lower_area, buf);
        return;
    }

    let selected_song_metadata = &songs[*selected_song_pos].metadata;

    let year_binding = &to_string(&selected_song_metadata.year).unwrap_or_default();
    let track_number_binding = &to_string(&selected_song_metadata.track_number).unwrap_or_default();
    let total_tracks_binding = &to_string(&selected_song_metadata.total_tracks).unwrap_or_default();
    let disc_number_binding = &to_string(&selected_song_metadata.disc_number).unwrap_or_default();
    let total_discs_binding = &to_string(&selected_song_metadata.total_discs).unwrap_or_default();
    let metadata = vec![
        ("TITLE", &selected_song_metadata.title),
        ("ARTIST", &selected_song_metadata.artist),
        ("ALBUM", &selected_song_metadata.album),
        ("YEAR", year_binding),
        ("GENERE", &selected_song_metadata.genre),
        ("TRACK NUMBER", track_number_binding),
        ("TOTAL TRACK", total_tracks_binding),
        ("DISC NUMBER", disc_number_binding),
        ("TOTAL DISC", total_discs_binding),
    ];
    let table = shared::table::echo_metadata_table(
        metadata,
        echo_tab_state.echo_metadata_selected_pos,
        &echo_tab_state.echo_subtab,
        config.colors["colors"].title,
        config.colors["colors"].fg,
    );

    table.block(metadata_block).render(lower_area, buf);
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

fn echo_main_title_search<'a>(info: Color, title: Color, bg: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(" S", Style::default().bg(info).fg(title)),
        Span::styled("EARCH ·· ", Style::default().bg(info).fg(title)),
        Span::styled("|", Style::default().bg(title).fg(info)),
        Span::styled(" I", Style::default().bg(title).fg(info)),
        Span::styled("MPORT + ", Style::default().bg(title).fg(bg)),
        Span::styled("|", Style::default().bg(title).fg(info)),
        Span::styled(" D", Style::default().bg(title).fg(info)),
        Span::styled("OWNLOAD ▼ ", Style::default().bg(title).fg(bg)),
    ])
}

fn echo_main_title_import<'a>(info: Color, title: Color, bg: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(" S", Style::default().bg(title).fg(info)),
        Span::styled("EARCH ·· ", Style::default().bg(title).fg(bg)),
        Span::styled("|", Style::default().bg(title).fg(info)),
        Span::styled(" I", Style::default().bg(info).fg(title)),
        Span::styled("MPORT + ", Style::default().bg(info).fg(title)),
        Span::styled("|", Style::default().bg(title).fg(info)),
        Span::styled(" D", Style::default().bg(title).fg(info)),
        Span::styled("OWNLOAD ▼ ", Style::default().bg(title).fg(bg)),
    ])
}

fn echo_main_title_download<'a>(info: Color, title: Color, bg: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(" S", Style::default().bg(title).fg(info)),
        Span::styled("EARCH ·· ", Style::default().bg(title).fg(bg)),
        Span::styled("|", Style::default().bg(title).fg(info)),
        Span::styled(" I", Style::default().bg(title).fg(info)),
        Span::styled("MPORT + ", Style::default().bg(title).fg(bg)),
        Span::styled("|", Style::default().bg(title).fg(info)),
        Span::styled(" D", Style::default().bg(info).fg(title)),
        Span::styled("OWNLOAD ▼ ", Style::default().bg(info).fg(title)),
    ])
}

fn echo_main_title_metadata<'a>(info: Color, title: Color, bg: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(" S", Style::default().bg(title).fg(info)),
        Span::styled("EARCH ·· ", Style::default().bg(title).fg(bg)),
        Span::styled("|", Style::default().bg(title).fg(info)),
        Span::styled(" I", Style::default().bg(title).fg(info)),
        Span::styled("MPORT + ", Style::default().bg(title).fg(bg)),
        Span::styled("|", Style::default().bg(title).fg(info)),
        Span::styled(" D", Style::default().bg(title).fg(info)),
        Span::styled("OWNLOAD ▼ ", Style::default().bg(title).fg(info)),
    ])
}

fn render_import_subtab(
    left_area: Rect,
    buf: &mut Buffer,
    echo_main_title: Line<'static>,
    config: &UiConfig, // Adjust this type if needed
    buffer: Arc<Mutex<String>>,
    info: ratatui::style::Color,
    title: ratatui::style::Color,
    echo_tab_state: &EchoTabState,
) {
    let outer_block = shared::block::bordered_block(
        echo_main_title,
        ratatui::style::Color::from(config.colors["colors"].border),
    )
    .title_bottom(" ⎔  ⎔  FOUND:")
    .title_style(Style::default().fg(config.colors["colors"].title));

    let inner_area = outer_block.inner(left_area);
    outer_block.render(left_area, buf);

    let import_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner_area);

    let guard = buffer.try_lock();
    let buf_ref: &str = match &guard {
        Ok(g) => g.as_str(),
        Err(_) => "",
    };
    let input_block =
        shared::block::inner_input_block(buf_ref, info, title, &echo_tab_state.echo_subtab, true);

    let input_widget = Paragraph::new(buf_ref)
        .block(input_block)
        .style(Style::default().fg(info));

    input_widget.render(import_layout[0], buf);

    let list_area = import_layout[1];

    if !echo_tab_state.import_file_list.is_empty() {
        let fg = config.colors["colors"].fg;
        let sel_pos = echo_tab_state.import_file_selected_pos;

        let rows = echo_tab_state
            .import_file_list
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let marker = if echo_tab_state
                    .import_selected
                    .get(i)
                    .copied()
                    .unwrap_or(false)
                {
                    "[x]"
                } else {
                    "[ ]"
                };
                let name = std::path::Path::new(f)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(f);
                let row_style = if i == sel_pos {
                    Style::default().add_modifier(Modifier::REVERSED).fg(title)
                } else {
                    Style::default().fg(fg)
                };
                Row::new(vec![Cell::from(Text::from(format!(
                    " {} {}",
                    marker, name
                )))])
                .height(1)
                .style(row_style)
            });

        let table = Table::new(rows, [Constraint::Percentage(100)]).block(
            shared::block::bordered_block(
                Line::from(" FILES "),
                ratatui::style::Color::from(config.colors["colors"].border),
            )
            .title_bottom(" w/s nav · SPACE select · a all · ENTER import · ESC cancel ")
            .title_style(Style::default().fg(config.colors["colors"].title)),
        );

        table.render(list_area, buf);
    }

    if echo_tab_state.is_confirm_import {
        let n = echo_tab_state
            .import_selected
            .iter()
            .filter(|s| **s)
            .count();
        let prompt = Paragraph::new(
            Line::from(format!(" IMPORT {} FILE(S)? (y/n) ", n)).style(
                Style::default()
                    .fg(config.colors["colors"].bg)
                    .bg(config.colors["colors"].accent),
            ),
        );
        let area = Rect {
            x: inner_area.x,
            y: inner_area.bottom().saturating_sub(1),
            width: inner_area.width,
            height: 1,
        };
        prompt.render(area, buf);
    }
}

fn render_search_subtab<'a>(
    left_area: Rect,
    buf: &mut Buffer,
    echo_main_title: Line<'static>,
    config: &UiConfig,
    buffer: &String,
    info: ratatui::style::Color,
    title: ratatui::style::Color,
    echo_tab_state: &EchoTabState,
    songs_path: &Path,
    songs: &Vec<Song>,
    selected_song_pos: &usize,
) {
    let proj = songs_path.to_string_lossy();
    let outer_block = shared::block::bordered_block(
        echo_main_title,
        ratatui::style::Color::from(config.colors["colors"].border),
    )
    .title_bottom(" ⎔  ⎔  FROM:")
    .title_bottom(proj)
    .title_style(Style::default().fg(config.colors["colors"].title));

    let inner_area = outer_block.inner(left_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box height
            Constraint::Min(0),    // Table takes the rest
        ])
        .split(inner_area);

    outer_block.render(left_area, buf);

    let input_block =
        shared::block::inner_input_block(buffer, info, title, &echo_tab_state.echo_subtab, true);

    let input_widget = Paragraph::new(buffer.as_str())
        .block(input_block)
        .style(Style::default().fg(info));

    input_widget.render(chunks[0], buf);

    let table = shared::table::local_songs_table(
        songs,
        config.colors["colors"].fg,
        config.colors["colors"].bg,
        config.colors["colors"].accent,
        config.colors["colors"].title,
        selected_song_pos,
        &echo_tab_state.echo_subtab,
    );

    table.render(chunks[1], buf);

    if echo_tab_state.is_confirm_delete {
        let prompt = Paragraph::new(
            Line::from(" DELETE SONG FROM DB? (y/n) ").style(
                Style::default()
                    .fg(config.colors["colors"].bg)
                    .bg(config.colors["colors"].accent),
            ),
        );
        let area = Rect {
            x: inner_area.x,
            y: inner_area.bottom().saturating_sub(1),
            width: inner_area.width,
            height: 1,
        };
        prompt.render(area, buf);
    }
}
