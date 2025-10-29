use std::{io, path::PathBuf};

use ratatui::{
    style::{palette::tailwind, Stylize},
    text::Line,
};
use strum::{Display, EnumIter, FromRepr};
use tokio::task::JoinHandle;

use super::ui;
use crate::config::Config;

#[derive(Debug, Default)]
pub struct State {
    pub exit: bool,
    pub selected_tab: SelectedTab,
}

impl State {
    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }
}

#[derive(Default, Debug, Clone, Copy, Display, FromRepr, EnumIter)]
pub enum SelectedTab {
    #[default]
    #[strum(to_string = "Echo")]
    Echo,
    #[strum(to_string = "Playlist")]
    Playlist,
    #[strum(to_string = "Download")]
    Download,
    #[strum(to_string = "Misc")]
    Misc,
}

impl SelectedTab {
    pub fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    pub const fn palette(self) -> tailwind::Palette {
        match self {
            Self::Echo => tailwind::BLUE,
            Self::Playlist => tailwind::EMERALD,
            Self::Download => tailwind::INDIGO,
            Self::Misc => tailwind::RED,
        }
    }

    pub fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    pub fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }
}

pub async fn start(data: (Config, PathBuf)) -> io::Result<()> {
    let state = State::default();
    let mut canvas = ui::Canvas::init(state, data.0);

    let ui_handle: JoinHandle<io::Result<()>> = tokio::spawn(async move {
        if let Err(e) = canvas.paint().await {
            eprintln!("Canvas render error: {}", e);
            return Err(e); 
        }
        Ok(())
    });

    let ui_result = ui_handle.await;

    ratatui::restore();

    match ui_result {
        Ok(inner_result) => inner_result,
        Err(join_err) => {
            eprintln!("UI task panicked or failed to join: {}", join_err);
            Err(io::Error::new(io::ErrorKind::Other, "UI task failed"))
        }
    }
}
