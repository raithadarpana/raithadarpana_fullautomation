pub mod assets;
pub mod data;
pub mod dictionary;
pub mod ffdeps;
pub mod render;
pub mod scrape;
pub mod storage;
pub mod templates;
pub mod ui;
pub mod video;
pub mod voiceover;


use anyhow::Result;
use chrono::Local;
use clap::Parser;
use std::time::Duration;

use dictionary::{Dictionary, Language};
use render::{ForceFlags, VariantSelection};

/// Maximum age of a cached report JSON before it's considered stale and
/// re-scraped.
const DATA_MAX_AGE: Duration = Duration::from_secs(4 * 60 * 60);

#[derive(Parser, Debug)]
#[command(name = "Raitha Darpana Content Creator")]
#[command(about = "Scrapes market price reports from Karnataka gov site and creates city wise cover images", long_about = None)]
struct Args {
    /// Language to use (kannada or english)
    #[arg(short, long, default_value = "kannada")]
    language: String,

    /// Run without launching the interactive UI (suitable for cron/automation).
    #[arg(long)]
    headless: bool,

    /// Comma-separated city names (English) to render; omit for all cities.
    /// Only used in headless mode.
    #[arg(long)]
    cities: Option<String>,

    /// Date to scrape, in dd/mm/yyyy. Defaults to today. Only used in headless mode.
    #[arg(long)]
    date: Option<String>,

    /// Create the Instagram variant. If neither --ig nor --yt is given, both are created.
    #[arg(long)]
    ig: bool,

    /// Create the YouTube variant. If neither --ig nor --yt is given, both are created.
    #[arg(long)]
    yt: bool,

    /// Skip video creation; only generate cover images.
    #[arg(long = "no-video")]
    no_video: bool,

    /// Force re-fetching data from the web even if a recent cached copy exists.
    #[arg(long = "force-data")]
    force_data: bool,

    /// Force re-creating cover images even if they already exist on disk.
    #[arg(long = "force-image")]
    force_image: bool,

    /// Force re-creating videos even if they already exist on disk. Overrides --no-video.
    #[arg(long = "force-video")]
    force_video: bool,

    /// Force re-fetching data and re-creating images and videos. Overrides --no-video.
    #[arg(long = "force-all")]
    force_all: bool,

    /// TTS voice identity to use (e.g. "kn-IN-GaganNeural"). Defaults to
    /// a sensible per-language voice if omitted.
    #[arg(long)]
    voice: Option<String>,

    /// Speech rate as a signed percentage, e.g. "+20%" or "-10%".
    #[arg(long)]
    rate: Option<String>,

    /// Speech volume as a signed percentage, e.g. "+0%" or "-5%".
    #[arg(long)]
    volume: Option<String>,

    /// Speech pitch as a signed Hz value, e.g. "+0Hz" or "-20Hz".
    #[arg(long)]
    pitch: Option<String>,

    /// Background music volume as a fraction (0.0-1.0). Default: 0.08 (8%).
    #[arg(long = "bg-music-volume")]
    bg_music_volume: Option<f64>,

    /// Silence padding (seconds) added before and after the voiceover
    /// when mixed with background music. Default: 3.
    #[arg(long = "padding-secs")]
    padding_secs: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging()?;

    let args = Args::parse();

    if args.headless {
        ensure_ffmpeg_available_cli().await?;
        run_headless(&args).await
    } else {
        // In UI mode the dependency check/prompt happens in the browser
        // (see ui.rs), since the person may not be watching the terminal.
        ui::run_ui().await
    }
}

/// Checks for ffmpeg/ffprobe before running the headless CLI pipeline.
/// If either is missing, prompts on stdin/stdout for the person to
/// either install it themselves and restart, or approve an automatic
/// download to `rd_media/bin/`.
async fn ensure_ffmpeg_available_cli() -> Result<()> {
    use std::io::Write;

    let status = ffdeps::check_status();
    if status.all_available() {
        return Ok(());
    }

    let missing: Vec<&str> = [
        (!status.ffmpeg_available).then_some("ffmpeg"),
        (!status.ffprobe_available).then_some("ffprobe"),
    ]
    .into_iter()
    .flatten()
    .collect();

    println!(
        "\n⚠️  {} not found on your PATH. Video generation requires it.",
        missing.join(" and ")
    );

    if !ffdeps::download_supported() {
        anyhow::bail!(
            "No pre-built ffmpeg is available for your platform/architecture. \
             Please install ffmpeg and ffprobe manually, add them to your PATH, and restart."
        );
    }

    println!("You can either:");
    println!("  1. Install ffmpeg/ffprobe yourself, add them to PATH, and restart this app.");
    println!("  2. Let this app automatically download them now (to rd_media/bin/).");
    print!("Auto-download ffmpeg and ffprobe now? [y/N]: ");
    std::io::stdout().flush().ok();

    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    let approved = matches!(answer.trim().to_lowercase().as_str(), "y" | "yes");

    if !approved {
        anyhow::bail!(
            "ffmpeg/ffprobe are required to continue. Install them and add to PATH, then restart, \
             or re-run and approve the auto-download."
        );
    }

    ffdeps::download_ffmpeg(|msg| println!("{}", msg))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to download ffmpeg: {}", e))?;

    let status = ffdeps::check_status();
    if !status.all_available() {
        anyhow::bail!("ffmpeg/ffprobe still not available after download attempt.");
    }

    println!("✅ ffmpeg and ffprobe are ready.\n");
    Ok(())
}

/// Logs are written to `rd_media/rd.log` (appended across runs) rather
/// than stderr. This matters especially in UI mode, which takes over
/// the terminal with an alternate screen -- stderr output there would
/// either be invisible or corrupt the display. Set `RUST_LOG` (e.g.
/// `RUST_LOG=info` or `RUST_LOG=debug`) to control verbosity; defaults
/// to `info` if unset.
fn init_logging() -> Result<()> {
    std::fs::create_dir_all(storage::RD_MEDIA_ROOT)?;
    let log_path = std::path::Path::new(storage::RD_MEDIA_ROOT).join("rd.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .format_timestamp_secs()
        .init();

    log::info!("=== raitha_darpana started, logging to {} ===", log_path.display());
    Ok(())
}

async fn run_headless(args: &Args) -> Result<()> {
    let lang = Language::from_str(&args.language).ok_or_else(|| {
        anyhow::anyhow!("Unsupported language: {}. Use 'kannada' or 'english'", args.language)
    })?;

    let dict = Dictionary::load();

    let (date_ddmmyyyy, date_ymd) = match &args.date {
        Some(d) => {
            // Expect dd/mm/yyyy; derive YYYYMMDD for storage.
            let parts: Vec<&str> = d.split('/').collect();
            if parts.len() != 3 {
                anyhow::bail!("Invalid --date format, expected dd/mm/yyyy");
            }
            let ymd = format!("{}{}{}", parts[2], parts[1], parts[0]);
            (d.clone(), ymd)
        }
        None => {
            let today = Local::now();
            (today.format("%d/%m/%Y").to_string(), today.format("%Y%m%d").to_string())
        }
    };

    // Variant selection: if neither --ig nor --yt given, both are produced.
    let variants = match (args.ig, args.yt) {
        (true, false) => VariantSelection::InstagramOnly,
        (false, true) => VariantSelection::YoutubeOnly,
        _ => VariantSelection::Both,
    };

    // force-all implies force-data, force-image, force-video.
    let force_data = args.force_data || args.force_all;
    let force_image = args.force_image || args.force_all;
    let force_video = args.force_video || args.force_all;

    // force-video / force-all override no-video.
    let create_video = if force_video {
        true
    } else {
        !args.no_video
    };

    let force = ForceFlags {
        force_image,
        force_video,
    };

    let needs_refresh = storage::needs_data_refresh(&date_ymd, lang, DATA_MAX_AGE, force_data)?;

    let report = if needs_refresh {
        println!("Fetching and scraping market report for {}...", date_ddmmyyyy);
        let report = scrape::scrape_agriculture_data(&date_ddmmyyyy, lang).await?;
        println!("Scrape complete: {} cities found.", report.cities.len());

        // Data changed: clear all previously generated artifacts (HTML,
        // images, videos, city folders) for this date so nothing stale
        // from the old data lingers alongside the fresh report.
        storage::clear_day_artifacts(&date_ymd, lang)?;

        let json_path = storage::write_report_json(&date_ymd, lang, &report)?;
        println!("Extracted JSON to: {}", json_path.display());
        report
    } else {
        println!("Using cached data for {} ({})...", date_ddmmyyyy, args.language);
        storage::read_report_json(&date_ymd, lang)?
            .ok_or_else(|| anyhow::anyhow!("Expected cached report JSON but none was found"))?
    };

    let cities_filter: Option<Vec<String>> = args.cities.as_ref().map(|s| {
        s.split(',')
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect()
    });
    let filter_slice = cities_filter.as_deref();

    let voice_settings = render::VoiceSettings {
        voice: args.voice.clone(),
        rate: args.rate.clone(),
        volume: args.volume.clone(),
        pitch: args.pitch.clone(),
        bg_music_volume: args.bg_music_volume,
        padding_secs: args.padding_secs,
    };

    let outcome = render::render_report_images(
        &report,
        &date_ymd,
        &dict,
        lang,
        filter_slice,
        variants,
        force,
        create_video,
        &voice_settings,
        |msg| {
            println!("{}", msg);
        },
        |event| {
            println!("Ready: {:?} for {} -> {}", event.kind, event.city, event.path.display());
        },
    )
    .await?;

    if outcome.written.is_empty() && outcome.videos_written.is_empty() {
        println!("No new images or videos were rendered.");
        if !outcome.skipped_cities.is_empty() {
            println!("The following cities were found in the report but did not match your --cities filter:");
            for (scraped, resolved) in &outcome.skipped_cities {
                println!("  - scraped: '{}' -> resolved: '{}'", scraped, resolved);
            }
            println!("Check that --cities values match the resolved English name above (case-insensitive).");
        }
    } else {
        for path in &outcome.written {
            println!("Rendered: {}", path.display());
        }
        for path in &outcome.videos_written {
            println!("Video: {}", path.display());
        }
        if !outcome.skipped_cities.is_empty() {
            println!("Skipped {} cities not matching the filter.", outcome.skipped_cities.len());
        }
    }

    Ok(())
}