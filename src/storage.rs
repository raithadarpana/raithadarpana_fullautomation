use crate::data::AgriculturalReport;
use crate::dictionary::{city_folder_name, Dictionary, Language};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{Duration, SystemTime};

/// Root directory for all generated media.
pub const RD_MEDIA_ROOT: &str = "rd_media";

/// Short-form language suffix used in all file/folder names so English
/// and Kannada artifacts for the same city/date don't collide.
pub fn lang_short(lang: Language) -> &'static str {
    match lang {
        Language::English => "eng",
        Language::Kannada => "kan",
    }
}

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

/// Path for the report JSON, with a short-form language suffix so English
/// and Kannada reports for the same date don't overwrite each other.
/// e.g. `rd_media/20260801/report_20260801-eng.json`
pub fn report_json_path(date_ymd: &str, lang: Language) -> Result<PathBuf> {
    let dir = day_dir(date_ymd)?;
    Ok(dir.join(format!("report_{}-{}.json", date_ymd, lang_short(lang))))
}

/// Writes the report JSON under `rd_media/YYYYMMDD/`, suffixed with the
/// short-form language code.
pub fn write_report_json(date_ymd: &str, lang: Language, report: &AgriculturalReport) -> Result<PathBuf> {
    let path = report_json_path(date_ymd, lang)?;
    let json = serde_json::to_string_pretty(report)?;
    fs::write(&path, json)?;
    Ok(path)
}

/// Reads a previously written report JSON, if present, for the given
/// date/language. Returns `None` if the file doesn't exist.
pub fn read_report_json(date_ymd: &str, lang: Language) -> Result<Option<AgriculturalReport>> {
    let path = report_json_path(date_ymd, lang)?;
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)?;
    let report: AgriculturalReport = serde_json::from_str(&contents)?;
    Ok(Some(report))
}

/// Returns `Some(age)` if the report JSON for this date/language exists,
/// where `age` is how long ago it was last modified. Returns `None` if
/// the file does not exist.
pub fn report_json_age(date_ymd: &str, lang: Language) -> Result<Option<Duration>> {
    let path = report_json_path(date_ymd, lang)?;
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::metadata(&path)?;
    let modified = metadata.modified()?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::from_secs(0));
    Ok(Some(age))
}

/// Decides whether the data for `date_ymd`/`lang` needs to be (re-)scraped:
/// true if the JSON file is missing, older than `max_age`, or the caller
/// explicitly forces a refresh.
pub fn needs_data_refresh(date_ymd: &str, lang: Language, max_age: Duration, force: bool) -> Result<bool> {
    if force {
        return Ok(true);
    }
    match report_json_age(date_ymd, lang)? {
        None => Ok(true),
        Some(age) => Ok(age > max_age),
    }
}

/// Removes previously generated artifacts (HTML, images, videos, audio)
/// for a given date, scoped to a single language, then prunes any
/// subfolders left empty as a result. The day directory itself is kept.
///
/// Scoping to `lang` means refreshing data for one language never wipes
/// out artifacts already generated for the other language on the same
/// date. A file is considered to belong to `lang` if its name contains
/// the marker `-{lang_short}.` immediately before the extension (e.g.
/// `bengaluru-20260801-ig-eng.png`, `bengaluru-20260801-voice-eng.mp3`).
pub fn clear_day_artifacts(date_ymd: &str, lang: Language) -> Result<()> {
    let dir = day_dir(date_ymd)?;
    let marker = format!("-{}.", lang_short(lang));

    clear_lang_files_recursive(&dir, &marker)?;
    prune_empty_dirs(&dir)?;

    Ok(())
}

/// Recursively removes files under `dir` whose name contains `marker`
/// (this also covers the language-specific report JSON, since its name
/// is `report_YYYYMMDD-{lang}.json`).
fn clear_lang_files_recursive(dir: &Path, marker: &str) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            clear_lang_files_recursive(&path, marker)?;
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.contains(marker) {
                fs::remove_file(&path)?;
            }
        }
    }
    Ok(())
}

/// Recursively removes any subdirectories under `dir` that are left
/// empty (bottom-up), keeping `dir` itself even if it ends up empty.
fn prune_empty_dirs(dir: &Path) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            prune_empty_dirs(&path)?;
            let is_empty = fs::read_dir(&path)?.next().is_none();
            if is_empty {
                fs::remove_dir(&path)?;
            }
        }
    }
    Ok(())
}

/// Path for the Instagram (4:5) cover image for a given city, date, language.
pub fn instagram_image_path(date_ymd: &str, english_city_name: &str, lang: Language) -> Result<PathBuf> {
    let dir = city_dir(date_ymd, english_city_name)?;
    let folder = city_folder_name(english_city_name);
    Ok(dir.join(format!("{}-{}-ig-{}.png", folder, date_ymd, lang_short(lang))))
}

/// Path for the YouTube (16:9) cover image for a given city, date, language.
pub fn youtube_image_path(date_ymd: &str, english_city_name: &str, lang: Language) -> Result<PathBuf> {
    let dir = city_dir(date_ymd, english_city_name)?;
    let folder = city_folder_name(english_city_name);
    Ok(dir.join(format!("{}-{}-yt-{}.png", folder, date_ymd, lang_short(lang))))
}

/// Path for the generated voiceover (raw TTS, before background music mix).
pub fn voice_audio_path(date_ymd: &str, english_city_name: &str, lang: Language) -> Result<PathBuf> {
    let dir = city_dir(date_ymd, english_city_name)?;
    let folder = city_folder_name(english_city_name);
    Ok(dir.join(format!("{}-{}-voice-{}.mp3", folder, date_ymd, lang_short(lang))))
}

/// Path for the final mixed audio (voice + background music).
pub fn mixed_audio_path(date_ymd: &str, english_city_name: &str, lang: Language) -> Result<PathBuf> {
    let dir = city_dir(date_ymd, english_city_name)?;
    let folder = city_folder_name(english_city_name);
    Ok(dir.join(format!("{}-{}-audio-{}.mp3", folder, date_ymd, lang_short(lang))))
}

/// Path for the Instagram (portrait) video.
pub fn instagram_video_path(date_ymd: &str, english_city_name: &str, lang: Language) -> Result<PathBuf> {
    let dir = city_dir(date_ymd, english_city_name)?;
    let folder = city_folder_name(english_city_name);
    Ok(dir.join(format!("{}-{}-ig-{}.mp4", folder, date_ymd, lang_short(lang))))
}

/// Path for the YouTube (landscape) video.
pub fn youtube_video_path(date_ymd: &str, english_city_name: &str, lang: Language) -> Result<PathBuf> {
    let dir = city_dir(date_ymd, english_city_name)?;
    let folder = city_folder_name(english_city_name);
    Ok(dir.join(format!("{}-{}-yt-{}.mp4", folder, date_ymd, lang_short(lang))))
}

/// Convenience: given a scraped (possibly non-English) city name, resolve
/// the canonical English form via the dictionary before building paths.
pub fn resolve_english_city_name(dict: &Dictionary, scraped_name: &str) -> String {
    dict.city_to_english(scraped_name)
}