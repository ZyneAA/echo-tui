use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style, Styled},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use strum::IntoEnumIterator;

use crate::{app::SelectedTab, awdio::song::Song};

pub fn bordered_block(title: Line<'static>, color: Color) -> Block<'static> {
    Block::bordered()
        .title(title)
        .border_set(border::ROUNDED)
        .style(Style::default().fg(color))
}

pub fn unbordered_block(title: Line<'static>) -> Block<'static> {
    Block::bordered().title(title).borders(Borders::empty())
}

pub fn paragraph(text: Vec<Line<'static>>, block: Block<'static>) -> Paragraph<'static> {
    Paragraph::new(text).block(block)
}

pub fn tabs(
    selected_tab: SelectedTab,
    block: Block<'static>,
    spinner: usize,
    animation_spinner: Vec<char>,
    fg: Color,
    accent: Color,
) -> Paragraph<'static> {
    let mut spans = vec![];
    for (i, title) in SelectedTab::iter().enumerate() {
        let is_selected = i == selected_tab as usize;
        let content = title.title();
        let span = if is_selected {
            Span::styled(
                format!(" {} {} ", content, animation_spinner[spinner]),
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!(" {} | ", content),
                Style::default().fg(accent).add_modifier(Modifier::DIM),
            )
        };
        spans.push(span);
    }

    Paragraph::new(Line::from(spans))
        .left_aligned()
        .block(block)
        .alignment(ratatui::layout::Alignment::Center)
}

pub fn local_songs_table(
    songs: &Vec<Song>,
    fg: Color,
    bg: Color,
    accent: Color,
    title: Color,
    selected_song_pos: &usize,
) -> Table<'static> {
    let selected_row_style = Style::default().add_modifier(Modifier::REVERSED).fg(title);

    let header = ["Name", "Artist", "Album"]
        .into_iter()
        .map(|s| Cell::from(Text::from(s).style(Style::default().fg(fg))))
        .collect::<Row>()
        .height(1);

    let rows = songs.iter().enumerate().map(|(i, data)| {
        let row_style = if i == *selected_song_pos {
            selected_row_style
        } else {
            Style::default().style().fg(fg)
        };

        let item = data.ref_array();

        item.into_iter()
            .map(|content| {
                Cell::from(Text::from(format!(
                    "\n{}\n",
                    content.as_deref().unwrap_or_default()
                )))
            })
            .collect::<Row>()
            .height(2)
            .style(row_style)
    });

    let t = Table::new(
        rows,
        [
            Constraint::Percentage(60),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ],
    )
    .header(header);
    t
}
