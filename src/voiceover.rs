
use crate::data::CommodityEntry;

use edge_tts_rust::{Boundary, EdgeTtsClient, SpeakOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

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

fn find_background_music(folder_path: &Path) -> Option<PathBuf> {
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

pub fn generate_kannada_script(market: &str, date: &str, items: &[CommodityEntry]) -> String {
    let mut script = format!(
        "ನಮಸ್ಕಾರ ವೀಕ್ಷಕರೇ, ರೈತ ದರ್ಪಣ ಮಾರುಕಟ್ಟೆ ಬೆಲೆಗಳ ಮಾಹಿತಿ ಚಾನೆಲ್‌ಗೆ ಸುಸ್ವಾಗತ. ಇಂದಿನ {} ಮಾರುಕಟ್ಟೆಯ ತರಕಾರಿ ಬೆಲೆಗಳ ಮಾಹಿತಿ ಇಲ್ಲಿದೆ. ವರದಿ ದಿನಾಂಕ: {}. ",
        market, date
    );

    for item in items {
        script.push_str(&format!(
            "{}: ಕನಿಷ್ಠ ಬೆಲೆ {} ರೂಪಾಯಿ, ಗರಿಷ್ಠ ಬೆಲೆ {} ರೂಪಾಯಿ. ",
            item.commodity, item.min_rs, item.max_rs
        ));
    }

    script.push_str("ದಿನನಿತ್ಯದ ನಿಖರ ಮಾಹಿತಿಗಾಗಿ ನಮ್ಮ ಇನ್ಸ್ಟಾಗ್ರಾಮ್ ಮತ್ತು ಯೂಟ್ಯೂಬ್ ಚಾನೆಲ್‌ಗೆ ಸಬ್‌ಸ್ಕ್ರೈಬ್ ಆಗಿ. ಧನ್ಯವಾದಗಳು!");
    script
}

pub async fn generate_audio_file(
    text: &str,
    primary_output_path: &Path,
    mixed_output_path: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    generate_edge_tts(text, primary_output_path).await?;

    if let Some(bg_path) = find_background_music(primary_output_path.parent().unwrap_or_else(|| Path::new("."))) {
        println!(
            "🎧 Found background music at {}. Mixing at 8% volume.",
            bg_path.display()
        );
        mix_audio_with_bg(primary_output_path, &bg_path, mixed_output_path)?;
        // attempt to remove the temporary primary voice file to avoid duplicates
        if let Err(e) = fs::remove_file(primary_output_path) {
            eprintln!("⚠️ Warning: failed to remove temporary voice file {}: {}", primary_output_path.display(), e);
        }
        Ok(mixed_output_path.to_path_buf())
    } else {
        println!("ℹ️ No background music file found. Using voice audio only.");
        Ok(primary_output_path.to_path_buf())
    }
}

async fn generate_edge_tts(text: &str, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let client = EdgeTtsClient::new()?;
    let voice_identity = std::env::var("VOICE_ID").unwrap_or_else(|_| "kn-IN-GaganNeural".into());

    client
        .save(
            text,
            SpeakOptions {
                voice: voice_identity.into(),
                rate: "+20%".to_string(),
                boundary: Boundary::Sentence,
                ..SpeakOptions::default()
            },
            output_path,
            None::<&Path>,
        )
        .await?;

    Ok(())
}

fn mix_audio_with_bg(
    voice_path: &Path,
    bg_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let ffmpeg_path = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let filter = "[1:a]volume=0.12[a1];[0:a][a1]amix=inputs=2:dropout_transition=0:normalize=0[aout]";
    let output = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-i")
        .arg(voice_path)
        .arg("-i")
        .arg(bg_path)
        .arg("-filter_complex")
        .arg(filter)
        .arg("-map")
        .arg("[aout]")
        .arg("-c:a")
        .arg("libmp3lame")
        .arg("-q:a")
        .arg("2")
        .arg("-shortest")
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