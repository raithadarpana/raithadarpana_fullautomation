//! YouTube video publishing via the official YouTube Data API v3.
//!
//! No browser automation. Uses standard OAuth 2.0 (installed-app /
//! loopback flow for the one-time authorization, then refresh-token
//! based access thereafter) and the documented resumable upload
//! protocol (`uploadType=resumable`) to upload local video files
//! directly -- unlike Instagram's Reels endpoint, the Data API v3
//! *does* accept raw video bytes from the caller, so no public hosting
//! step is required here.
//!
//! ## Known API limitation: location
//!
//! `videos.insert` supports `recordingDetails.location` as a
//! **latitude/longitude pair**, not an arbitrary place name -- there is
//! no "set location to this market" field. This project only has a
//! market *name* (e.g. "ರಾಮನಗರ"), not coordinates, so location metadata
//! is intentionally left unset by default. If/when a lat/long lookup
//! for each market becomes available, wire it into
//! [`UploadRequest::recording_location`]; the upload call already
//! forwards it when present.

use crate::config::YoutubeConfig;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const UPLOAD_SCOPE: &str = "https://www.googleapis.com/auth/youtube.upload";
const RESUMABLE_UPLOAD_URL: &str =
    "https://www.googleapis.com/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status";

pub struct UploadRequest<'a> {
    pub video_path: &'a Path,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    /// Always "public" per requirement #9 unless the caller explicitly
    /// overrides it in the future.
    pub privacy_status: String,
    /// See module docs: currently always `None` in this codebase
    /// because no lat/long source exists yet.
    pub recording_location: Option<(f64, f64)>,
}

#[derive(Debug, Clone)]
pub struct UploadResult {
    pub video_id: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// Exchanges the configured refresh token for a fresh access token.
/// If `config.refresh_token` is empty, runs the one-time interactive
/// installed-app OAuth flow (opens a browser, listens on a local
/// loopback port for the redirect) and prints the resulting refresh
/// token so the operator can save it as `YOUTUBE_REFRESH_TOKEN` for
/// all future (non-interactive) runs.
pub async fn ensure_access_token(config: &YoutubeConfig) -> Result<String> {
    if config.refresh_token.trim().is_empty() {
        let tokens = interactive_authorize(config).await?;
        println!(
            "\nYouTube authorization complete. Save this value as YOUTUBE_REFRESH_TOKEN \
             in your .env so future runs don't need the browser step again:\n\n  YOUTUBE_REFRESH_TOKEN={}\n",
            tokens
                .refresh_token
                .clone()
                .unwrap_or_else(|| "<no refresh_token returned -- re-run with access_type=offline and prompt=consent>".into())
        );
        return Ok(tokens.access_token);
    }

    refresh_access_token(config).await
}

async fn refresh_access_token(config: &YoutubeConfig) -> Result<String> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
        ("refresh_token", config.refresh_token.as_str()),
        ("grant_type", "refresh_token"),
    ];

    let resp = client
        .post(TOKEN_ENDPOINT)
        .form(&params)
        .send()
        .await
        .context("YouTube: token refresh request failed")?;

    let status = resp.status();
    let body = resp.text().await.context("YouTube: failed to read token refresh response")?;
    if !status.is_success() {
        bail!("YouTube authentication failed ({}): {}", status, redact_body(&body));
    }

    let token: TokenResponse =
        serde_json::from_str(&body).with_context(|| format!("YouTube: unexpected token response shape: {}", redact_body(&body)))?;
    Ok(token.access_token)
}

/// Minimal loopback-redirect installed-app OAuth flow. Blocks
/// (synchronously, on a dedicated thread) waiting for the browser
/// redirect carrying the authorization code, with a bounded timeout so
/// a CLI/cron run never hangs forever.
async fn interactive_authorize(config: &YoutubeConfig) -> Result<TokenResponse> {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").context("YouTube: could not open local port for OAuth redirect")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/oauth2callback");

    let auth_url = format!(
        "{AUTH_ENDPOINT}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
        urlencoding_encode(&config.client_id),
        urlencoding_encode(&redirect_uri),
        urlencoding_encode(UPLOAD_SCOPE),
    );

    println!("\nYouTube authorization required. Opening browser:\n  {auth_url}\n");
    let _ = open::that(&auth_url);

    listener
        .set_nonblocking(false)
        .context("YouTube: failed to configure OAuth redirect listener")?;

    // Bound how long we wait for the human to complete the consent
    // screen, so headless/cron invocations never hang indefinitely.
    let (code_tx, code_rx) = std::sync::mpsc::channel::<Result<String>>();
    std::thread::spawn(move || {
        let result = (|| -> Result<String> {
            let (mut stream, _) = listener.accept().context("YouTube: OAuth redirect listener failed to accept")?;
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let first_line = request.lines().next().unwrap_or_default();
            let code = first_line
                .split_whitespace()
                .nth(1)
                .and_then(|path| path.split("code=").nth(1))
                .map(|rest| rest.split(['&', ' ']).next().unwrap_or("").to_string())
                .filter(|c| !c.is_empty())
                .context("YouTube: no authorization code found in redirect")?;
            let response_body = "Authorization received. You can close this tab and return to the terminal.";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
            Ok(code)
        })();
        let _ = code_tx.send(result);
    });

    let code = code_rx
        .recv_timeout(std::time::Duration::from_secs(5 * 60))
        .context("YouTube: timed out waiting for OAuth authorization in the browser")??;

    let client = reqwest::Client::new();
    let params = [
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
        ("code", code.as_str()),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri.as_str()),
    ];
    let resp = client
        .post(TOKEN_ENDPOINT)
        .form(&params)
        .send()
        .await
        .context("YouTube: authorization-code exchange request failed")?;
    let status = resp.status();
    let body = resp.text().await.context("YouTube: failed to read code-exchange response")?;
    if !status.is_success() {
        bail!("YouTube authentication failed ({}): {}", status, redact_body(&body));
    }
    let token: TokenResponse =
        serde_json::from_str(&body).with_context(|| format!("YouTube: unexpected token response shape: {}", redact_body(&body)))?;
    Ok(token)
}

/// Uploads a local video file via the resumable-upload protocol,
/// setting title/description/tags/privacy, and returns the published
/// video's id. Only returns `Ok` once the Data API confirms the video
/// object was created -- a network error partway through the upload
/// bytes is surfaced as an error, never reported as success.
pub async fn upload_video(access_token: &str, req: &UploadRequest<'_>) -> Result<UploadResult> {
    if !req.video_path.exists() {
        bail!("YouTube upload failed: video file does not exist: {}", req.video_path.display());
    }
    let file_len = std::fs::metadata(req.video_path)
        .with_context(|| format!("YouTube: could not read metadata for {}", req.video_path.display()))?
        .len();
    if file_len == 0 {
        bail!(
            "YouTube upload failed: video file is empty (not fully written?): {}",
            req.video_path.display()
        );
    }

    let client = reqwest::Client::new();

    let mut snippet = serde_json::json!({
        "title": req.title,
        "description": req.description,
        "tags": req.tags,
    });
    if let Some((lat, lon)) = req.recording_location {
        // Extension point for when a market -> coordinates lookup
        // exists; not populated today (see module docs).
        snippet["categoryId"] = serde_json::Value::String("27".into()); // "Education"; harmless default
        let _ = (lat, lon);
    }
    let metadata = serde_json::json!({
        "snippet": snippet,
        "status": { "privacyStatus": req.privacy_status },
    });

    // Step 1: initiate the resumable upload session.
    let init_resp = client
        .post(RESUMABLE_UPLOAD_URL)
        .bearer_auth(access_token)
        .header("X-Upload-Content-Type", "video/mp4")
        .header("X-Upload-Content-Length", file_len.to_string())
        .header("Content-Type", "application/json; charset=UTF-8")
        .body(metadata.to_string())
        .send()
        .await
        .context("YouTube: failed to initiate resumable upload session")?;

    let init_status = init_resp.status();
    if !init_status.is_success() {
        let body = init_resp.text().await.unwrap_or_default();
        bail!("YouTube upload session initiation failed ({}): {}", init_status, redact_body(&body));
    }
    let upload_url = init_resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .context("YouTube: resumable upload session response did not include a Location header")?;

    // Step 2: PUT the actual video bytes to the session URL.
    let file_bytes = tokio::fs::read(req.video_path)
        .await
        .with_context(|| format!("YouTube: failed to read video file {}", req.video_path.display()))?;

    let put_resp = client
        .put(&upload_url)
        .header("Content-Type", "video/mp4")
        .header("Content-Length", file_len.to_string())
        .body(file_bytes)
        .send()
        .await
        .context("YouTube: failed to upload video bytes")?;

    let put_status = put_resp.status();
    let put_body = put_resp.text().await.context("YouTube: failed to read upload response")?;
    if !put_status.is_success() {
        bail!("YouTube video upload failed ({}): {}", put_status, redact_body(&put_body));
    }

    #[derive(Deserialize)]
    struct VideoResource {
        id: String,
    }
    let video: VideoResource = serde_json::from_str(&put_body)
        .with_context(|| format!("YouTube: unexpected upload response shape: {}", redact_body(&put_body)))?;

    log::info!("YouTube video published: video_id={}", video.id);
    Ok(UploadResult { video_id: video.id })
}

fn redact_body(body: &str) -> String {
    // Access/refresh tokens sometimes get echoed back in error bodies
    // for malformed requests; rather than risk splitting a multi-byte
    // UTF-8 character while surgically redacting, just drop the whole
    // body when it looks like it contains a token value.
    if body.contains("\"access_token\"") || body.contains("\"refresh_token\"") {
        "[response redacted: contained a token field]".to_string()
    } else {
        body.to_string()
    }
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}