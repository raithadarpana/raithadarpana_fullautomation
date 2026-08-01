pub mod assets;
pub mod data;
pub mod dictionary;
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

use dictionary::{Dictionary, Language};

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
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging()?;

    let args = Args::parse();

    if args.headless {
        run_headless(&args).await
    } else {
        ui::run_ui().await
    }
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

    println!("Fetching and scraping market report for {}...", date_ddmmyyyy);
    let report = scrape::scrape_agriculture_data(&date_ddmmyyyy, lang).await?;
    println!("Scrape complete: {} cities found.", report.cities.len());

    let json_path = storage::write_report_json(&date_ymd, &report)?;
    println!("Extracted JSON to: {}", json_path.display());

    let cities_filter: Option<Vec<String>> = args.cities.as_ref().map(|s| {
        s.split(',')
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect()
    });
    let filter_slice = cities_filter.as_deref();

    let outcome = render::render_report_images(&report, &date_ymd, &dict, lang, filter_slice, |msg| {
        println!("{}", msg);
    }, true)
    .await?;

    if outcome.written.is_empty() {
        println!("No images were rendered.");
        if !outcome.skipped_cities.is_empty() {
            println!("The following cities were found in the report but did not match your --cities filter:");
            for (scraped, resolved) in &outcome.skipped_cities {
                println!("  - scraped: '{}' -> resolved: '{}'", scraped, resolved);
            }
            println!("Check that --cities values match the resolved English name above (case-insensitive).");
        } else {
            println!("The report itself contained no cities. Check the JSON output for details.");
        }
    } else {
        for path in &outcome.written {
            println!("Rendered: {}", path.display());
        }
        if !outcome.skipped_cities.is_empty() {
            println!("Skipped {} cities not matching the filter.", outcome.skipped_cities.len());
        }
    }

    Ok(())
}