//! Instagram Reel publishing via the official Meta Graph API.
//!
//! No browser automation, no Selenium/Playwright, no undocumented
//! endpoints -- this talks directly to
//! `https://graph.facebook.com/{version}/...` using the documented
//! Reels publishing flow:
//!
//! 1. `POST /{ig-user-id}/media` with `media_type=REELS`, `video_url`,
//!    `caption` -> returns a `creation_id` (a *container*, not yet
//!    published).
//! 2. Poll `GET /{creation_id}?fields=status_code` until the container's
//!    `status_code` is `FINISHED` (Meta processes the video
//!    asynchronously; polling immediately and publishing is a common
//!    source of silently-broken Reels).
//! 3. `POST /{ig-user-id}/media_publish` with the `creation_id` ->
//!    returns the published media's id. Only *this* step counts as a
//!    real publish; a `creation_id` alone is not a success.
//!
//! ## Known API limitation: video source
//!
//! The Graph API's `/media` endpoint expects `video_url` to be a
//! **publicly reachable HTTPS URL** that Meta's servers fetch the
//! video from -- it does not accept a local file path or a multipart
//! upload of raw bytes for Reels. This CLI generates videos to local
//! disk (`rd_media/...`), so before calling [`publish_reel`] the
//! caller must make the file reachable at a public URL (e.g. upload it
//! to your own storage/CDN, or reverse-proxy `rd_media/`) and pass
//! that URL in. There is currently no supported Graph API workaround
//! for uploading raw local bytes directly for Reels.
//!
//! ## Known API limitation: location tagging
//!
//! As of the current Graph API version, Reels published via
//! `/media` + `media_type=REELS` do not support an arbitrary
//! `location_id`/place tag the way regular feed photo/video posts do
//! (`location_id` is documented for `IMAGE`/`VIDEO` feed containers,
//! not `REELS`). Rather than silently dropping the requirement or
//! faking a tag, [`publish_reel`] keeps the market name in the caption
//! (which *is* fully supported) and exposes an unused
//! `location_id: Option<&str>` extension point so location tagging can
//! be wired in the moment Meta adds Reels support for it.

use crate::config::InstagramConfig;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::time::Duration;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);
const DEFAULT_MAX_POLLS: u32 = 60; // ~5 minutes at the default interval
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
pub struct ReelPublishResult {
    /// The published Reel's Instagram media id.
    pub media_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateContainerResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ContainerStatusResponse {
    status_code: String,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PublishResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GraphErrorEnvelope {
    error: GraphError,
}

#[derive(Debug, Deserialize)]
struct GraphError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<i64>,
}

fn redact(token: &str) -> String {
    if token.len() <= 8 {
        "***".to_string()
    } else {
        format!("{}...{}", &token[..4], &token[token.len() - 4..])
    }
}

/// Publishes `video_url` (a public HTTPS URL to the already-generated
/// 9:16 video) as an Instagram Reel with the given caption, and
/// returns the published Reel's media id.
///
/// `location_id` is accepted for forward-compatibility (see module
/// docs) but is currently a documented no-op for Reels; pass `None`.
pub async fn publish_reel(
    config: &InstagramConfig,
    video_url: &str,
    caption: &str,
    location_id: Option<&str>,
) -> Result<ReelPublishResult> {
    let client = reqwest::Client::new();
    let base = format!(
        "https://graph.facebook.com/{}/{}",
        config.graph_api_version, config.ig_user_id
    );

    if location_id.is_some() {
        log::warn!(
            "Instagram Reels do not currently support location_id tagging via the Graph API; \
             ignoring the requested location and keeping the market name in the caption only."
        );
    }

    // Step 1: create the media container. Deliberately not adding a
    // location_id param here: not supported for REELS as of the
    // current Graph API version -- see module docs.
    let params = vec![
        ("media_type", "REELS".to_string()),
        ("video_url", video_url.to_string()),
        ("caption", caption.to_string()),
        ("access_token", config.access_token.clone()),
    ];

    let create_resp = client
        .post(&format!("{base}/media"))
        .form(&params)
        .send()
        .await
        .with_context(|| "Instagram: failed to send media-container creation request")?;

    let create_status = create_resp.status();
    let create_body = create_resp
        .text()
        .await
        .with_context(|| "Instagram: failed to read media-container creation response")?;

    if !create_status.is_success() {
        bail!(
            "Instagram media-container creation failed ({}): {}",
            create_status,
            describe_graph_error(&create_body)
        );
    }

    let creation_id = serde_json::from_str::<CreateContainerResponse>(&create_body)
        .with_context(|| format!("Instagram: unexpected container-creation response: {create_body}"))?
        .id;

    // Step 2: poll until Meta finishes processing the video. Creating a
    // container does NOT mean it's ready to publish, and does NOT mean
    // publishing will succeed -- polling until FINISHED (or a bounded
    // timeout/error) is required so we never report success just
    // because the *creation* request was accepted.
    wait_until_ready(&client, &config.graph_api_version, &creation_id, &config.access_token).await?;

    // Step 3: publish. Only after this succeeds do we consider the
    // Reel actually published.
    let publish_params = [
        ("creation_id", creation_id.as_str()),
        ("access_token", config.access_token.as_str()),
    ];
    let publish_resp = client
        .post(&format!("{base}/media_publish"))
        .form(&publish_params)
        .send()
        .await
        .with_context(|| "Instagram: failed to send media_publish request")?;

    let publish_status = publish_resp.status();
    let publish_body = publish_resp
        .text()
        .await
        .with_context(|| "Instagram: failed to read media_publish response")?;

    if !publish_status.is_success() {
        bail!(
            "Instagram Reel publish failed ({}): {}",
            publish_status,
            describe_graph_error(&publish_body)
        );
    }

    let published = serde_json::from_str::<PublishResponse>(&publish_body)
        .with_context(|| format!("Instagram: unexpected media_publish response: {publish_body}"))?;

    log::info!(
        "Instagram Reel published: media_id={} (token {})",
        published.id,
        redact(&config.access_token)
    );

    Ok(ReelPublishResult {
        media_id: published.id,
    })
}

async fn wait_until_ready(
    client: &reqwest::Client,
    graph_api_version: &str,
    creation_id: &str,
    access_token: &str,
) -> Result<()> {
    let deadline = std::time::Instant::now() + DEFAULT_TIMEOUT;
    for attempt in 0..DEFAULT_MAX_POLLS {
        if std::time::Instant::now() > deadline {
            bail!("Instagram Reel processing timed out after {:?}", DEFAULT_TIMEOUT);
        }

        let resp = client
            .get(&format!(
                "https://graph.facebook.com/{graph_api_version}/{creation_id}"
            ))
            .query(&[("fields", "status_code"), ("access_token", access_token)])
            .send()
            .await
            .with_context(|| "Instagram: failed to poll media-container status")?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .with_context(|| "Instagram: failed to read media-container status response")?;

        if !status.is_success() {
            bail!(
                "Instagram media-container status check failed ({}): {}",
                status,
                describe_graph_error(&body)
            );
        }

        let parsed = serde_json::from_str::<ContainerStatusResponse>(&body)
            .with_context(|| format!("Instagram: unexpected status response: {body}"))?;

        match parsed.status_code.as_str() {
            "FINISHED" => return Ok(()),
            "ERROR" => bail!(
                "Instagram reported an error processing the Reel: {}",
                parsed.status.unwrap_or_default()
            ),
            "IN_PROGRESS" | "EXPIRED" | "PUBLISHED" if parsed.status_code == "EXPIRED" => {
                bail!("Instagram media container expired before it could be published")
            }
            _ => {
                log::debug!(
                    "Instagram Reel container {creation_id} not ready yet (status_code={}, attempt {}/{})",
                    parsed.status_code,
                    attempt + 1,
                    DEFAULT_MAX_POLLS
                );
                tokio::time::sleep(DEFAULT_POLL_INTERVAL).await;
            }
        }
    }

    bail!(
        "Instagram Reel processing did not finish after {} polling attempts",
        DEFAULT_MAX_POLLS
    )
}

fn describe_graph_error(body: &str) -> String {
    match serde_json::from_str::<GraphErrorEnvelope>(body) {
        Ok(env) => format!(
            "{}{}",
            env.error.message,
            match (env.error.error_type, env.error.code) {
                (Some(t), Some(c)) => format!(" (type={t}, code={c})"),
                (Some(t), None) => format!(" (type={t})"),
                (None, Some(c)) => format!(" (code={c})"),
                (None, None) => String::new(),
            }
        ),
        Err(_) => body.to_string(),
    }
}