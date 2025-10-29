use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::Widget,
};

use crate::app::SelectedTab;

use super::Canvas;
use super::components;

impl Widget for &Canvas {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(2), Constraint::Percentage(98)])
            .split(area);

        let header_area = chunks[0];
        let body_area = chunks[1];

        components::tabs(self.state.selected_tab).render(header_area, buf);
        self.state.selected_tab.render(body_area, buf);
    }
}

impl Widget for SelectedTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            Self::Echo => render_echo(area, buf),
            Self::Playlist => render_playlist(area, buf),
            Self::Download => render_echo(area, buf),
            Self::Misc => render_echo(area, buf),
        }
    }
}

fn render_echo(area: Rect, buf: &mut Buffer) {
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

    let title_songs = Line::from(" songs ");
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
