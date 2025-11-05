use std::rc::Rc;
use std::{cell::RefCell, io, time::Duration};

use ratatui::{
    style::{Style, palette::tailwind},
    text::Line,
};
use strum::{Display, EnumIter, FromRepr};

use super::awdio::AudioPlayer;
use super::ui;
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
pub struct State {
    pub exit: bool,
    pub selected_tab: SelectedTab,
    pub input: String,
    pub animations: AnimationState,
    pub uptime: Duration,
}

impl State {
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

pub async fn start(data: (Config, Paths)) -> io::Result<()> {
    let mut state = State::default();
    state.set_animations(
        data.0.animations["animations"].spinner.len(),
        data.0.animations["animations"].hpulse.len(),
        data.0.animations["animations"].dot,
        data.0.animations["animations"].timestamp.clone(),
        data.0.animations["animations"].timestamp_bar.clone(),
    );

    let test_song = data.1.songs.join("test4.mp3");

    let mut audio_player = match AudioPlayer::new(test_song.to_str().unwrap()) {
        Ok(player) => player,
        Err(e) => {
            eprintln!("Failed to create audio player: {}", e);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to create audio player",
            ));
        }
    };

    if let Err(e) = audio_player.play() {
        eprintln!("Failed to start audio player: {}", e);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to start audio player",
        ));
    }

    let audio_state = audio_player.state.clone();

    let mut canvas = ui::EchoCanvas::init(state, data.0, audio_state);

    let ui = canvas.paint().await;

    match ui {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("UI task failed: {}", e);
            Err(io::Error::new(io::ErrorKind::Other, "UI task failed"))
        }
    }
}
