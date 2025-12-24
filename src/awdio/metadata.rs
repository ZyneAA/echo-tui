use audiotags::{MimeType, Tag};

#[derive(Debug)]
pub struct Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,

    pub track_number: Option<u32>,
    pub total_tracks: Option<u32>,

    pub disc_number: Option<u32>,
    pub total_discs: Option<u32>,

    pub year: Option<u32>,
    pub genre: Option<String>,
    pub cover: Option<AlbumArt>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            title: Some("Unknown".into()),
            artist: Some("Unknown".into()),
            album: Some("Unknown".into()),
            album_artist: None,
            track_number: Some(1),
            total_tracks: None,
            disc_number: Some(1),
            total_discs: None,
            year: Some(0),
            genre: Some("".into()),
            cover: None,
        }
    }
}

#[derive(Debug)]
pub struct AlbumArt {
    pub data: Vec<u8>,
    pub mime: MimeType,
}

impl Metadata {
    pub fn from_path(path: &str) -> Result<Metadata, audiotags::Error> {
        let tag = Tag::new().read_from_path(path)?;

        let metadata = Metadata {
            title: tag.title().map(String::from),
            artist: tag.artist().map(String::from),
            album: tag.album_title().map(String::from),
            year: tag.year().map(|y| y as u32),
            genre: tag.genre().map(String::from),
            track_number: tag.track_number().map(|n| n as u32),
            total_tracks: tag.total_tracks().map(|n| n as u32),
            cover: tag.album_cover().map(|pic| AlbumArt {
                data: pic.data.into(),
                mime: pic.mime_type,
            }),
            album_artist: tag.album_artist().map(String::from),
            disc_number: tag.disc_number().map(|n| n as u32),
            total_discs: tag.total_discs().map(|n| n as u32),
        };

        Ok(metadata)
    }
}
