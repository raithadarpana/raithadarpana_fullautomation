use crate::data::CommodityEntry;
use crate::dictionary::Language;
use crate::templates::format_price_indian;

use edge_tts_rust::{Boundary, EdgeTtsClient, SpeakOptions};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Default silence padding added before and after the voiceover when
/// mixing in background music, in seconds. Overridable via
/// `VoiceSettings.padding_secs`.
const DEFAULT_BG_MUSIC_BUFFER_SECS: f64 = 3.0;

/// Default background music volume (as a fraction, e.g. 0.08 = 8%) used
/// when mixing with the voiceover. Overridable via
/// `VoiceSettings.bg_music_volume`.
const DEFAULT_BG_MUSIC_VOLUME: f64 = 0.06;

/// User-configurable overrides for Edge TTS voice generation, mirroring
/// `edge_tts_rust::SpeakOptions` fields that are safe to expose to the
/// web UI. `voice` is a speaker identity (e.g. "kn-IN-GaganNeural");
/// `rate`/`volume` are signed percentages ("+10%", "-5%"); `pitch` is a
/// signed Hz value ("+0Hz"). `bg_music_volume` is a fraction (0.0-1.0,
/// default 0.06 = 6%) applied to the background music track when mixed
/// with the voiceover; `padding_secs` is the silence padding (seconds,
/// default 3.0) added before and after the voiceover in the mixed track.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceSettings {
    pub voice: Option<String>,
    pub rate: Option<String>,
    pub volume: Option<String>,
    pub pitch: Option<String>,
    pub bg_music_volume: Option<f64>,
    pub padding_secs: Option<f64>,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            voice: None,
            rate: None,
            volume: None,
            pitch: None,
            bg_music_volume: None,
            padding_secs: None,
        }
    }
}

/// Named speaker choices surfaced in the UI, per language/gender.
pub fn voice_choices(lang: Language) -> &'static [(&'static str, &'static str)] {
    match lang {
        Language::Kannada => &[
            ("kn-IN-GaganNeural", "Gagan (Male)"),
            ("kn-IN-SapnaNeural", "Sapna (Female)"),
        ],
        Language::English => &[
            ("en-IN-PrabhatNeural", "Prabhat (Male)"),
            ("en-IN-NeerjaNeural", "Neerja (Female)"),
        ],
    }
}

pub fn sanitize_filename(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    sanitized.trim_matches('_').to_string()
}

pub fn find_background_music(folder_path: &Path) -> Option<PathBuf> {
    let candidates = [
        "bg_music.mp3",
        "background.mp3",
        "background_music.mp3",
        "music.mp3",
    ];

    for candidate in candidates {
        let path = folder_path.join(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    for entry in WalkDir::new(folder_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("mp3") {
                let name = path.file_name().unwrap().to_string_lossy().to_lowercase();
                if name.contains("bg") || name.contains("background") || name.contains("music") {
                    return Some(path.to_path_buf());
                }
            }
        }
    }

    None
}

fn convert_integer_to_kannada(mut num: i64) -> String {
    if num == 0 {
        return "ಶೂನ್ಯ".to_string();
    }

    let units = ["", "ಒಂದು", "ಎರಡು", "ಮೂರು", "ನಾಲ್ಕು",
                 "ಐದು", "ಆರು", "ಏಳು", "ಎಂಟು", "ಒಂಬತ್ತು"];
    
    let tens = ["", "ಹತ್ತು", "ಇಪ್ಪತ್ತು", "ಮೂವತ್ತು", "ನಲವತ್ತು",
                "ಐವತ್ತು", "ಅರವತ್ತು", "ಎಪ್ಪತ್ತು", "ಎಂಭತ್ತು", "ತೊಂಬತ್ತು"];
    
    let teens = ["ಹತ್ತು", "ಹನ್ನೊಂದು", "ಹನ್ನೆರಡು", "ಹದಿಮೂರು", "ಹದಿನಾಲ್ಕು",
                 "ಹದಿನೈದು", "ಹದಿನಾರು", "ಹದಿನೇಳು", "ಹದಿನೆಂಟು", "ಹತ್ತೊಂಬತ್ತು"];
    let hundreds = ["",
    "ನೂರು",
    "ಇನ್ನೂರು",
    "ಮುನ್ನೂರು",
    "ನಾನೂರು",
    "ಐನೂರು",
    "ಆರುನೂರು",
    "ಏಳುನೂರು",
    "ಎಂಟುನೂರು",
    "ಒಂಬೈನೂರು",
];
    
    let scales = ["", "ಸಾವಿರ", "ಲಕ್ಷ", "ಕೋಟಿ", "ಅರ್ಬುದ"];

    let mut result = String::new();
    let mut scale_idx = 0;

    while num > 0 && scale_idx < scales.len() {
        let chunk = num % 1000;
        num /= 1000;

        if chunk > 0 {
            let mut chunk_str = convert_hundreds(chunk as i64, &units, &tens, &teens);
            
            // Add scale BEFORE the chunk, not after
            if scale_idx > 0 {
                chunk_str.push(' ');
                chunk_str.push_str(scales[scale_idx]);
            }
            
            if !result.is_empty() {
                chunk_str.push(' ');
                chunk_str.push_str(&result);
            }
            result = chunk_str;
        }

        scale_idx += 1;
    }

    result
}

fn convert_hundreds(num: i64, units: &[&str], tens: &[&str], teens: &[&str]) -> String {
    let mut result = String::new();

    let hundreds = num / 100;
    if hundreds > 0 {
        result.push_str(units[hundreds as usize]);
        result.push_str(" ನೂರು");
    }

    let remainder = num % 100;
    if remainder > 0 {
        if !result.is_empty() {
            result.push(' ');
        }

        if remainder < 10 {
            result.push_str(units[remainder as usize]);
        } else if remainder < 20 {
            result.push_str(teens[remainder as usize - 10]);
        } else {
            let ten = remainder / 10;
            let unit = remainder % 10;

            result.push_str(tens[ten as usize]);
            if unit > 0 {
                result.push(' ');
                result.push_str(units[unit as usize]);
            }
        }
    }

    result
}

fn convert_single_digit(digit: i64) -> String {
    let digits = ["ಶೂನ್ಯ", "ಒಂದು", "ಎರಡು", "ಮೂರು", "ನಾಲ್ಕು",
                  "ಐದು", "ಆರು", "ಏಳು", "ಎಂಟು", "ಒಂಬತ್ತು"];
    digits[digit as usize].to_string()
}

fn number_to_kannada(value: f64) -> String {
    if value.is_nan() {
        return "ಸಂಖ್ಯೆ ಅಲ್ಲ".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            "ಅನಂತ".to_string()
        } else {
            "ಋಣ ಅನಂತ".to_string()
        };
    }

    let is_negative = value < 0.0;
    let abs_value = value.abs();

    let integer_part = abs_value.trunc() as i64;
    let decimal_part = abs_value.fract();

    let mut result = String::new();

    if is_negative {
        result.push_str("ಋಣ ");
    }

    result.push_str(&convert_integer_to_kannada(integer_part));

    if decimal_part > 1e-10 {
        result.push_str(" ದಶಮಾಂಶ");

        let decimal_str = format!("{:.15}", decimal_part);
        if let Some(dot_pos) = decimal_str.find('.') {
            let digits = &decimal_str[dot_pos + 1..];
            for digit_char in digits.chars() {
                if digit_char.is_ascii_digit() {
                    let digit = digit_char.to_digit(10).unwrap() as i64;
                    result.push(' ');
                    result.push_str(&convert_single_digit(digit));
                }
            }
        }
    }

    result.trim().to_string()
}


/// A narration script split into its three parts: the fixed intro (read
/// while no commodity rows are shown yet), one sentence per commodity
/// (in the same order as the table rows, so `items[i]` narrates the same
/// row that `templates::top_commodities` puts at position `i`), and the
/// fixed outro (read once every row is visible).
///
/// This is what lets the video pipeline estimate, per row, roughly when
/// during the audio that row's sentence is spoken -- see
/// `render::generate_city_video_assets` / `row_reveal_timings`. There's
/// no per-word timestamp from the TTS engine here, so timing is
/// approximated by each part's share of total word count; that's close
/// enough to look "animated in sync" without needing exact timestamps.
pub struct ScriptSegments {
    pub intro: String,
    pub items: Vec<String>,
    pub outro: String,
}

impl ScriptSegments {
    /// Concatenates the parts back into the single narration string that
    /// gets sent to the TTS engine (identical to the old `generate_script`
    /// output).
    pub fn joined(&self) -> String {
        let mut script = self.intro.clone();
        for item in &self.items {
            script.push_str(item);
        }
        script.push_str(&self.outro);
        script
    }
}

fn word_count(s: &str) -> usize {
    s.split_whitespace().count().max(1)
}

pub fn generate_script_segments(
    lang: Language,
    market: &str,
    date: &str,
    items: &[CommodityEntry],
) -> ScriptSegments {
    match lang {
        Language::Kannada => kannada_script_segments(market, date, items),
        Language::English => english_script_segments(market, date, items),
    }
}

pub fn kannada_script_segments(market: &str, date: &str, items: &[CommodityEntry]) -> ScriptSegments {
    let intro = format!(
        "ನಮಸ್ಕಾರ ವೀಕ್ಷಕರೇ, ರೈತ ದರ್ಪಣ ಮಾರುಕಟ್ಟೆ ಬೆಲೆಗಳ ಮಾಹಿತಿ ಚಾನೆಲ್‌ಗೆ ಸುಸ್ವಾಗತ. ಇಂದಿನ {} ಮಾರುಕಟ್ಟೆಯ ತರಕಾರಿ ಬೆಲೆಗಳ ಮಾಹಿತಿ ಇಲ್ಲಿದೆ. ವರದಿ ದಿನಾಂಕ: {}. ",
        market, date
    );

  let item_sentences: Vec<String> = items
    .iter()
    .map(|item| {
        let commodity_variety = if item.commodity.trim().eq_ignore_ascii_case(item.variety.trim()) {
            format!("{}", item.commodity)
        } else {
            format!("{}, ತಳಿ {}", item.commodity, item.variety)
        };

        format!(
            "{}: ಕನಿಷ್ಠ ಬೆಲೆ {} ರೂಪಾಯಿ, ಗರಿಷ್ಠ ಬೆಲೆ {} ರೂಪಾಯಿ. ",
            commodity_variety,
            number_to_kannada(item.min_rs),
            number_to_kannada(item.max_rs)
        )
    })
    .collect();

    let outro = "ದಿನನಿತ್ಯದ ನಿಖರ ಮಾಹಿತಿಗಾಗಿ ನಮ್ಮ ಇನ್ಸ್ಟಾಗ್ರಾಮ್ ಮತ್ತು ಯೂಟ್ಯೂಬ್ ಚಾನೆಲ್‌ಗೆ ಸಬ್‌ಸ್ಕ್ರೈಬ್ ಆಗಿ. ಧನ್ಯವಾದಗಳು!".to_string();

    let segments = ScriptSegments {
        intro,
        items: item_sentences,
        outro,
    };
    log::info!("{}", segments.joined());
    segments
}

pub fn english_script_segments(market: &str, date: &str, items: &[CommodityEntry]) -> ScriptSegments {
    let intro = format!(
        "Hello viewers, welcome to Raitha Darpana market price update. Here is today's {} market commodity price report. Report date: {}. ",
        market, date
    );

    let item_sentences: Vec<String> = items
        .iter()
        .map(|item| {
            format!(
                "{}: minimum price {} rupees, maximum price {} rupees. ",
                item.commodity,
                format_price_indian(item.min_rs),
                format_price_indian(item.max_rs)
            )
        })
        .collect();

    let outro = "Subscribe to our Instagram and YouTube channels for daily accurate updates. Thank you!".to_string();

    ScriptSegments {
        intro,
        items: item_sentences,
        outro,
    }
}

/// Cumulative time offsets (seconds, from the start of `audio_path`) at
/// which each additional row should become visible, followed by the
/// total audio duration. `offsets[0]` is always `0.0` (no rows visible
/// yet, intro playing); `offsets[i]` (for `1..=items.len()`) is when row
/// `i` should appear; the final element is the full audio length (used
/// as the end time for the "all rows visible" frame). Length is always
/// `items.len() + 2`.
pub fn row_reveal_offsets(segments: &ScriptSegments, total_audio_secs: f64) -> Vec<f64> {
    let intro_w = word_count(&segments.intro);
    let item_w: Vec<usize> = segments.items.iter().map(|s| word_count(s)).collect();
    let outro_w = word_count(&segments.outro);
    let total_w: usize = intro_w + item_w.iter().sum::<usize>() + outro_w;
    let total_w = total_w.max(1) as f64;

    let mut offsets = Vec::with_capacity(segments.items.len() + 2);
    let mut cumulative_w = intro_w as f64;
    offsets.push(0.0);
    for w in &item_w {
        offsets.push((cumulative_w / total_w) * total_audio_secs);
        cumulative_w += *w as f64;
    }
    offsets.push(total_audio_secs);
    offsets
}

pub fn generate_kannada_script(market: &str, date: &str, items: &[CommodityEntry]) -> String {
    kannada_script_segments(market, date, items).joined()
}

pub fn generate_english_script(market: &str, date: &str, items: &[CommodityEntry]) -> String {
    english_script_segments(market, date, items).joined()
}

/// Generates the narration script for a city's commodities in the given
/// language.
pub fn generate_script(lang: Language, market: &str, date: &str, items: &[CommodityEntry]) -> String {
    match lang {
        Language::Kannada => generate_kannada_script(market, date, items),
        Language::English => generate_english_script(market, date, items),
    }
}

/// Returns a sensible default Edge TTS voice identity for the given
/// language, used unless overridden by the `VOICE_ID` env var or an
/// explicit `VoiceSettings.voice`.
pub fn default_voice_id(lang: Language) -> &'static str {
    match lang {
        Language::Kannada => "kn-IN-GaganNeural",
        Language::English => "en-IN-PrabhatNeural",
    }
}

pub async fn generate_audio_file(
    text: &str,
    lang: Language,
    primary_output_path: &Path,
    mixed_output_path: &Path,
    music_search_dir: &Path,
    voice_settings: &VoiceSettings,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    generate_edge_tts(text, lang, primary_output_path, voice_settings).await?;

    if let Some(bg_path) = find_background_music(music_search_dir) {
        let bg_volume = voice_settings.bg_music_volume.unwrap_or(DEFAULT_BG_MUSIC_VOLUME);
        log::info!("{}", format!(
            "🎧 Found background music at {}. Mixing at {:.0}% volume.",
            bg_path.display(),
            bg_volume * 100.0
        ));
        mix_audio_with_bg(primary_output_path, &bg_path, mixed_output_path, voice_settings)?;
        Ok(mixed_output_path.to_path_buf())
    } else {
        println!("ℹ️ No background music file found. Using voice audio only.");
        Ok(primary_output_path.to_path_buf())
    }
}

async fn generate_edge_tts(
    text: &str,
    lang: Language,
    output_path: &Path,
    voice_settings: &VoiceSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = EdgeTtsClient::new()?;
    let voice_identity = voice_settings
        .voice
        .clone()
        .or_else(|| std::env::var("VOICE_ID").ok())
        .unwrap_or_else(|| default_voice_id(lang).to_string());

    let rate = voice_settings.rate.clone().unwrap_or_else(|| "+10%".to_string());
    let volume = voice_settings.volume.clone().unwrap_or_else(|| "+0%".to_string());
    let pitch = voice_settings.pitch.clone().unwrap_or_else(|| "+0Hz".to_string());

    client
        .save(
            text,
            SpeakOptions {
                voice: voice_identity.into(),
                rate,
                volume,
                pitch,
                boundary: Boundary::Sentence,
                ..SpeakOptions::default()
            },
            output_path,
            None::<&Path>,
        )
        .await?;

    Ok(())
}

/// Public wrapper around `probe_duration_secs`, for callers (the video
/// pipeline) that need to know an already-generated audio file's length
/// -- e.g. to time the row-reveal animation against it.
pub fn audio_duration_secs(path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    probe_duration_secs(path)
}

/// Returns the duration of a media file in seconds, via `ffprobe`.
fn probe_duration_secs(path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    let ffprobe_path = crate::ffdeps::ffprobe_path();
    let output = Command::new(ffprobe_path)
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "ffprobe failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let secs: f64 = stdout.trim().parse().map_err(|e| {
        format!(
            "Could not parse ffprobe duration output '{}' for {}: {}",
            stdout.trim(),
            path.display(),
            e
        )
    })?;
    Ok(secs)
}

/// Mixes the voiceover with background music, so the final clip is
/// exactly `3s (lead-in silence) + voiceover length + 3s (trailing
/// silence)` long. The background music is looped continuously if it's
/// shorter than that total duration, then trimmed to fit exactly.
fn mix_audio_with_bg(
    voice_path: &Path,
    bg_path: &Path,
    output_path: &Path,
    voice_settings: &VoiceSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let ffmpeg_path = crate::ffdeps::ffmpeg_path();

    let padding_secs = voice_settings.padding_secs.unwrap_or(DEFAULT_BG_MUSIC_BUFFER_SECS);
    let bg_volume = voice_settings.bg_music_volume.unwrap_or(DEFAULT_BG_MUSIC_VOLUME);

    let voice_secs = probe_duration_secs(voice_path)?;
    let total_secs = padding_secs + voice_secs + padding_secs;

    // [0:a] voice delayed by the lead-in buffer (in ms), then padded with
    // silence at the tail so it reaches the full target length.
    // [1:a] background music looped indefinitely, then trimmed to the
    // target length -- `-stream_loop -1` on the input handles the
    // "loop continuously until long enough" requirement regardless of
    // how short the source music file is.
    let delay_ms = (padding_secs * 1000.0).round() as i64;
    let filter = format!(
        "[0:a]adelay={delay}|{delay},apad=whole_dur={total}[voice];\
         [1:a]atrim=0:{total},asetpts=PTS-STARTPTS,volume={bg_volume}[bg];\
         [voice][bg]amix=inputs=2:dropout_transition=0:normalize=0,\
         atrim=0:{total},asetpts=PTS-STARTPTS[aout]",
        delay = delay_ms,
        total = total_secs,
        bg_volume = bg_volume,
    );

    let output = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-i")
        .arg(voice_path)
        .arg("-stream_loop")
        .arg("-1")
        .arg("-i")
        .arg(bg_path)
        .arg("-filter_complex")
        .arg(&filter)
        .arg("-map")
        .arg("[aout]")
        .arg("-t")
        .arg(total_secs.to_string())
        .arg("-c:a")
        .arg("libmp3lame")
        .arg("-q:a")
        .arg("2")
        .arg(output_path)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "ffmpeg audio mix failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}