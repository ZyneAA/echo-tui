use crate::{
    awdio::{metadata::Metadata, song::Song},
    result::EchoResult,
};
use sqlx::sqlite::SqlitePool;

#[derive(Debug, Clone)]
pub struct Repository;

impl Repository {
    pub async fn get_songs_from_db(
        pool: &SqlitePool,
        start: usize,
        stop: usize,
    ) -> EchoResult<Vec<Song>> {
        let limit = (stop - start) as i64;
        let offset = start as i64;

        let rows = sqlx::query!(
            "SELECT title, artist, 
            album, year, 
            genre, track_number, 
            total_tracks, disc_number, 
            total_discs, album_artist,
            file_path, has_cover 
            FROM songs ORDER BY id LIMIT ? OFFSET ?",
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let song_list = rows
            .into_iter()
            .map(|row| {
                let metadata = Metadata::new(
                    row.title.clone().unwrap_or_default(),
                    row.artist.clone().unwrap_or_default(),
                    row.album.clone().unwrap_or_default(),
                    row.year.unwrap_or_default() as u32,
                    row.genre.clone().unwrap_or_default(),
                    row.track_number.unwrap_or_default() as u32,
                    row.total_tracks.unwrap_or_default() as u32,
                    row.disc_number.unwrap_or_default() as u32,
                    row.total_discs.unwrap_or_default() as u32,
                    row.album_artist.clone().unwrap_or_default(),
                    if row.has_cover.unwrap_or(false) {
                        Some("internal".into())
                    } else {
                        None
                    },
                );

                let path = row.file_path;
                Song::new_temp(path, metadata)
            })
            .collect::<Vec<Song>>();

        Ok(song_list)
    }
}
