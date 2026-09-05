use std::path::Path;

use ratatui::{
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders},
};

use crate::app::{EchoSubTab, SearchFilter};

pub fn inner_input_block<'a>(
    input: &'a str,
    fg: Color,
    title_color: Color,
    echo_subtab: &EchoSubTab,
    is_focused: bool,
    filter: &SearchFilter,
) -> Block<'a> {
    let block_style;
    match (echo_subtab, is_focused) {
        (EchoSubTab::IMPORT, true) => {
            block_style = Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD);
        }
        (EchoSubTab::SEARCH, true)
        | (EchoSubTab::METADATA, true)
        | (EchoSubTab::DOWNLOAD, true) => {
            block_style = Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD);
        }
        (_, true) => {
            block_style = Style::default().fg(fg).add_modifier(Modifier::REVERSED);
        }
        _ => {
            block_style = Style::default().fg(fg);
        }
    }

    let file_name_hint = Path::new(input)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("...");

    let title_line = if matches!(echo_subtab, EchoSubTab::SEARCH) {
        // filter options with the active one highlighted; no query echo
        let mut spans = vec![Span::styled(" [ FILTER: ", Style::default().fg(fg))];
        for (i, opt) in SearchFilter::OPTIONS.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" · ", Style::default().fg(fg)));
            }
            if *opt == *filter {
                spans.push(Span::styled(
                    format!("{} ", opt.label()),
                    Style::default()
                        .fg(title_color)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                ));
            } else {
                spans.push(Span::styled(
                    opt.label().to_string(),
                    Style::default().fg(fg),
                ));
            }
        }
        spans.push(Span::styled(" ] ", Style::default().fg(fg)));
        Line::from(spans)
    } else {
        Line::from(vec![
            Span::styled(" [ ", Style::default().fg(fg)),
            Span::styled(
                "FILE PATH",
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" | {} ] ", file_name_hint), Style::default().fg(fg)),
        ])
    };

    Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(block_style)
        .title(title_line)
}

pub fn bordered_block(title: Line<'static>, color: Color) -> Block<'static> {
    Block::bordered()
        .title(title)
        .border_set(border::ROUNDED)
        .style(Style::default().fg(color))
}

pub fn unbordered_block(title: Line<'static>) -> Block<'static> {
    Block::bordered().title(title).borders(Borders::empty())
}
