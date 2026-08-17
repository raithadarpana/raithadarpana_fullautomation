//! Tracks what has already been published for a given (date, market),
//! so re-running the CLI for the same day doesn't silently re-upload
//! the same Reel/video.
//!
//! Stored alongside the other per-city artifacts as
//! `rd_media/YYYYMMDD/<city_folder>/publish_status.json`, reusing the
//! project's existing `storage::city_dir` convention rather than
//! introducing a separate database.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublishStatus {
    pub date_ymd: String,
    pub english_city_name: String,
    pub instagram_media_id: Option<String>,
    pub youtube_video_id: Option<String>,
}

fn status_path(date_ymd: &str, english_city_name: &str) -> Result<PathBuf> {
    Ok(crate::storage::city_dir(date_ymd, english_city_name)?.join("publish_status.json"))
}

pub fn load(date_ymd: &str, english_city_name: &str) -> Result<PublishStatus> {
    let path = status_path(date_ymd, english_city_name)?;
    if !path.exists() {
        return Ok(PublishStatus {
            date_ymd: date_ymd.to_string(),
            english_city_name: english_city_name.to_string(),
            ..Default::default()
        });
    }
    let contents = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&contents).unwrap_or(PublishStatus {
        date_ymd: date_ymd.to_string(),
        english_city_name: english_city_name.to_string(),
        ..Default::default()
    }))
}

pub fn save(status: &PublishStatus) -> Result<()> {
    let path = status_path(&status.date_ymd, &status.english_city_name)?;
    let json = serde_json::to_string_pretty(status)?;
    std::fs::write(path, json)?;
    Ok(())
}