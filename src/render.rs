use crate::data::{AgriculturalReport, CityMarketData};
use crate::dictionary::{Dictionary, Language};
use crate::storage;
use crate::templates::{self, INSTAGRAM_HEIGHT, INSTAGRAM_WIDTH, YOUTUBE_HEIGHT, YOUTUBE_WIDTH};
use anyhow::Result;
use base64::Engine;
use headless_chrome::{
    protocol::cdp::Page::{self, CaptureScreenshotFormatOption},
    Browser, LaunchOptions,
};
use std::fs;
use std::path::PathBuf;

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
}

pub async fn render_report_images(
    report: &AgriculturalReport,
    date_ymd: &str,
    dict: &Dictionary,
    lang: Language,
    cities_filter: Option<&[String]>,
) -> Result<RenderOutcome> {
    let launch_options = LaunchOptions::default_builder()
        .window_size(Some((INSTAGRAM_WIDTH.max(YOUTUBE_WIDTH), INSTAGRAM_HEIGHT.max(YOUTUBE_HEIGHT))))
        .build()
        .unwrap();
    let browser = Browser::new(launch_options)?;
    let mut outcome = RenderOutcome::default();

    for city in &report.cities {
        let english_name = storage::resolve_english_city_name(dict, &city.city_name);

        if let Some(filter) = cities_filter {
            if !filter.is_empty()
                && !filter
                    .iter()
                    .any(|f| f.trim().eq_ignore_ascii_case(english_name.trim()))
            {
                log::info!(
                    "Skipping city '{}' (resolved: '{}') - not in filter {:?}",
                    city.city_name,
                    english_name,
                    filter
                );
                outcome
                    .skipped_cities
                    .push((city.city_name.clone(), english_name));
                continue;
            }
        }

        log::info!("Rendering images for city '{}' -> '{}'", city.city_name, english_name);
        outcome.rendered_cities.push(english_name.clone());

        let ig_path = render_city_variant(
            &browser,
            city,
            &report.report_date,
            date_ymd,
            &english_name,
            dict,
            lang,
            Variant::Instagram,
        )?;
        outcome.written.push(ig_path);

        let yt_path = render_city_variant(
            &browser,
            city,
            &report.report_date,
            date_ymd,
            &english_name,
            dict,
            lang,
            Variant::YouTube,
        )?;
        outcome.written.push(yt_path);
    }

    Ok(outcome)
}

enum Variant {
    Instagram,
    YouTube,
}

fn render_city_variant(
    browser: &Browser,
    city: &CityMarketData,
    report_date: &str,
    date_ymd: &str,
    english_city_name: &str,
    dict: &Dictionary,
    lang: Language,
    variant: Variant,
) -> Result<PathBuf> {
    let (html, width, height, out_path) = match variant {
        Variant::Instagram => (
            templates::instagram_html(city, report_date, dict, lang),
            INSTAGRAM_WIDTH,
            INSTAGRAM_HEIGHT,
            storage::instagram_image_path(date_ymd, english_city_name)?,
        ),
        Variant::YouTube => (
            templates::youtube_html(city, report_date, dict, lang),
            YOUTUBE_WIDTH,
            YOUTUBE_HEIGHT,
            storage::youtube_image_path(date_ymd, english_city_name)?,
        ),
    };

    // Save the rendered HTML into rd_media alongside the images, both so
    // it can be inspected for debugging and so it serves as the actual
    // source file navigated to (avoiding any separate temp-file path
    // issues). Written next to the image it corresponds to.
    let html_path = out_path.with_extension("html");
    fs::write(&html_path, &html)
        .map_err(|e| anyhow::anyhow!("Failed to write debug HTML to {}: {}", html_path.display(), e))?;

    let tab = browser.new_tab()?;
    let canonical_html_path = html_path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve path {}: {}", html_path.display(), e))?;
    let url = format!("file://{}", canonical_html_path.display());
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