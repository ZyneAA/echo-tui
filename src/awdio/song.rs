use std::{fs, path::Path};

use crate::awdio::metadata;

#[derive(Debug, Default)]
pub struct Song {
    pub metadata: metadata::Metadata,
    pub path: String,
}

impl Song {
    pub fn new(path: String) -> Self {
        Song {
            metadata: metadata::Metadata::from_path(&path).unwrap(),
            path,
        }
    }

    pub fn ref_array(&self) -> [&Option<String>; 3] {
        [
            &self.metadata.title,
            &self.metadata.artist,
            &self.metadata.album,
        ]
    }
}

pub fn get_local_songs(url: &str) -> Vec<Song> {
    let path = Path::new(url);
    if !path.is_dir() {
        eprintln!("Error: Path is not a directory or does not exist: {}", url);
        return Vec::new();
    }

    let mut songs = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(extension) = file_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                {
                    if ["mp3", "flac", "wav", "ogg", "m4a", "aiff"].contains(&extension.as_str()) {
                        let song_path = file_path.to_string_lossy().to_string();
                        let new_song = Song::new(song_path);

                        songs.push(new_song);
                    }
                }
            }
        }
    } else {
        eprintln!("Error reading directory: {}", url);
    }

    songs
}
