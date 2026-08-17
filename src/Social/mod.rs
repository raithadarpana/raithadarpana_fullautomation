//! Modular publishing layer that pushes already-generated videos to
//! Instagram (Reels) and YouTube. Kept fully separate from
//! `render`/`video` (which only ever produce local media files) so the
//! existing generation pipeline continues to work identically whether
//! or not publishing is enabled.

#[path = "Caption.rs"]
pub mod caption;
#[path = "Config.rs"]
pub mod config;
#[path = "Instagram.rs"]
pub mod instagram;
#[path = "Metadata.rs"]
pub mod metadata;
#[path = "Youtube.rs"]
pub mod youtube;

use crate::config::SocialConfig;
use anyhow::Result;
use std::path::Path;

/// Which platforms to publish to for a given run. Mirrors
/// `render::VariantSelection` but is intentionally a separate type:
/// generating a variant and publishing it are different decisions
/// (e.g. you may generate both but only want to auto-publish one).
#[derive(Debug, Clone, Copy, Default)]
pub struct PublishFlags {
    pub instagram: bool,
    pub youtube: bool,
}

impl PublishFlags {
    pub fn any(&self) -> bool {
        self.instagram || self.youtube
    }
}

/// Outcome of attempting to publish one city's videos, for CLI
/// logging. Each field is `None` if that platform wasn't requested,
/// `Some(Ok(..))` on success, `Some(Err(..))` on failure -- failures on
/// one platform never prevent the other from being attempted (see
/// requirement: "if Instagram fails but YouTube succeeds, report
/// both").
#[derive(Debug, Default)]
pub struct PublishOutcome {
    pub instagram: Option<Result<String>>,
    pub youtube: Option<Result<String>>,
}

/// Publishes one city's already-generated videos to whichever
/// platforms are requested in `flags`, skipping platforms whose
/// config is missing or that were already published for this
/// (date, city) per the persisted [`metadata::PublishStatus`].
///
/// `market_kannada_name` and `caption_date` are supplied by the caller
/// (derived from the app's selected-market/selected-date state -- see
/// `caption::build_social_caption`), never hard-coded here.
#[allow(clippy::too_many_arguments)]
pub async fn publish_city(
    config: &SocialConfig,
    flags: PublishFlags,
    date_ymd: &str,
    english_city_name: &str,
    market_kannada_name: &str,
    ig_video_path: Option<&Path>,
    yt_video_path: Option<&Path>,
    instagram_public_video_url: Option<&str>,
) -> Result<PublishOutcome> {
    let caption_date = caption::format_caption_date(date_ymd);
    let text = caption::build_social_caption(market_kannada_name, &caption_date);

    let mut outcome = PublishOutcome::default();
    let mut status = metadata::load(date_ymd, english_city_name).unwrap_or_default();
    let mut status_dirty = false;

    if flags.instagram {
        if let Some(existing) = &status.instagram_media_id {
            log::info!(
                "Instagram: {} on {} already published as media_id={}, skipping duplicate upload.",
                english_city_name,
                date_ymd,
                existing
            );
            outcome.instagram = Some(Ok(existing.clone()));
        } else {
            let result = publish_instagram(config, ig_video_path, instagram_public_video_url, &text).await;
            if let Ok(media_id) = &result {
                status.instagram_media_id = Some(media_id.clone());
                status_dirty = true;
            }
            outcome.instagram = Some(result);
        }
    }

    if flags.youtube {
        if let Some(existing) = &status.youtube_video_id {
            log::info!(
                "YouTube: {} on {} already published as video_id={}, skipping duplicate upload.",
                english_city_name,
                date_ymd,
                existing
            );
            outcome.youtube = Some(Ok(existing.clone()));
        } else {
            let result = publish_youtube(config, yt_video_path, &text).await;
            if let Ok(video_id) = &result {
                status.youtube_video_id = Some(video_id.clone());
                status_dirty = true;
            }
            outcome.youtube = Some(result);
        }
    }

    if status_dirty {
        if let Err(e) = metadata::save(&status) {
            log::warn!("Failed to persist publish-status metadata for {}: {}", english_city_name, e);
        }
    }

    Ok(outcome)
}

async fn publish_instagram(
    config: &SocialConfig,
    video_path: Option<&Path>,
    public_video_url: Option<&str>,
    caption: &str,
) -> Result<String> {
    let ig_config = config
        .instagram
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Instagram publishing requested but INSTAGRAM_ACCESS_TOKEN/INSTAGRAM_USER_ID are not configured"))?;

    let Some(video_path) = video_path else {
        anyhow::bail!("Instagram publishing requested but no 9:16 video was generated for this city");
    };
    if !video_path.exists() {
        anyhow::bail!("Instagram upload failed: video file does not exist: {}", video_path.display());
    }
    if std::fs::metadata(video_path).map(|m| m.len()).unwrap_or(0) == 0 {
        anyhow::bail!("Instagram upload failed: video file is empty (not fully written?): {}", video_path.display());
    }

    let video_url = public_video_url.ok_or_else(|| {
        anyhow::anyhow!(
            "Instagram publishing requires a publicly reachable HTTPS URL for the video \
             (the Graph API fetches it server-side; local file paths are not accepted). \
             Set SOCIAL_PUBLIC_MEDIA_BASE_URL to wherever rd_media/ is hosted, or otherwise \
             provide a public URL for {}.",
            video_path.display()
        )
    })?;

    let result = instagram::publish_reel(ig_config, video_url, caption, None).await?;
    Ok(result.media_id)
}

async fn publish_youtube(config: &SocialConfig, video_path: Option<&Path>, text: &str) -> Result<String> {
    let yt_config = config
        .youtube
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("YouTube publishing requested but YOUTUBE_CLIENT_ID/YOUTUBE_CLIENT_SECRET are not configured"))?;

    let Some(video_path) = video_path else {
        anyhow::bail!("YouTube publishing requested but no landscape video was generated for this city");
    };

    let access_token = youtube::ensure_access_token(yt_config).await?;

    let req = youtube::UploadRequest {
        video_path,
        title: text.to_string(),
        description: text.to_string(),
        tags: caption::youtube_tags(),
        privacy_status: "public".to_string(),
        recording_location: None, // see youtube.rs module docs: no lat/long source yet
    };

    let result = youtube::upload_video(&access_token, &req).await?;
    Ok(result.video_id)
}