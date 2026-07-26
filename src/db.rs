use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

use crate::awdio::metadata::Metadata;
use crate::result::EchoResult;

pub mod repository;

pub async fn init_db(path: &str) -> EchoResult<SqlitePool> {
    let options = SqliteConnectOptions::from_str(path)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

pub async fn insert_song(
    pool: &SqlitePool,
    metadata: &Metadata,
    file_path: &str,
) -> EchoResult<i64> {
    let title = if metadata.title.is_empty() {
        "Unknown Title"
    } else {
        &metadata.title
    };

    let id = sqlx::query!(
        "INSERT INTO songs (title, artist, album, year, genre, track_number, total_tracks, disc_number, total_discs, album_artist, file_path) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        title,
        metadata.artist,
        metadata.album,
        metadata.year,
        metadata.genre,
        metadata.track_number,
        metadata.total_tracks,
        metadata.disc_number,
        metadata.total_discs,
        metadata.album_artist,
        file_path,
    )
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

pub async fn update_song_path(pool: &SqlitePool, song_id: i64, new_path: &str) -> EchoResult<()> {
    sqlx::query!(
        "UPDATE songs SET file_path = ? WHERE id = ?",
        new_path,
        song_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_song_metadata(
    pool: &SqlitePool,
    file_path: &str,
    metadata: &Metadata,
) -> EchoResult<()> {
    let title = if metadata.title.is_empty() {
        "Unknown Title"
    } else {
        &metadata.title
    };
    sqlx::query!(
        "UPDATE songs SET title = ?, artist = ?, album = ?, year = ?, genre = ?, track_number = ?, total_tracks = ?, disc_number = ?, total_discs = ?, album_artist = ? WHERE file_path = ?",
        title,
        metadata.artist,
        metadata.album,
        metadata.year,
        metadata.genre,
        metadata.track_number,
        metadata.total_tracks,
        metadata.disc_number,
        metadata.total_discs,
        metadata.album_artist,
        file_path,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
}

pub async fn get_all_playlists(pool: &SqlitePool) -> EchoResult<Vec<Playlist>> {
    let rows = sqlx::query!("SELECT id, name FROM playlists ORDER BY id")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| Playlist {
            id: r.id,
            name: r.name,
        })
        .collect())
}

pub async fn create_playlist(pool: &SqlitePool, name: &str) -> EchoResult<i64> {
    let id = sqlx::query!("INSERT INTO playlists (name) VALUES (?)", name)
        .execute(pool)
        .await?
        .last_insert_rowid();
    Ok(id)
}

pub async fn delete_playlist(pool: &SqlitePool, playlist_id: i64) -> EchoResult<()> {
    sqlx::query!("DELETE FROM playlists WHERE id = ?", playlist_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn add_song_to_playlist(
    pool: &SqlitePool,
    playlist_id: i64,
    song_path: &str,
) -> EchoResult<()> {
    let row: (Option<i32>,) =
        sqlx::query_as("SELECT MAX(order_index) FROM playlist_songs WHERE playlist_id = ?")
            .bind(playlist_id)
            .fetch_one(pool)
            .await?;

    let next_order = row.0.unwrap_or(0) + 1;

    sqlx::query!(
        "INSERT OR IGNORE INTO playlist_songs (playlist_id, song_path, order_index) VALUES (?, ?, ?)",
        playlist_id,
        song_path,
        next_order,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_song_from_playlist(
    pool: &SqlitePool,
    playlist_id: i64,
    song_path: &str,
) -> EchoResult<()> {
    sqlx::query!(
        "DELETE FROM playlist_songs WHERE playlist_id = ? AND song_path = ?",
        playlist_id,
        song_path,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_playlist_song_paths(
    pool: &SqlitePool,
    playlist_id: i64,
) -> EchoResult<Vec<String>> {
    let rows = sqlx::query_scalar!(
        "SELECT song_path FROM playlist_songs WHERE playlist_id = ? ORDER BY order_index",
        playlist_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
