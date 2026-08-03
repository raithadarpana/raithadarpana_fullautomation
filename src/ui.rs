//! Web UI for the interactive (non-headless) mode.
//!
//! Replaces the previous ratatui terminal UI with a local web server: the
//! person picks language, date, and cities in a browser, watches progress
//! and generated media stream in live, and can preview/playback results
//! without restarting the application.

use crate::data::AgriculturalReport;
use crate::dictionary::{Dictionary, Language};
use crate::render::{self, ForceFlags, MediaKind, VariantSelection};
use crate::scrape;
use crate::storage;

use anyhow::Result;
use axum::{
    extract::{Path as AxPath, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use uuid::Uuid;

/// Shared application state for the web UI server.
struct AppState {
    dict: Dictionary,
    jobs: Mutex<HashMap<String, Job>>,
    /// Cached reports keyed by "YYYYMMDD-lang" so switching between
    /// screens (or re-rendering) doesn't require re-scraping.
    reports: Mutex<HashMap<String, AgriculturalReport>>,
}

/// A single piece of media as sent to the frontend, already resolved to
/// a `/media/...` URL.
#[derive(Debug, Clone, Serialize)]
struct MediaItem {
    city: String,
    kind: String, // "ig_image" | "yt_image" | "ig_video" | "yt_video"
    url: String,
}

/// A single pipeline run's live progress state, polled by the frontend.
#[derive(Debug, Clone, Serialize, Default)]
struct Job {
    status: JobStatusRepr,
    /// Latest progress message only, so the UI can show a single
    /// overwriting status line instead of a scrolling log.
    current_message: String,
    /// Media items in the order they became available, for streaming.
    media: Vec<MediaItem>,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum JobStatusRepr {
    Running,
    Done,
    Failed,
}

impl Default for JobStatusRepr {
    fn default() -> Self {
        JobStatusRepr::Running
    }
}

/// Entrypoint for the interactive UI mode: starts a local web server and
/// opens it in the person's default browser.
pub async fn run_ui() -> Result<()> {
    let dict = Dictionary::load();
    let state = Arc::new(AppState {
        dict,
        jobs: Mutex::new(HashMap::new()),
        reports: Mutex::new(HashMap::new()),
    });

    std::fs::create_dir_all(storage::RD_MEDIA_ROOT)?;

    let app = Router::new()
        .route("/", get(index_page))
        .route("/api/fetch", post(fetch_data))
        .route("/api/render", post(start_render))
        .route("/api/jobs/:job_id", get(job_status))
        .nest_service("/media", ServeDir::new(storage::RD_MEDIA_ROOT))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let url = format!("http://{}", addr);

    println!("Raitha Darpana web UI running at {}", url);
    log::info!("Web UI listening on {}", url);

    // Best-effort: if a browser can't be opened (headless server, no
    // display, etc.), the person can still copy the printed URL.
    if let Err(e) = open::that(&url) {
        log::warn!("Could not auto-open browser: {}", e);
        println!("Open this URL in your browser: {}", url);
    }

    axum::serve(listener, app).await?;
    Ok(())
}

async fn index_page() -> Html<&'static str> {
    Html(INDEX_HTML)
}

#[derive(Debug, Deserialize)]
struct FetchRequest {
    language: String,
    /// dd/mm/yyyy
    date: String,
}

#[derive(Debug, Serialize)]
struct FetchResponse {
    report_date: String,
    cities: Vec<String>,
    date_ymd: String,
}

fn report_cache_key(date_ymd: &str, lang: Language) -> String {
    format!("{}-{}", date_ymd, storage::lang_short(lang))
}

/// Fetches (or reuses cached) report data for a date/language and returns
/// the resolved city list actually present in that report, so the person
/// can pick from cities that exist in the data rather than the full
/// dictionary.
async fn fetch_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FetchRequest>,
) -> Result<Json<FetchResponse>, (StatusCode, String)> {
    let lang = Language::from_str(&req.language)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid language".to_string()))?;

    let parts: Vec<&str> = req.date.split('/').collect();
    if parts.len() != 3 {
        return Err((StatusCode::BAD_REQUEST, "Invalid date, expected dd/mm/yyyy".to_string()));
    }
    let date_ymd = format!("{}{}{}", parts[2], parts[1], parts[0]);

    let needs_refresh = storage::needs_data_refresh(&date_ymd, lang, std::time::Duration::from_secs(4 * 60 * 60), false)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let report = if needs_refresh {
        let report = scrape::scrape_agriculture_data(&req.date, lang)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Scrape failed: {}", e)))?;
        storage::clear_day_artifacts(&date_ymd, lang).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        storage::write_report_json(&date_ymd, lang, &report)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        report
    } else {
        storage::read_report_json(&date_ymd, lang)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "Missing cached report".to_string()))?
    };

    let mut seen = std::collections::HashSet::new();
    let mut cities = Vec::new();
    for city in &report.cities {
        let english = storage::resolve_english_city_name(&state.dict, &city.city_name);
        if seen.insert(english.clone()) {
            cities.push(english);
        }
    }
    cities.sort();

    let response = FetchResponse {
        report_date: report.report_date.clone(),
        cities,
        date_ymd: date_ymd.clone(),
    };

    state
        .reports
        .lock()
        .await
        .insert(report_cache_key(&date_ymd, lang), report);

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct RenderRequest {
    language: String,
    date_ymd: String,
    /// Empty = all cities.
    cities: Vec<String>,
    ig: bool,
    yt: bool,
    no_video: bool,
    force_data: bool,
    force_image: bool,
    force_video: bool,
    force_all: bool,
    /// Empty = use the language default.
    voice: Option<String>,
    /// Signed percentage, e.g. "+20%". Empty/None = default.
    rate: Option<String>,
    /// Signed percentage, e.g. "+0%". Empty/None = default.
    volume: Option<String>,
    /// Signed Hz value, e.g. "+0Hz". Empty/None = default.
    pitch: Option<String>,
    /// Background music volume as a fraction (0.0-1.0). None = default (0.08).
    bg_music_volume: Option<f64>,
    /// Silence padding in seconds added before/after the voiceover. None = default (3).
    padding_secs: Option<f64>,
}

#[derive(Debug, Serialize)]
struct RenderStartResponse {
    job_id: String,
}

async fn start_render(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RenderRequest>,
) -> Result<Json<RenderStartResponse>, (StatusCode, String)> {
    let lang = Language::from_str(&req.language)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid language".to_string()))?;

    let job_id = Uuid::new_v4().to_string();
    state.jobs.lock().await.insert(job_id.clone(), Job::default());

    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_pipeline_job(state_clone.clone(), job_id_clone.clone(), lang, req).await {
            let mut jobs = state_clone.jobs.lock().await;
            if let Some(job) = jobs.get_mut(&job_id_clone) {
                job.status = JobStatusRepr::Failed;
                job.error = Some(e.to_string());
            }
        }
    });

    Ok(Json(RenderStartResponse { job_id }))
}

async fn run_pipeline_job(
    state: Arc<AppState>,
    job_id: String,
    lang: Language,
    req: RenderRequest,
) -> Result<()> {
    let date_ymd = req.date_ymd.clone();

    let force_data = req.force_data || req.force_all;
    let force_image = req.force_image || req.force_all;
    let force_video = req.force_video || req.force_all;
    let create_video = if force_video { true } else { !req.no_video };

    let variants = match (req.ig, req.yt) {
        (true, false) => VariantSelection::InstagramOnly,
        (false, true) => VariantSelection::YoutubeOnly,
        _ => VariantSelection::Both,
    };

    let needs_refresh = storage::needs_data_refresh(&date_ymd, lang, std::time::Duration::from_secs(4 * 60 * 60), force_data)?;

    let cache_key = report_cache_key(&date_ymd, lang);
    let cached = state.reports.lock().await.get(&cache_key).cloned();

    let report = if needs_refresh || cached.is_none() {
        push_message(&state, &job_id, "Fetching fresh data...".to_string()).await;
        // Reconstruct dd/mm/yyyy from YYYYMMDD for the scraper.
        let y = &date_ymd[0..4];
        let m = &date_ymd[4..6];
        let d = &date_ymd[6..8];
        let date_ddmmyyyy = format!("{}/{}/{}", d, m, y);

        let report = scrape::scrape_agriculture_data(&date_ddmmyyyy, lang).await?;
        storage::clear_day_artifacts(&date_ymd, lang)?;
        storage::write_report_json(&date_ymd, lang, &report)?;
        state.reports.lock().await.insert(cache_key, report.clone());
        report
    } else {
        cached.unwrap()
    };

    let cities_filter: Option<Vec<String>> = if req.cities.is_empty() {
        None
    } else {
        Some(req.cities.clone())
    };
    let filter_slice = cities_filter.as_deref();

    let force = ForceFlags {
        force_image,
        force_video,
    };

    let non_empty = |s: &Option<String>| s.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()).map(|v| v.to_string());
    let voice_settings = render::VoiceSettings {
        voice: non_empty(&req.voice),
        rate: non_empty(&req.rate),
        volume: non_empty(&req.volume),
        pitch: non_empty(&req.pitch),
        bg_music_volume: req.bg_music_volume,
        padding_secs: req.padding_secs,
    };

    let state_for_progress = state.clone();
    let job_id_for_progress = job_id.clone();
    let progress = move |msg: &str| {
        let state = state_for_progress.clone();
        let job_id = job_id_for_progress.clone();
        let msg = msg.to_string();
        // Block on the (uncontended, in-memory) state update so messages
        // are recorded in strict order and immediately, rather than
        // racing the synchronous pipeline via a spawned task (which was
        // causing the displayed progress to lag one step behind).
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                push_message(&state, &job_id, msg).await;
            });
        });
    };

    let state_for_media = state.clone();
    let job_id_for_media = job_id.clone();
    let on_media = move |event: render::MediaEvent| {
        let state = state_for_media.clone();
        let job_id = job_id_for_media.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                push_media(&state, &job_id, event).await;
            });
        });
    };

    render::render_report_images(
        &report,
        &date_ymd,
        &state.dict,
        lang,
        filter_slice,
        variants,
        force,
        create_video,
        &voice_settings,
        progress,
        on_media,
    )
    .await?;

    let mut jobs = state.jobs.lock().await;
    if let Some(job) = jobs.get_mut(&job_id) {
        job.status = JobStatusRepr::Done;
        job.current_message = "Pipeline complete.".to_string();
    }

    Ok(())
}

/// Converts an on-disk path under `rd_media/` into a `/media/...` URL the
/// frontend can fetch/play directly.
fn to_media_url(path: &std::path::Path) -> String {
    let root = std::path::Path::new(storage::RD_MEDIA_ROOT);
    match path.strip_prefix(root) {
        Ok(rel) => format!("/media/{}", rel.to_string_lossy().replace('\\', "/")),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

fn media_kind_str(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::InstagramImage => "ig_image",
        MediaKind::YoutubeImage => "yt_image",
        MediaKind::InstagramVideo => "ig_video",
        MediaKind::YoutubeVideo => "yt_video",
        MediaKind::VoiceAudio => "voice_audio",
        MediaKind::MixedAudio => "mixed_audio",
    }
}

async fn push_message(state: &Arc<AppState>, job_id: &str, msg: String) {
    log::info!("[job {}] {}", job_id, msg);
    let mut jobs = state.jobs.lock().await;
    if let Some(job) = jobs.get_mut(job_id) {
        job.current_message = msg;
    }
}

async fn push_media(state: &Arc<AppState>, job_id: &str, event: render::MediaEvent) {
    let item = MediaItem {
        city: event.city,
        kind: media_kind_str(event.kind).to_string(),
        url: to_media_url(&event.path),
    };
    let mut jobs = state.jobs.lock().await;
    if let Some(job) = jobs.get_mut(job_id) {
        job.media.push(item);
    }
}

async fn job_status(
    State(state): State<Arc<AppState>>,
    AxPath(job_id): AxPath<String>,
) -> impl IntoResponse {
    let jobs = state.jobs.lock().await;
    match jobs.get(&job_id) {
        Some(job) => Json(job.clone()).into_response(),
        None => (StatusCode::NOT_FOUND, "Job not found").into_response(),
    }
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Raitha Darpana - Content Creator</title>
<style>
  * { box-sizing: border-box; }
  body {
    font-family: -apple-system, Segoe UI, Roboto, sans-serif;
    margin: 0; padding: 0; background: #f2f5f2; color: #1a1a1a;
    display: flex; height: 100vh; overflow: hidden;
  }
  h1 { color: #1c5c2e; font-size: 1.3rem; margin: 0 0 1rem 0; }
  fieldset { border: 1px solid #cfe0d0; border-radius: 8px; margin-bottom: 1rem; padding: 0.8rem; }
  legend { font-weight: 600; color: #1c5c2e; padding: 0 0.5rem; font-size: 0.9rem; }
  label { display: inline-flex; align-items: center; gap: 0.3rem; margin-right: 1rem; font-size: 0.9rem; }
  select, input[type=date], input[type=text] { padding: 0.4rem; border-radius: 6px; border: 1px solid #ccc; font-size: 0.9rem; }
  button { background: #1c5c2e; color: white; border: none; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; font-size: 0.9rem; }
  button.secondary { background: #6b8f70; }
  button:hover { filter: brightness(1.1); }
  button:disabled { background: #999; cursor: not-allowed; }

  #leftPanel {
    width: 560px; min-width: 480px; padding: 1rem; overflow-y: auto;
    background: #fff; border-right: 1px solid #dfe8df;
  }
  #rightPanel {
    flex: 1; padding: 1rem; overflow-y: auto;
  }

  .row { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; margin-bottom: 0.5rem; }
  .optionsGrid { display: grid; grid-template-columns: 1fr; gap: 0.7rem 1rem; }
  .optionsGrid .field { display: flex; flex-direction: column; gap: 0.2rem; }
  .optionsGrid .field label { margin-right: 0; font-weight: 600; font-size: 0.8rem; color: #3a5a3f; }
  .optionsGrid .field select, .optionsGrid .field input[type=text] { width: 100%; }
  .hint { font-size: 0.75rem; color: #789; margin-top: 0.15rem; }

  .voiceOptionsGrid { display: grid; grid-template-columns: 1fr auto; gap: 0.7rem 1rem; }
  .voiceOptionsGrid .field { display: flex; flex-direction: column; gap: 0.2rem; }
  .voiceOptionsGrid .field label { margin-right: 0; font-weight: 600; font-size: 0.8rem; color: #3a5a3f; }
  .voiceOptionsGrid .field select, .optionsGrid .field input[type=text] { width: 100%; }

  .topRow { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; }
  .topRow label { margin-right: 0.3rem; }
  .topRow button { white-space: nowrap; }

  .sideBySide { display: flex; gap: 0.8rem; align-items: flex-start; }
  .sideBySide fieldset { flex: 1; min-width: 0; margin-bottom: 1rem; }

  #citySearch { width: 100%; margin-bottom: 0.5rem; }
  #cityControls { display: flex; align-items: center; margin-bottom: 0.5rem; }
  #cityControls label { margin-right: 0; font-size: 0.85rem; font-weight: 600; }
  #cityList { height: 120px; overflow-y: auto; border: 1px solid #ddd; border-radius: 6px; padding: 0.4rem; background: #fafcfa; }
  #cityList label { display: flex; margin: 0.15rem 0; font-size: 0.85rem; margin-right: 0; }
  #cityCount { font-size: 0.8rem; color: #557; margin-bottom: 0.4rem; }

  #status {
    background: #10241a; color: #b6f2c9; font-family: monospace; padding: 0.6rem 0.8rem;
    border-radius: 8px; font-size: 0.82rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }

  .city-section { margin-top: 2.5rem; }
  .city-header {
    display: flex; align-items: center; gap: 1rem; flex-wrap: wrap;
    border-bottom: 1px solid #dfe8df; padding-bottom: 0.3rem; margin-bottom: 0.5rem;
  }
  .city-header h3 { margin: 0; color: #1c5c2e; }
  .city-header .audio-slot { display: flex; align-items: center; gap: 0.3rem; }
  .city-header .audio-slot .label { font-size: 0.7rem; color: #557; }
  .city-header audio { height: 30px; width: 225px; margin-right: 1.5rem;}

  .media-row { display: flex; gap: 0.6rem; flex-wrap: wrap; }
  .media-card {
    background: white; padding: 0.3rem; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.08);
    display: flex; flex-direction: column; align-items: center; gap: 0.2rem;
  }
  .media-card img {
    height: 130px; width: auto; max-width: 100%; border-radius: 6px; border: 1px solid #ddd;
    object-fit: contain; cursor: zoom-in;
  }
  .media-card video { max-height: 130px; width: auto; border-radius: 6px; border: 1px solid #ddd; object-fit: contain; }
  .media-card .label { font-size: 0.7rem; color: #557; }

  #runBtn { width: 100%; padding: 0.7rem; font-size: 1rem; margin-top: 0.5rem; }

  #imageModalOverlay {
    display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.75);
    z-index: 1000; align-items: center; justify-content: center;
  }
  #imageModalOverlay.open { display: flex; }
  #imageModalOverlay img {
    max-width: 92vw; max-height: 92vh; border-radius: 8px; box-shadow: 0 4px 24px rgba(0,0,0,0.4);
  }
  #imageModalClose {
    position: absolute; top: 1.2rem; right: 1.5rem; background: white; color: #1a1a1a;
    border: none; width: 2.2rem; height: 2.2rem; border-radius: 50%; font-size: 1.1rem;
    cursor: pointer; line-height: 1;
  }
</style>
</head>
<body>

<div id="leftPanel">
  <h1>Raitha Darpana Media Creator</h1>

  <fieldset>
    <legend>1. Language, Date &amp; Fetch</legend>
    <div class="topRow">
      <label>Language:
        <select id="language">
          <option value="kannada">Kannada</option>
          <option value="english">English</option>
        </select>
      </label>
      <label>Date: <input type="date" id="date"></label>
      <button id="fetchBtn">Fetch data</button>
    </div>
  </fieldset>

  <div class="sideBySide">
    <fieldset>
      <legend>2. Cities</legend>
      <input type="text" id="citySearch" placeholder="Search cities...">
      <div id="cityControls">
        <label><input type="checkbox" id="allCitiesCb"> All</label>
      </div>
      <div id="cityCount">Fetch data first to see available cities.</div>
      <div id="cityList"></div>
    </fieldset>

    <fieldset>
      <legend>3. Options</legend>
      <div class="optionsGrid">
        <label><input type="checkbox" id="ig"> Instagram only</label>
        <label><input type="checkbox" id="yt"> YouTube only</label>
        <label><input type="checkbox" id="noVideo"> No video (images only)</label>
        <label><input type="checkbox" id="forceData"> Force re-fetch data</label>
        <label><input type="checkbox" id="forceImage"> Force re-create images</label>
        <label><input type="checkbox" id="forceVideo"> Force re-create videos</label>
        <label><input type="checkbox" id="forceAll"> Force all</label>
      </div>
    </fieldset>
  </div>

  <fieldset>
    <legend>4. Voice and Audio settings</legend>
    <div class="voiceOptionsGrid">
      <div class="field">
        <label for="voice">Speaker</label>
        <select id="voice">
          <option value="">Default</option>
        </select>
      </div>
      <div class="field">
        <label for="rate">Rate</label>
        <input type="text" id="rate" placeholder="+20%">
        <div class="hint">Signed %, e.g. +20% or -10%</div>
      </div>
      <div class="field">
        <label for="volume">Voice volume</label>
        <input type="text" id="volume" placeholder="+0%">
        <div class="hint">Signed %, e.g. +0% or -5%</div>
      </div>
      <div class="field">
        <label for="pitch">Pitch</label>
        <input type="text" id="pitch" placeholder="+0Hz">
        <div class="hint">Signed Hz, e.g. +0Hz or -20Hz</div>
      </div>
      <div class="field">
        <label for="bgMusicVolume">Background music volume (%)</label>
        <input type="text" id="bgMusicVolume" placeholder="8">
        <div class="hint">0-100, default 8</div>
      </div>
      <div class="field">
        <label for="paddingSecs">Padding (seconds)</label>
        <input type="text" id="paddingSecs" placeholder="3">
        <div class="hint">Silence before/after voiceover, default 3</div>
      </div>
    </div>
  </fieldset>

  <button id="runBtn" disabled>Run pipeline</button>

  <h3 style="margin-top:1rem;">Progress</h3>
  <div id="status">Idle.</div>
</div>

<div id="rightPanel">
  <div id="output"></div>
</div>

<div id="imageModalOverlay">
  <button id="imageModalClose" aria-label="Close">&times;</button>
  <img id="imageModalImg" src="" alt="">
</div>

<script>
let dateYmd = null;
let allCities = [];
// Persists the set of selected cities across search re-renders, since
// filtering the list removes non-matching checkboxes from the DOM
// entirely (losing their checked state otherwise).
let selectedCities = new Set();

const VOICE_CHOICES = {
  kannada: [
    { id: 'kn-IN-GaganNeural', label: 'Gagan (Male)' },
    { id: 'kn-IN-SapnaNeural', label: 'Sapna (Female)' }
  ],
  english: [
    { id: 'en-IN-PrabhatNeural', label: 'Prabhat (Male)' },
    { id: 'en-IN-NeerjaNeural', label: 'Neerja (Female)' }
  ]
};

function populateVoiceChoices() {
  const lang = document.getElementById('language').value;
  const voiceSel = document.getElementById('voice');
  const prevValue = voiceSel.value;
  voiceSel.innerHTML = '<option value="">Default</option>';
  (VOICE_CHOICES[lang] || []).forEach(v => {
    const opt = document.createElement('option');
    opt.value = v.id;
    opt.textContent = v.label;
    voiceSel.appendChild(opt);
  });
  // Keep the previous selection if it's still valid for the new
  // language, otherwise fall back to Default.
  if ((VOICE_CHOICES[lang] || []).some(v => v.id === prevValue)) {
    voiceSel.value = prevValue;
  }
}

document.getElementById('language').addEventListener('change', populateVoiceChoices);
populateVoiceChoices();

document.getElementById('date').valueAsDate = new Date();

function ddmmyyyy(dateStr) {
  const [y, m, d] = dateStr.split('-');
  return `${d}/${m}/${y}`;
}

// Reads a percentage-valued input (0-100) and converts to a 0.0-1.0
// fraction for the backend. Returns null (use default) if empty/invalid.
function parsePercentField(id) {
  const raw = document.getElementById(id).value.trim();
  if (raw === '') return null;
  const n = parseFloat(raw);
  if (isNaN(n)) return null;
  return n / 100.0;
}

// Reads a plain numeric input. Returns null (use default) if empty/invalid.
function parseNumberField(id) {
  const raw = document.getElementById(id).value.trim();
  if (raw === '') return null;
  const n = parseFloat(raw);
  if (isNaN(n)) return null;
  return n;
}

function renderCityList(filterText) {
  const cityList = document.getElementById('cityList');
  cityList.innerHTML = '';
  const filtered = allCities.filter(c => c.toLowerCase().includes(filterText.toLowerCase()));
  filtered.forEach(city => {
    const label = document.createElement('label');
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.value = city;
    cb.checked = selectedCities.has(city);
    label.appendChild(cb);
    label.appendChild(document.createTextNode(city));
    cityList.appendChild(label);
  });
  updateCityCount();
  syncAllCitiesCheckbox();
}

function updateCityCount() {
  const total = allCities.length;
  const selected = selectedCities.size;
  const visible = document.querySelectorAll('#cityList input').length;
  document.getElementById('cityCount').textContent =
    `${selected} of ${total} selected (${visible} shown)`;
}

// Keeps the "All" checkbox in sync with whether every known city is
// currently selected (checked, unchecked, or indeterminate when it's a
// partial selection).
function syncAllCitiesCheckbox() {
  const allCb = document.getElementById('allCitiesCb');
  if (allCities.length === 0) {
    allCb.checked = false;
    allCb.indeterminate = false;
    return;
  }
  if (selectedCities.size === allCities.length) {
    allCb.checked = true;
    allCb.indeterminate = false;
  } else if (selectedCities.size === 0) {
    allCb.checked = false;
    allCb.indeterminate = false;
  } else {
    allCb.checked = false;
    allCb.indeterminate = true;
  }
}

document.getElementById('citySearch').addEventListener('input', (e) => {
  renderCityList(e.target.value);
});

document.getElementById('allCitiesCb').addEventListener('change', (e) => {
  if (e.target.checked) {
    allCities.forEach(c => selectedCities.add(c));
  } else {
    selectedCities.clear();
  }
  renderCityList(document.getElementById('citySearch').value);
});

document.getElementById('cityList').addEventListener('change', (e) => {
  const cb = e.target;
  if (cb && cb.type === 'checkbox') {
    if (cb.checked) selectedCities.add(cb.value);
    else selectedCities.delete(cb.value);
  }
  updateCityCount();
  syncAllCitiesCheckbox();
});

document.getElementById('fetchBtn').addEventListener('click', async () => {
  const language = document.getElementById('language').value;
  const dateVal = document.getElementById('date').value;
  if (!dateVal) { alert('Pick a date'); return; }
  const date = ddmmyyyy(dateVal);

  const btn = document.getElementById('fetchBtn');
  btn.disabled = true;
  btn.textContent = 'Fetching...';
  try {
    const resp = await fetch('/api/fetch', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({ language, date })
    });
    if (!resp.ok) throw new Error(await resp.text());
    const data = await resp.json();
    dateYmd = data.date_ymd;
    allCities = data.cities;
    selectedCities.clear();
    document.getElementById('citySearch').value = '';
    renderCityList('');

    document.getElementById('runBtn').disabled = false;
  } catch (e) {
    alert('Fetch failed: ' + e.message);
  } finally {
    btn.disabled = false;
    btn.textContent = 'Fetch data';
  }
});

document.getElementById('runBtn').addEventListener('click', async () => {
  if (!dateYmd) { alert('Fetch data first'); return; }

  const cities = Array.from(selectedCities);
  if (cities.length === 0) { alert('Select at least one city'); return; }
  const allSelected = cities.length === allCities.length;

  const body = {
    language: document.getElementById('language').value,
    date_ymd: dateYmd,
    cities: allSelected ? [] : cities,
    ig: document.getElementById('ig').checked,
    yt: document.getElementById('yt').checked,
    no_video: document.getElementById('noVideo').checked,
    force_data: document.getElementById('forceData').checked,
    force_image: document.getElementById('forceImage').checked,
    force_video: document.getElementById('forceVideo').checked,
    force_all: document.getElementById('forceAll').checked,
    voice: document.getElementById('voice').value || null,
    rate: document.getElementById('rate').value || null,
    volume: document.getElementById('volume').value || null,
    pitch: document.getElementById('pitch').value || null,
    bg_music_volume: parsePercentField('bgMusicVolume'),
    padding_secs: parseNumberField('paddingSecs')
  };

  const runBtn = document.getElementById('runBtn');
  runBtn.disabled = true;
  runBtn.textContent = 'Running...';
  document.getElementById('status').textContent = 'Starting...';
  document.getElementById('output').innerHTML = '';

  try {
    const resp = await fetch('/api/render', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify(body)
    });
    if (!resp.ok) throw new Error(await resp.text());
    const { job_id } = await resp.json();
    pollJob(job_id);
  } catch (e) {
    alert('Failed to start: ' + e.message);
    runBtn.disabled = false;
    runBtn.textContent = 'Run pipeline';
  }
});

const cityMediaSections = new Map(); // city -> DOM row element
const renderedMediaKeys = new Set(); // `${city}:${kind}` already rendered

const KIND_LABELS = {
  ig_image: 'Instagram (image)',
  yt_image: 'YouTube (image)',
  ig_video: 'Instagram (video)',
  yt_video: 'YouTube (video)',
  voice_audio: 'Voiceover',
  mixed_audio: 'Mixed Audio'
};

// Returns { headerAudioSlot, detailsRow } for a city, creating the
// section (header row with city name + audio, details row with
// images/videos) on first use.
function getOrCreateCitySection(city) {
  if (cityMediaSections.has(city)) return cityMediaSections.get(city);

  const section = document.createElement('div');
  section.className = 'city-section';

  const header = document.createElement('div');
  header.className = 'city-header';
  const heading = document.createElement('h3');
  heading.textContent = city;
  header.appendChild(heading);

  const detailsRow = document.createElement('div');
  detailsRow.className = 'media-row';

  section.appendChild(header);
  section.appendChild(detailsRow);
  document.getElementById('output').appendChild(section);

  const refs = { header, detailsRow };
  cityMediaSections.set(city, refs);
  return refs;
}

function openImageModal(src) {
  document.getElementById('imageModalImg').src = src;
  document.getElementById('imageModalOverlay').classList.add('open');
}

function closeImageModal() {
  document.getElementById('imageModalOverlay').classList.remove('open');
  document.getElementById('imageModalImg').src = '';
}

document.getElementById('imageModalClose').addEventListener('click', closeImageModal);
document.getElementById('imageModalOverlay').addEventListener('click', (e) => {
  // Only close when clicking the transparent overlay itself, not the image.
  if (e.target === document.getElementById('imageModalOverlay')) closeImageModal();
});

function appendMediaItem(item) {
  const key = `${item.city}:${item.kind}`;
  if (renderedMediaKeys.has(key)) return;
  renderedMediaKeys.add(key);

  const { header, detailsRow } = getOrCreateCitySection(item.city);

  // Voice narration and mixed audio go in the header row, next to the
  // city name, instead of taking up a separate card in the details row.
  if (item.kind === 'voice_audio' || item.kind === 'mixed_audio') {
    const slot = document.createElement('div');
    slot.className = 'audio-slot';
    const label = document.createElement('div');
    label.className = 'label';
    label.textContent = KIND_LABELS[item.kind] || item.kind;
    const audio = document.createElement('audio');
    audio.src = item.url;
    audio.controls = true;
    slot.appendChild(label);
    slot.appendChild(audio);
    header.appendChild(slot);
    return;
  }

  const card = document.createElement('div');
  card.className = 'media-card';

  if (item.kind === 'ig_image' || item.kind === 'yt_image') {
    const img = document.createElement('img');
    img.src = item.url;
    img.addEventListener('click', () => openImageModal(item.url));
    card.appendChild(img);
  } else {
    const vid = document.createElement('video');
    vid.src = item.url;
    vid.controls = true;
    card.appendChild(vid);
  }

  const label = document.createElement('div');
  label.className = 'label';
  label.textContent = KIND_LABELS[item.kind] || item.kind;
  card.appendChild(label);

  detailsRow.appendChild(card);
}

function pollJob(jobId) {
  const statusEl = document.getElementById('status');
  cityMediaSections.clear();
  renderedMediaKeys.clear();

  const interval = setInterval(async () => {
    const resp = await fetch('/api/jobs/' + jobId);
    if (!resp.ok) { clearInterval(interval); return; }
    const job = await resp.json();
    statusEl.textContent = job.current_message || '...';

    job.media.forEach(appendMediaItem);

    if (job.status === 'done' || job.status === 'failed') {
      clearInterval(interval);
      const runBtn = document.getElementById('runBtn');
      runBtn.disabled = false;
      runBtn.textContent = 'Run pipeline';

      if (job.status === 'failed') {
        alert('Pipeline failed: ' + (job.error || 'unknown error'));
      }
    }
  }, 600);
}
</script>
</body>
</html>
"#;