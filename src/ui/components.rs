use ratatui::{
    style::{Color, Style},
    symbols::{self, border},
    text::Line,
    widgets::{Block, Padding, Paragraph, Tabs},
};
use strum::IntoEnumIterator;

use crate::app::SelectedTab;

pub fn bordered_block(title: Line<'static>, color: Color) -> Block<'static> {
    Block::bordered()
        .title(title)
        .border_set(symbols::border::PROPORTIONAL_TALL)
        .border_set(border::ROUNDED)
        .style(Style::default().fg(color))
}

pub fn paragraph(text: &'static str, block: Block<'static>) -> Paragraph<'static> {
    Paragraph::new(text).block(block)
}

pub fn tabs(selected_tab: SelectedTab) -> Tabs<'static> {
    let titles = SelectedTab::iter().map(SelectedTab::title);
    let highlight_style = (Color::default(), selected_tab.palette().c700);
    let selected_tab_index = selected_tab as usize;

    Tabs::new(titles)
        .highlight_style(highlight_style)
        .select(selected_tab_index)
        .padding("", "")
        .divider(" ")
}
