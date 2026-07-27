use crate::data::AgriculturalReport;
use crate::dictionary::{city_folder_name, Dictionary};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;

/// Root directory for all generated media.
pub const RD_MEDIA_ROOT: &str = "rd_media";

/// Returns `rd_media/YYYYMMDD`, creating it if necessary.
pub fn day_dir(date_ymd: &str) -> Result<PathBuf> {
    let dir = Path::new(RD_MEDIA_ROOT).join(date_ymd);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Returns `rd_media/YYYYMMDD/city_name`, creating it if necessary.
/// `city_name` is the canonical English city name (spaces -> underscores).
pub fn city_dir(date_ymd: &str, english_city_name: &str) -> Result<PathBuf> {
    let folder = city_folder_name(english_city_name);
    let dir = day_dir(date_ymd)?.join(folder);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Writes `report_YYYYMMDD.json` under `rd_media/YYYYMMDD/`.
pub fn write_report_json(date_ymd: &str, report: &AgriculturalReport) -> Result<PathBuf> {
    let dir = day_dir(date_ymd)?;
    let path = dir.join(format!("report_{}.json", date_ymd));
    let json = serde_json::to_string_pretty(report)?;
    fs::write(&path, json)?;
    Ok(path)
}

/// Path for the Instagram (4:5) cover image for a given city and date.
pub fn instagram_image_path(date_ymd: &str, english_city_name: &str) -> Result<PathBuf> {
    let dir = city_dir(date_ymd, english_city_name)?;
    let folder = city_folder_name(english_city_name);
    Ok(dir.join(format!("{}-{}-ig.png", folder, date_ymd)))
}

/// Path for the YouTube (16:9) cover image for a given city and date.
pub fn youtube_image_path(date_ymd: &str, english_city_name: &str) -> Result<PathBuf> {
    let dir = city_dir(date_ymd, english_city_name)?;
    let folder = city_folder_name(english_city_name);
    Ok(dir.join(format!("{}-{}-yt.png", folder, date_ymd)))
}

/// Convenience: given a scraped (possibly non-English) city name, resolve
/// the canonical English form via the dictionary before building paths.
pub fn resolve_english_city_name(dict: &Dictionary, scraped_name: &str) -> String {
    dict.city_to_english(scraped_name)
}
