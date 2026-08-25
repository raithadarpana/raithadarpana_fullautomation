//! Configuration for the social-media publishing layer.
//!
//! Credentials are never hard-coded. They are read from process
//! environment variables, optionally pre-populated by loading a `.env`
//! file (see [`load_dotenv`]) from the current working directory. This
//! mirrors the common Rust-CLI convention of "env vars are the
//! interface; `.env` is just a convenience for local/cron use".

use anyhow::{Context, Result};
use std::path::Path;

/// Instagram (Meta Graph API) credentials/config.
#[derive(Debug, Clone)]
pub struct InstagramConfig {
    /// Long-lived Page/Instagram access token with the
    /// `instagram_content_publish` permission.
    pub access_token: String,
    /// The Instagram professional account's numeric user id (the
    /// IG User ID backing the Page, not the @handle).
    pub ig_user_id: String,
    /// Graph API version to call, e.g. "v21.0". Configurable because
    /// Meta deprecates old versions on a rolling schedule.
    pub graph_api_version: String,
}

/// YouTube Data API OAuth 2.0 credentials/config.
#[derive(Debug, Clone)]
pub struct YoutubeConfig {
    pub client_id: String,
    pub client_secret: String,
    /// May be empty on first run; the OAuth flow in `social::youtube`
    /// will populate and persist it, after which it should be exported
    /// back into `.env`/the environment for subsequent runs.
    pub refresh_token: String,
}

#[derive(Debug, Clone, Default)]
pub struct SocialConfig {
    pub instagram: Option<InstagramConfig>,
    pub youtube: Option<YoutubeConfig>,
}

/// Loads a simple `KEY=VALUE` `.env` file (if present) into the process
/// environment, without overwriting variables that are already set
/// (so real environment variables — e.g. from a CI/cron environment —
/// always win over the file). Lines starting with `#`, and blank
/// lines, are ignored. This intentionally avoids pulling in a
/// third-party dotenv crate for such a small amount of parsing.
pub fn load_dotenv() {
    load_dotenv_from(Path::new(".env"));
}

fn load_dotenv_from(path: &Path) {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return;
    };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let mut value = value.trim();
        if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
            || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
        {
            value = &value[1..value.len() - 1];
        }
        if key.is_empty() {
            continue;
        }
        if std::env::var(key).is_err() {
            // SAFETY: single-threaded startup path, before any other
            // code reads these env vars.
            unsafe {
                std::env::set_var(key, value);
            }
        }
    }
}

fn env_var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// Loads Instagram config from `INSTAGRAM_ACCESS_TOKEN` /
/// `INSTAGRAM_USER_ID` (and optional `INSTAGRAM_GRAPH_API_VERSION`).
/// Returns a clear, actionable error naming exactly which variable is
/// missing rather than a generic failure.
pub fn load_instagram_config() -> Result<InstagramConfig> {
    let access_token = env_var("INSTAGRAM_ACCESS_TOKEN")
        .context("INSTAGRAM_ACCESS_TOKEN is not set. Set it in your environment or .env file.")?;
    let ig_user_id = env_var("INSTAGRAM_USER_ID")
        .context("INSTAGRAM_USER_ID is not set. Set it in your environment or .env file.")?;
    let graph_api_version =
        env_var("INSTAGRAM_GRAPH_API_VERSION").unwrap_or_else(|| "v21.0".to_string());
    Ok(InstagramConfig {
        access_token,
        ig_user_id,
        graph_api_version,
    })
}

/// Loads YouTube OAuth config from `YOUTUBE_CLIENT_ID` /
/// `YOUTUBE_CLIENT_SECRET` / `YOUTUBE_REFRESH_TOKEN`. The refresh
/// token is allowed to be empty (first-run device/installed-app flow
/// fills it in interactively) but client id/secret are always
/// required since they identify the OAuth app itself.
pub fn load_youtube_config() -> Result<YoutubeConfig> {
    let client_id = env_var("YOUTUBE_CLIENT_ID")
        .context("YOUTUBE_CLIENT_ID is not set. Set it in your environment or .env file.")?;
    let client_secret = env_var("YOUTUBE_CLIENT_SECRET")
        .context("YOUTUBE_CLIENT_SECRET is not set. Set it in your environment or .env file.")?;
    let refresh_token = env_var("YOUTUBE_REFRESH_TOKEN").unwrap_or_default();
    Ok(YoutubeConfig {
        client_id,
        client_secret,
        refresh_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_instagram_vars_produce_useful_error() {
        // Deliberately not setting the vars in this process; whichever
        // is unset (or both, in a clean test environment) should
        // surface a message naming the specific variable.
        if env_var("INSTAGRAM_ACCESS_TOKEN").is_none() {
            let err = load_instagram_config().unwrap_err();
            assert!(err.to_string().contains("INSTAGRAM_ACCESS_TOKEN"));
        }
    }
}