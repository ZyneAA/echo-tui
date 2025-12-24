use core::str;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::{cell::RefCell, io, time::Duration};

use ratatui::{
    style::{Style, palette::tailwind},
    text::Line,
};
use strum::{Display, EnumIter, FromRepr};

use super::awdio::song::Song;
use super::result::EchoResult;
use super::ui;
use crate::awdio::{AudioPlayer, song};
use crate::result::EchoError;
use crate::{config::Config, ignite::Paths};

#[derive(Debug)]
pub struct AnimationTimeStamp {
    pub vals: [String; 50],
}

impl Default for AnimationTimeStamp {
    fn default() -> Self {
        let vals = core::array::from_fn(|_| String::from(""));
        AnimationTimeStamp { vals }
    }
}

#[derive(Debug, Default)]
pub struct AnimationState {
    pub timestamp: (u64, u64),
    pub timestamp_location: usize,

    // animations
    pub animation_timestamp: Rc<RefCell<AnimationTimeStamp>>,
    pub animation_spinner: (usize, usize),
    pub animation_hpulse: (usize, usize),
    pub animation_dot: (usize, usize),
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
        Line::styled(format!(" {} ", self), Style::new().fg(self.palette().c200)).right_aligned()
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

#[derive(Debug, Default)]
pub enum LogLevel {
    #[default]
    INFO,
    ERR,
    WARN,
}

#[derive(Debug, Default)]
pub struct Report {
    pub log: String,
    pub level: LogLevel,
}

#[derive(Debug, Default)]
pub struct State {
    pub exit: bool,
    pub selected_tab: SelectedTab,
    pub input: String,
    pub animations: AnimationState,

    pub uptime: Duration,
    pub uptime_readable: String,
    pub current_clock: String,

    // Local songs
    pub selected_song_pos: usize,
    pub local_songs: Vec<Song>,

    // Logging
    pub report: Arc<Mutex<Report>>,
}

impl State {
    pub fn next_local_song(&mut self) {
        let mut new_index = self.selected_song_pos + 1;
        if new_index > self.local_songs.len() - 1 {
            new_index = 0;
        }

        self.selected_song_pos = new_index;
    }

    pub fn previous_local_song(&mut self) {
        let song_count = self.local_songs.len();

        if self.selected_song_pos == 0 {
            self.selected_song_pos = song_count.saturating_sub(1);
        } else {
            self.selected_song_pos -= 1;
        }
    }

    pub fn set_animations(
        &mut self,
        spinner: usize,
        hpulse: usize,
        dot: usize,
        timestamp: String,
        timestamp_bar: String,
    ) {
        self.animations.animation_spinner.1 = spinner;
        self.animations.animation_hpulse.1 = hpulse;
        self.animations.animation_dot.1 = dot;

        for i in self
            .animations
            .animation_timestamp
            .borrow_mut()
            .vals
            .iter_mut()
        {
            *i = timestamp_bar.clone();
        }
        self.animations.animation_timestamp.borrow_mut().vals[0] = timestamp;
    }

    pub fn append_input(&mut self, input: &str) {
        self.input.push_str(input);
    }

    pub fn reset_input(&mut self) {
        self.input.clear();
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }
}

pub async fn start(data: (Config, Paths)) -> EchoResult<()> {
    let mut state = State::default();

    state.set_animations(
        data.0.animations["animations"].spinner.len(),
        data.0.animations["animations"].hpulse.len(),
        data.0.animations["animations"].dot,
        data.0.animations["animations"].timestamp.clone(),
        data.0.animations["animations"].timestamp_bar.clone(),
    );

    let local_songs = song::get_local_songs(data.1.songs.to_str().unwrap());
    state.local_songs = local_songs;

    let mut canvas = ui::EchoCanvas::init(state, data.0, None, AudioPlayer::bad());

    let ui = canvas.paint().await;

    match ui {
        Ok(()) => Ok(()),
        Err(e) => {
            Err(EchoError::Io(io::Error::other(e)))
        }
    }
}
