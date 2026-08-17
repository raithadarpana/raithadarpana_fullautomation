pub mod assets;
pub mod config;
pub mod data;
pub mod dictionary;
pub mod ffdeps;
pub mod render;
pub mod scrape;
#[path = "Social/mod.rs"]
pub mod Social;
pub mod social;
pub mod storage;
pub mod templates;
pub mod ui;
pub mod video;
pub mod voiceover;


use anyhow::Result;
use chrono::Local;
use clap::Parser;
use rustls::crypto::{CryptoProvider, ring::default_provider};
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

    /// After generating each city's 9:16 video, publish it to
    /// Instagram as a Reel. Requires INSTAGRAM_ACCESS_TOKEN and
    /// INSTAGRAM_USER_ID (env or .env). Implies the Instagram video
    /// variant is generated even if --yt-only was otherwise implied.
    #[arg(long = "upload-instagram")]
    upload_instagram: bool,

    /// After generating each city's landscape video, publish it to
    /// YouTube. Requires YOUTUBE_CLIENT_ID and YOUTUBE_CLIENT_SECRET
    /// (env or .env); YOUTUBE_REFRESH_TOKEN is filled in interactively
    /// on first use if not already set.
    #[arg(long = "upload-youtube")]
    upload_youtube: bool,

    /// Public HTTPS base URL under which this machine's `rd_media/`
    /// directory is reachable (e.g. your own CDN/reverse proxy).
    /// Required for --upload-instagram, since the Graph API fetches
    /// Reel video files from a URL rather than accepting a local-file
    /// upload. Example: https://media.example.com
    #[arg(long = "public-media-base-url")]
    public_media_base_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    config::load_dotenv();
    init_logging()?;
    CryptoProvider::install_default(default_provider())
        .map_err(|_| anyhow::anyhow!("default crypto provider already installed"))?;

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

/// Loads only the config needed for the platforms actually requested,
/// so e.g. running with just `--upload-instagram` never requires
/// YouTube credentials to be set. Returns a clear, specific error
/// (naming the missing env var) rather than failing generically.
fn build_social_config(flags: social::PublishFlags) -> Result<config::SocialConfig> {
    let instagram = if flags.instagram {
        Some(config::load_instagram_config()?)
    } else {
        None
    };
    let youtube = if flags.youtube {
        Some(config::load_youtube_config()?)
    } else {
        None
    };
    Ok(config::SocialConfig { instagram, youtube })
}

/// Publishes every city's generated video(s) to the requested
/// platform(s). Runs after generation is fully complete so a
/// publishing failure never interrupts or corrupts the local
/// image/video pipeline. Each city's Instagram and YouTube attempts
/// are independent -- one failing never skips the other -- and every
/// result (success or failure) is printed clearly.
async fn publish_generated_videos(
    social_config: &config::SocialConfig,
    flags: social::PublishFlags,
    date_ymd: &str,
    dict: &Dictionary,
    city_videos: &std::collections::HashMap<String, (Option<std::path::PathBuf>, Option<std::path::PathBuf>)>,
    public_media_base_url: Option<&str>,
) {
    if city_videos.is_empty() {
        println!("\nNo generated videos to publish.");
        return;
    }

    println!("\nPublishing...");
    for (english_city_name, (ig_path, yt_path)) in city_videos {
        let market_kannada = dict.city_display(english_city_name, dictionary::Language::Kannada);
        println!("\n{english_city_name}\n{}", "-".repeat(english_city_name.len()));

        let instagram_public_url = match (flags.instagram, ig_path, public_media_base_url) {
            (true, Some(path), Some(base)) => Some(build_public_media_url(base, date_ymd, english_city_name, path)),
            _ => None,
        };

        match social::publish_city(
            social_config,
            flags,
            date_ymd,
            english_city_name,
            &market_kannada,
            ig_path.as_deref(),
            yt_path.as_deref(),
            instagram_public_url.as_deref(),
        )
        .await
        {
            Ok(outcome) => {
                if let Some(result) = outcome.instagram {
                    match result {
                        Ok(media_id) => println!("✓ Instagram Reel published. Instagram media ID: {media_id}"),
                        Err(e) => println!("✗ Instagram upload failed\nReason: {e}"),
                    }
                }
                if let Some(result) = outcome.youtube {
                    match result {
                        Ok(video_id) => println!("✓ YouTube video published (public). YouTube video ID: {video_id}"),
                        Err(e) => println!("✗ YouTube upload failed\nReason: {e}"),
                    }
                }
            }
            Err(e) => {
                println!("✗ Publishing failed for {english_city_name}\nReason: {e}");
            }
        }
    }
}

/// Builds the public URL for a locally-generated video, given a base
/// URL under which the operator has made `rd_media/` reachable (e.g. a
/// reverse proxy or CDN pointed at that directory). Mirrors the UI
/// server's own `/media` mount (see `ui.rs`) so the same relative
/// layout is reused rather than inventing a different one.
fn build_public_media_url(
    base_url: &str,
    date_ymd: &str,
    english_city_name: &str,
    local_path: &std::path::Path,
) -> String {
    let folder = dictionary::city_folder_name(english_city_name);
    let filename = local_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default();
    format!("{}/{}/{}/{}", base_url.trim_end_matches('/'), date_ymd, folder, filename)
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
    // An --upload-* flag implies its corresponding variant must exist to
    // publish, even if the person only asked for the other one via --ig/--yt.
    let want_ig = args.ig || args.upload_instagram;
    let want_yt = args.yt || args.upload_youtube;
    let variants = match (want_ig, want_yt) {
        (true, false) => VariantSelection::InstagramOnly,
        (false, true) => VariantSelection::YoutubeOnly,
        _ => VariantSelection::Both,
    };

    let publish_flags = social::PublishFlags {
        instagram: args.upload_instagram,
        youtube: args.upload_youtube,
    };

    // Fail fast (before spending minutes scraping/rendering) if
    // publishing was requested but its credentials aren't configured.
    let social_config = build_social_config(publish_flags)?;
    let public_media_base_url = args
        .public_media_base_url
        .clone()
        .or_else(|| std::env::var("SOCIAL_PUBLIC_MEDIA_BASE_URL").ok().filter(|v| !v.trim().is_empty()));
    if publish_flags.instagram && public_media_base_url.is_none() {
        anyhow::bail!(
            "--upload-instagram requires --public-media-base-url (or SOCIAL_PUBLIC_MEDIA_BASE_URL): \
             the Instagram Graph API fetches Reel videos from a public HTTPS URL, not a local file."
        );
    }

    // force-all implies force-data, force-image, force-video.
    let force_data = args.force_data || args.force_all;
    let force_image = args.force_image || args.force_all;
    let force_video = args.force_video || args.force_all;

    // force-video / force-all / an --upload-* flag all override no-video:
    // there is nothing to publish without a generated video.
    let create_video = if force_video || publish_flags.any() {
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

    // Collect each city's video paths as they're produced, so the
    // publishing step below (which runs after all generation is done)
    // knows exactly which files to upload without re-deriving paths.
    let mut city_videos: std::collections::HashMap<String, (Option<std::path::PathBuf>, Option<std::path::PathBuf>)> =
        std::collections::HashMap::new();

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
            use render::MediaKind;
            match event.kind {
                MediaKind::InstagramVideo => {
                    city_videos.entry(event.city.clone()).or_default().0 = Some(event.path.clone());
                }
                MediaKind::YoutubeVideo => {
                    city_videos.entry(event.city.clone()).or_default().1 = Some(event.path.clone());
                }
                _ => {}
            }
            println!("Ready: {:?} for {} -> {}", event.kind, event.city, event.path.display());
        },
    )
    .await?;

    if publish_flags.any() {
        publish_generated_videos(
            &social_config,
            publish_flags,
            &date_ymd,
            &dict,
            &city_videos,
            public_media_base_url.as_deref(),
        )
        .await;
    }

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