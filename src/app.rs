use std::rc::Rc;
use std::sync::{Arc, mpsc::Sender};
use std::{cell::RefCell, io, time::Duration};

use ratatui::{
    style::{Style, palette::tailwind},
    text::Line,
};
use sqlx::SqlitePool;
use strum::{Display, EnumIter, FromRepr};
use tokio::{
    sync::Mutex,
    time::{self, Interval},
};

use super::awdio::song::Song;
use super::result::EchoResult;
use super::ui;
use crate::awdio::AudioPlayer;
use crate::db::Playlist;
use crate::db::repository::Repository;
use crate::result::EchoReport;
use crate::{config::UiConfig, ignite::Paths};

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

    pub is_blink: bool,
}

#[derive(Debug)]
pub enum EchoSubTab {
    SEARCH,
    METADATA,
    IMPORT,
    DOWNLOAD,
}

#[derive(Debug, Default)]
pub enum DownloadState {
    #[default]
    Idle,
    InputUrl,
    Downloading,
    Done(String),
    Error(String),
}

#[derive(Debug, Default)]
pub enum PlaylistSubTab {
    #[default]
    List,
    Songs,
    InputName,
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

#[derive(Debug)]
pub struct Report {
    pub log: Option<String>,
    pub report: Option<EchoReport>,
    pub level: LogLevel,
}

impl Default for Report {
    fn default() -> Self {
        Report {
            log: None,
            report: None,
            level: LogLevel::INFO,
        }
    }
}

#[derive(Debug)]
pub struct EchoTabState {
    pub is_fft_enable: bool,
    pub prev_sub_state: EchoSubTab,
    pub echo_subtab: EchoSubTab,

    pub echo_metadata_selected_pos: usize,
    pub is_echo_metadata_buffer_being_filled: bool,
    pub metadata_buffer: String,

    pub is_echo_search_buffer_being_filled: bool,
    pub search_buffer: String,
    pub is_confirm_delete: bool,

    pub is_echo_import_buffer_being_filled: bool,
    pub import_buffer: Arc<Mutex<String>>,
    pub import_file_list: Vec<String>,
    pub import_file_selected_pos: usize,
    pub import_selected: Vec<bool>,
    pub is_confirm_import: bool,

    pub is_zero_local_song: bool,
}

impl EchoTabState {
    pub fn new() -> Self {
        Self {
            is_fft_enable: true,
            prev_sub_state: EchoSubTab::SEARCH,
            echo_subtab: EchoSubTab::SEARCH,
            echo_metadata_selected_pos: 0,
            is_echo_metadata_buffer_being_filled: false,
            is_echo_search_buffer_being_filled: false,
            is_confirm_delete: false,
            is_echo_import_buffer_being_filled: false,
            import_file_list: Vec::new(),
            import_file_selected_pos: 0,
            import_selected: Vec::new(),
            is_confirm_import: false,
            is_zero_local_song: true,
            metadata_buffer: "".into(),
            search_buffer: "".into(),
            import_buffer: Arc::new(Mutex::new("".into())),
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub exit: bool,
    pub selected_tab: SelectedTab,
    pub echo_tab_state: EchoTabState,

    pub buffer: String,

    pub animations: AnimationState,

    pub active_track: Song,

    pub uptime: Duration,
    pub uptime_readable: String,
    pub current_clock: String,

    // ticker
    pub ticker: Interval,
    pub amimation_ticker: Interval,
    pub timestamp_ticker: Interval,

    pub selected_song_pos: usize,
    pub local_songs: Vec<Song>,

    // Logging
    pub report_tx: Sender<Report>,
    pub current_report: Option<Report>,

    // Download
    pub download_state: DownloadState,
    pub download_url_buffer: String,

    // Playlist
    pub playlists: Vec<Playlist>,
    pub selected_playlist_idx: usize,
    pub playlist_songs: Vec<Song>,
    pub selected_playlist_song_idx: usize,
    pub playlist_subtab: PlaylistSubTab,
    pub playlist_name_buffer: String,
    pub is_popup: bool,
}

impl State {
    fn new(tx: Sender<Report>) -> Self {
        State {
            exit: false,
            selected_tab: SelectedTab::default(),
            echo_tab_state: EchoTabState::new(),
            buffer: "".into(),
            animations: AnimationState::default(),
            active_track: Song::default(),
            uptime: Duration::default(),
            uptime_readable: "".into(),
            current_clock: "".into(),
            ticker: time::interval(Duration::from_millis(100)),
            amimation_ticker: time::interval(Duration::from_millis(200)),
            timestamp_ticker: time::interval(Duration::from_millis(1000)),
            selected_song_pos: 0,
            local_songs: Vec::new(),
            report_tx: tx,
            current_report: None,
            download_state: DownloadState::default(),
            download_url_buffer: String::new(),
            playlists: Vec::new(),
            selected_playlist_idx: 0,
            playlist_songs: Vec::new(),
            selected_playlist_song_idx: 0,
            playlist_subtab: PlaylistSubTab::default(),
            playlist_name_buffer: String::new(),
            is_popup: false,
        }
    }

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
        self.buffer.push_str(input);
    }

    pub fn reset_input(&mut self) {
        self.buffer.clear();
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }

    pub fn switch_echo_subtab(&mut self, keycode: char) {
        match keycode {
            'M' => self.echo_tab_state.echo_subtab = EchoSubTab::METADATA,
            'S' => self.echo_tab_state.echo_subtab = EchoSubTab::SEARCH,
            'I' => self.echo_tab_state.echo_subtab = EchoSubTab::IMPORT,
            'D' => self.echo_tab_state.echo_subtab = EchoSubTab::DOWNLOAD,
            _ => {}
        }
    }
}

pub async fn start(data: (UiConfig, SqlitePool, Paths)) -> EchoResult<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut state = State::new(tx);

    state.set_animations(
        data.0.animations["animations"].spinner.len(),
        data.0.animations["animations"].hpulse.len(),
        data.0.animations["animations"].dot,
        data.0.animations["animations"].timestamp.clone(),
        data.0.animations["animations"].timestamp_bar.clone(),
    );

    let local_songs = Repository::get_songs_from_db(&data.1, 0, 10).await?;
    if local_songs.len() == 0 {
        state.echo_tab_state.is_zero_local_song = true;
    }
    state.local_songs = local_songs;

    // Load playlists from DB
    if let Ok(pls) = crate::db::get_all_playlists(&data.1).await {
        state.playlists = pls;
    }

    let mut canvas =
        ui::EchoCanvas::init(state, data.0, data.1, None, AudioPlayer::bad(), rx, data.2);

    let ui = canvas.paint().await;

    match ui {
        Ok(()) => Ok(()),
        Err(e) => Err(EchoReport::Io(io::Error::other(e))),
    }
}
