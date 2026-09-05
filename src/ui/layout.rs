use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use crate::ui::components;

use super::EchoCanvas;

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

        components::main_header(
            song_name_area,
            tab_area,
            header_area,
            body_area,
            buf,
            &self.state,
            &self.ui_config,
            &self.audio_state,
            &self.all_paths,
        );

        if self.state.is_confirm_exit {
            let prompt = Paragraph::new(
                Line::from(" EXIT? (y/n) ").style(
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .centered();
            let area = Rect {
                x: body_area.x,
                y: body_area.bottom().saturating_sub(1),
                width: body_area.width,
                height: 1,
            };
            prompt.render(area, buf);
        }
    }
}
