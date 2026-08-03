use crate::assets::{self, BrandingAssets};
use crate::data::{AgriculturalReport, CityMarketData};
use crate::dictionary::{Dictionary, Language};
use crate::storage;
use crate::templates::{self, INSTAGRAM_HEIGHT, INSTAGRAM_WIDTH, YOUTUBE_HEIGHT, YOUTUBE_WIDTH};
use crate::video;
use crate::voiceover;
pub use crate::voiceover::VoiceSettings;

use anyhow::Result;
use base64::Engine;
use headless_chrome::{
    protocol::cdp::Page::{self, CaptureScreenshotFormatOption},
    Browser, LaunchOptions, Tab,
};
use std::sync::Arc;
use std::fs;
use std::path::PathBuf;

/// Convenience default for callers (e.g. the headless CLI) that don't
/// need to customize TTS voice/rate/volume/pitch.
pub fn voiceover_settings_default() -> VoiceSettings {
    VoiceSettings::default()
}

/// Which cover variant(s) to produce for each city.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariantSelection {
    Both,
    InstagramOnly,
    YoutubeOnly,
}

impl VariantSelection {
    pub fn wants_instagram(&self) -> bool {
        matches!(self, VariantSelection::Both | VariantSelection::InstagramOnly)
    }
    pub fn wants_youtube(&self) -> bool {
        matches!(self, VariantSelection::Both | VariantSelection::YoutubeOnly)
    }
}

/// Flags controlling when existing artifacts are regenerated. Mirrors the
/// CLI's `--force-*` flags (see `main.rs`).
#[derive(Debug, Clone, Copy, Default)]
pub struct ForceFlags {
    pub force_image: bool,
    pub force_video: bool,
}

/// Kind of media artifact just produced, for streaming progress to a UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    InstagramImage,
    YoutubeImage,
    InstagramVideo,
    YoutubeVideo,
    VoiceAudio,
    MixedAudio,
}

/// A single piece of media as soon as it's ready on disk, so a UI can
/// display it immediately rather than waiting for the whole pipeline.
#[derive(Debug, Clone)]
pub struct MediaEvent {
    pub city: String,
    pub kind: MediaKind,
    pub path: PathBuf,
}

/// Renders Instagram (4:5) and YouTube (16:9) cover images for every
/// city in the report, storing them under `rd_media/YYYYMMDD/city_name/`.
///
/// `date_ymd` should be in `YYYYMMDD` format and is used both for the
/// output folder and the filename suffix. `cities_filter`, if provided
/// and non-empty, restricts rendering to matching English city names.
/// Diagnostics for a render pass: which cities were rendered vs skipped,
/// and why -- so a filter mismatch produces a visible explanation
/// instead of silently writing nothing.
#[derive(Debug, Default)]
pub struct RenderOutcome {
    pub written: Vec<PathBuf>,
    pub rendered_cities: Vec<String>,
    pub skipped_cities: Vec<(String, String)>, // (scraped_name, resolved_english_name)
    pub videos_written: Vec<PathBuf>,
}

#[allow(clippy::too_many_arguments)]
pub async fn render_report_images(
    report: &AgriculturalReport,
    date_ymd: &str,
    dict: &Dictionary,
    lang: Language,
    cities_filter: Option<&[String]>,
    variants: VariantSelection,
    force: ForceFlags,
    create_video: bool,
    voice_settings: &VoiceSettings,
    mut on_progress: impl FnMut(&str),
    mut on_media: impl FnMut(MediaEvent),
) -> Result<RenderOutcome> {
    on_progress("Launching renderer...");
    let launch_options = LaunchOptions::default_builder()
        .window_size(Some((INSTAGRAM_WIDTH.max(YOUTUBE_WIDTH), INSTAGRAM_HEIGHT.max(YOUTUBE_HEIGHT))))
        .build()
        .unwrap();
    let browser = Browser::new(launch_options)?;
    // Reuse a single tab for every image across every city, rather than
    // opening a new tab per render. Previously each Instagram/YouTube
    // image opened its own tab via `browser.new_tab()` and never closed
    // it (closing a tab in headless Chrome can itself hang until
    // timeout, see rust-headless-chrome#434). Left-open tabs piled up
    // across cities and, after enough of them, the underlying Chrome
    // connection would drop entirely -- surfacing as "Unable to make
    // method calls because underlying connection is closed" partway
    // through the second (or later) city. Navigating a single long-lived
    // tab to each new HTML file avoids the tab buildup altogether.
    let tab = browser.new_tab()?;
    let mut outcome = RenderOutcome::default();

    // Loaded once per run (not per city) since the background files
    // don't change mid-run and reading + base64-encoding them is a bit of
    // work worth avoiding on every iteration.
    let branding = BrandingAssets::load();

    for city in &report.cities {
        let english_name = storage::resolve_english_city_name(dict, &city.city_name);

        if let Some(filter) = cities_filter {
            if !filter.is_empty()
                && !filter
                    .iter()
                    .any(|f| f.trim().eq_ignore_ascii_case(english_name.trim()))
            {
                // log::info!(
                //     "Skipping city '{}' (resolved: '{}') - not in filter {:?}",
                //     city.city_name,
                //     english_name,
                //     filter
                // );
                outcome
                    .skipped_cities
                    .push((city.city_name.clone(), english_name));
                continue;
            }
        }

        log::info!("Rendering images for city '{}' -> '{}'", city.city_name, english_name);
        outcome.rendered_cities.push(english_name.clone());

        let mut ig_path_opt: Option<PathBuf> = None;
        let mut yt_path_opt: Option<PathBuf> = None;

        if variants.wants_instagram() {
            let ig_path = storage::instagram_image_path(date_ymd, &english_name, lang)?;
            if force.force_image || !ig_path.exists() {
                on_progress(&format!("Rendering {} (Instagram)...", english_name));
                let written = render_city_variant(
                    &tab,
                    city,
                    &report.report_date,
                    date_ymd,
                    &english_name,
                    dict,
                    lang,
                    &branding,
                    Variant::Instagram,
                )?;
                outcome.written.push(written.clone());
                on_media(MediaEvent {
                    city: english_name.clone(),
                    kind: MediaKind::InstagramImage,
                    path: written.clone(),
                });
                ig_path_opt = Some(written);
            } else {
                on_progress(&format!("{} (Instagram) already exists, skipping.", english_name));
                on_media(MediaEvent {
                    city: english_name.clone(),
                    kind: MediaKind::InstagramImage,
                    path: ig_path.clone(),
                });
                ig_path_opt = Some(ig_path);
            }
        }

        if variants.wants_youtube() {
            let yt_path = storage::youtube_image_path(date_ymd, &english_name, lang)?;
            if force.force_image || !yt_path.exists() {
                on_progress(&format!("Rendering {} (YouTube)...", english_name));
                let written = render_city_variant(
                    &tab,
                    city,
                    &report.report_date,
                    date_ymd,
                    &english_name,
                    dict,
                    lang,
                    &branding,
                    Variant::YouTube,
                )?;
                outcome.written.push(written.clone());
                on_media(MediaEvent {
                    city: english_name.clone(),
                    kind: MediaKind::YoutubeImage,
                    path: written.clone(),
                });
                yt_path_opt = Some(written);
            } else {
                on_progress(&format!("{} (YouTube) already exists, skipping.", english_name));
                on_media(MediaEvent {
                    city: english_name.clone(),
                    kind: MediaKind::YoutubeImage,
                    path: yt_path.clone(),
                });
                yt_path_opt = Some(yt_path);
            }
        }

        if create_video {
            match generate_city_video_assets(
                city,
                &report.report_date,
                date_ymd,
                &english_name,
                lang,
                variants,
                force,
                ig_path_opt.as_deref(),
                yt_path_opt.as_deref(),
                voice_settings,
                &mut on_progress,
                &mut on_media,
            )
            .await
            {
                Ok(mut videos) => outcome.videos_written.append(&mut videos),
                Err(e) => {
                    log::error!("Video generation failed for '{}': {}", english_name, e);
                    on_progress(&format!("Video generation failed for {}: {}", english_name, e));
                }
            }
        }
    }

    on_progress("Render complete.");
    Ok(outcome)
}

/// Generates the voiceover, mixed audio, and video(s) for a single city,
/// honoring `force.force_video` to control regeneration. Requires the
/// corresponding cover image(s) to already exist on disk (either just
/// rendered above, or from a prior run).
#[allow(clippy::too_many_arguments)]
async fn generate_city_video_assets(
    city: &CityMarketData,
    report_date: &str,
    date_ymd: &str,
    english_city_name: &str,
    lang: Language,
    variants: VariantSelection,
    force: ForceFlags,
    ig_image_path: Option<&std::path::Path>,
    yt_image_path: Option<&std::path::Path>,
    voice_settings: &VoiceSettings,
    on_progress: &mut impl FnMut(&str),
    on_media: &mut impl FnMut(MediaEvent),
) -> Result<Vec<PathBuf>> {
    let mut written_videos = Vec::new();

    let ig_video_path = storage::instagram_video_path(date_ymd, english_city_name, lang)?;
    let yt_video_path = storage::youtube_video_path(date_ymd, english_city_name, lang)?;

    let need_ig_video = variants.wants_instagram() && (force.force_video || !ig_video_path.exists());
    let need_yt_video = variants.wants_youtube() && (force.force_video || !yt_video_path.exists());

    if !need_ig_video && !need_yt_video {
        on_progress(&format!("{}: videos already exist, skipping.", english_city_name));
        if variants.wants_instagram() && ig_video_path.exists() {
            on_media(MediaEvent {
                city: english_city_name.to_string(),
                kind: MediaKind::InstagramVideo,
                path: ig_video_path,
            });
        }
        if variants.wants_youtube() && yt_video_path.exists() {
            on_media(MediaEvent {
                city: english_city_name.to_string(),
                kind: MediaKind::YoutubeVideo,
                path: yt_video_path,
            });
        }
        let voice_path = storage::voice_audio_path(date_ymd, english_city_name, lang)?;
        let mixed_path = storage::mixed_audio_path(date_ymd, english_city_name, lang)?;
        if voice_path.exists() {
            on_media(MediaEvent {
                city: english_city_name.to_string(),
                kind: MediaKind::VoiceAudio,
                path: voice_path,
            });
        }
        if mixed_path.exists() {
            on_media(MediaEvent {
                city: english_city_name.to_string(),
                kind: MediaKind::MixedAudio,
                path: mixed_path,
            });
        }
        return Ok(written_videos);
    }

    // Voiceover + mixed audio are shared between the Instagram and
    // YouTube video variants, so generate them once per city.
    let voice_path = storage::voice_audio_path(date_ymd, english_city_name, lang)?;
    let mixed_path = storage::mixed_audio_path(date_ymd, english_city_name, lang)?;

    let audio_path = if force.force_video || (!voice_path.exists() && !mixed_path.exists()) {
        on_progress(&format!("Generating voiceover for {}...", english_city_name));
        let top_items: Vec<crate::data::CommodityEntry> =
            templates::top_commodities(city).into_iter().cloned().collect();
        let script = voiceover::generate_script(lang, english_city_name, report_date, &top_items);
        let assets_dir = assets::assets_dir();
        let path = voiceover::generate_audio_file(&script, lang, &voice_path, &mixed_path, &assets_dir, voice_settings)
            .await
            .map_err(|e| anyhow::anyhow!("Voiceover generation failed: {}", e))?;
        if voice_path.exists() {
            on_media(MediaEvent {
                city: english_city_name.to_string(),
                kind: MediaKind::VoiceAudio,
                path: voice_path.clone(),
            });
        }
        if mixed_path.exists() {
            on_media(MediaEvent {
                city: english_city_name.to_string(),
                kind: MediaKind::MixedAudio,
                path: mixed_path.clone(),
            });
        }
        path
    } else if mixed_path.exists() {
        mixed_path.clone()
    } else {
        voice_path.clone()
    };

    if need_ig_video {
        if let Some(image_path) = ig_image_path {
            on_progress(&format!("Generating Instagram video for {}...", english_city_name));
            video::generate_video(image_path, &audio_path, &ig_video_path, true)
                .map_err(|e| anyhow::anyhow!("Instagram video generation failed: {}", e))?;
            on_media(MediaEvent {
                city: english_city_name.to_string(),
                kind: MediaKind::InstagramVideo,
                path: ig_video_path.clone(),
            });
            written_videos.push(ig_video_path);
        } else {
            log::warn!(
                "Skipping Instagram video for '{}': cover image unavailable.",
                english_city_name
            );
        }
    }

    if need_yt_video {
        if let Some(image_path) = yt_image_path {
            on_progress(&format!("Generating YouTube video for {}...", english_city_name));
            video::generate_video(image_path, &audio_path, &yt_video_path, false)
                .map_err(|e| anyhow::anyhow!("YouTube video generation failed: {}", e))?;
            on_media(MediaEvent {
                city: english_city_name.to_string(),
                kind: MediaKind::YoutubeVideo,
                path: yt_video_path.clone(),
            });
            written_videos.push(yt_video_path);
        } else {
            log::warn!(
                "Skipping YouTube video for '{}': cover image unavailable.",
                english_city_name
            );
        }
    }

    Ok(written_videos)
}

enum Variant {
    Instagram,
    YouTube,
}

/// Converts a canonicalized filesystem path into a `file://` URL that
/// Chrome (via headless_chrome/CDP) will accept.
///
/// Two things `Path::display()` doesn't handle correctly here:
///
/// 1. On Windows, `canonicalize()` returns a "verbatim" path prefixed
///    with `\\?\` (e.g. `\\?\D:\Projects\...`) and using backslashes.
///    Chrome's URL parser doesn't understand that prefix and rejects the
///    navigation outright ("Cannot navigate to invalid URL"). The prefix
///    must be stripped and backslashes converted to forward slashes.
/// 2. City names (and therefore folder/file names) are often non-ASCII
///    (Kannada script). A raw `file://` URL isn't allowed to contain
///    those bytes unescaped, so each path segment is percent-encoded.
fn path_to_file_url(path: &std::path::Path) -> Result<String> {
    let mut path_str = path.to_string_lossy().to_string();

    // Strip the Windows verbatim-path prefix, if present.
    if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
        path_str = stripped.to_string();
    }

    // Normalize to forward slashes (no-op on Unix).
    let path_str = path_str.replace('\\', "/");

    let encoded_segments: Vec<String> = path_str.split('/').map(percent_encode_segment).collect();
    let encoded_path = encoded_segments.join("/");

    // On Windows the normalized path looks like "D:/Projects/..." and
    // needs a leading slash to form a valid `file:///D:/...` URL. On
    // Unix it already starts with "/", so plain "file://" is correct.
    if encoded_path.len() >= 2 && encoded_path.as_bytes()[1] == b':' {
        Ok(format!("file:///{}", encoded_path))
    } else {
        Ok(format!("file://{}", encoded_path))
    }
}

/// Percent-encodes a single path segment for use in a `file://` URL.
/// Keeps URL-safe ASCII characters as-is (including `:` and `.`, needed
/// for Windows drive letters and file extensions) and escapes everything
/// else, including non-ASCII UTF-8 bytes.
fn percent_encode_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b':' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

fn render_city_variant(
    tab: &Arc<Tab>,
    city: &CityMarketData,
    report_date: &str,
    date_ymd: &str,
    english_city_name: &str,
    dict: &Dictionary,
    lang: Language,
    branding: &BrandingAssets,
    variant: Variant,
) -> Result<PathBuf> {
    let (html, width, height, out_path) = match variant {
        Variant::Instagram => (
            templates::instagram_html(city, report_date, dict, lang, branding),
            INSTAGRAM_WIDTH,
            INSTAGRAM_HEIGHT,
            storage::instagram_image_path(date_ymd, english_city_name, lang)?,
        ),
        Variant::YouTube => (
            templates::youtube_html(city, report_date, dict, lang, branding),
            YOUTUBE_WIDTH,
            YOUTUBE_HEIGHT,
            storage::youtube_image_path(date_ymd, english_city_name, lang)?,
        ),
    };

    // Save the rendered HTML into rd_media alongside the images, both so
    // it can be inspected for debugging and so it serves as the actual
    // source file navigated to (avoiding any separate temp-file path
    // issues). Written next to the image it corresponds to.
    let html_path = out_path.with_extension("html");
    fs::write(&html_path, &html)
        .map_err(|e| anyhow::anyhow!("Failed to write debug HTML to {}: {}", html_path.display(), e))?;

    let canonical_html_path = html_path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve path {}: {}", html_path.display(), e))?;
    let url = path_to_file_url(&canonical_html_path)?;
    log::info!("Navigating to {} (city: {})", url, english_city_name);
    tab.navigate_to(&url)
        .map_err(|e| anyhow::anyhow!("Navigate failed for '{}' (path: {}): {}", url, canonical_html_path.display(), e))?;
    tab.wait_for_element("table")
        .map_err(|e| anyhow::anyhow!("Failed waiting for table element in {}: {}", url, e))?;

    // The CDP clip region below crops to the exact target dimensions,
    // regardless of the browser window's actual size, so an explicit
    // window resize isn't required for correct output dimensions.
    let clip = Page::Viewport {
        x: 0.0,
        y: 0.0,
        width: width as f64,
        height: height as f64,
        scale: 1.0,
    };

    let screenshot = tab.call_method(Page::CaptureScreenshot {
        format: Some(CaptureScreenshotFormatOption::Png),
        quality: None,
        clip: Some(clip),
        from_surface: Some(true),
        capture_beyond_viewport: Some(true),
        optimize_for_speed: None,
    })?;
    let png_data = base64::engine::general_purpose::STANDARD.decode(screenshot.data)?;

    fs::write(&out_path, png_data)
        .map_err(|e| anyhow::anyhow!("Failed to write image to {}: {}", out_path.display(), e))?;

    Ok(out_path)
}