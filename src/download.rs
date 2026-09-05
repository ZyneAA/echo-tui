use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::result::{EchoReport, EchoResult};

/// Download audio from a YouTube URL as MP3 using yt-dlp.
/// Returns the path to the downloaded file.
pub async fn download_mp3(url: &str, output_dir: &Path) -> EchoResult<PathBuf> {
    let output_template = output_dir.join("%(title)s.%(ext)s");

    let output = Command::new("yt-dlp")
        .args([
            "-x",
            "--audio-format",
            "mp3",
            "--no-playlist",
            "--no-overwrites",
            "--print",
            "after_move:filepath",
            "-o",
            output_template.to_str().unwrap_or("%(title)s.%(ext)s"),
            url,
        ])
        .output()
        .await
        .map_err(|e| EchoReport::DownloadError(format!("Failed to run yt-dlp: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(EchoReport::DownloadError(format!(
            "yt-dlp failed: {}",
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let file_path = stdout.trim().lines().last().unwrap_or("").trim();

    if file_path.is_empty() {
        return Err(EchoReport::DownloadError(
            "yt-dlp did not return a file path".into(),
        ));
    }

    let path = PathBuf::from(file_path);
    if !path.exists() {
        return Err(EchoReport::DownloadError(format!(
            "Downloaded file not found: {}",
            file_path
        )));
    }

    Ok(path)
}

/// Parsed yt-dlp progress line fragment.
#[derive(Debug, Clone, Default)]
pub struct DownloadTick {
    pub percent: f32,
    pub speed: String,
}

/// Download with live progress. `on_tick` is called per progress line.
pub async fn download_mp3_with_progress(
    url: &str,
    output_dir: &Path,
    mut on_tick: impl FnMut(DownloadTick),
) -> EchoResult<PathBuf> {
    use tokio::io::AsyncBufReadExt;

    let output_template = output_dir.join("%(title)s.%(ext)s");

    let mut child = Command::new("yt-dlp")
        .args([
            "-x",
            "--audio-format",
            "mp3",
            "--no-playlist",
            "--no-overwrites",
            "--newline",
            "--progress",
            "--print",
            "after_move:filepath",
            "-o",
            output_template.to_str().unwrap_or("%(title)s.%(ext)s"),
            url,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| EchoReport::DownloadError(format!("Failed to run yt-dlp: {}", e)))?;

    let mut found_path: Option<PathBuf> = None;
    if let Some(stdout) = child.stdout.take() {
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            // lines look like: "[download]  42.3% of    5.12MiB at    1.50MiB/s ETA 00:03"
            if let Some(rest) = line.strip_prefix("[download]") {
                let rest = rest.trim();
                if let Some(pct_str) = rest.split('%').next()
                    && let Ok(percent) = pct_str.trim().parse::<f32>()
                {
                    let speed = rest
                        .split(" at ")
                        .nth(1)
                        .and_then(|s| s.split_whitespace().next())
                        .unwrap_or("")
                        .to_string();
                    on_tick(DownloadTick { percent, speed });
                }
            } else if line.starts_with('/') || line.starts_with("\\\\") {
                // --print after_move:filepath output: absolute path
                let p = PathBuf::from(line.trim());
                if p.exists() {
                    found_path = Some(p);
                }
            }
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| EchoReport::DownloadError(format!("yt-dlp wait failed: {}", e)))?;

    if let Some(p) = found_path {
        return Ok(p);
    }

    if !status.success() {
        return Err(EchoReport::DownloadError(
            "yt-dlp exited with failure".into(),
        ));
    }

    Err(EchoReport::DownloadError(
        "yt-dlp did not return a file path".into(),
    ))
}
