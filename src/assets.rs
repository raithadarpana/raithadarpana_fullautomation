use base64::Engine;
use std::path::{Path, PathBuf};

/// Directory where branding assets (logo.png, background.png) are expected.
///
/// Resolved relative to the running binary's location (not the current
/// working directory), so the compiled binary can be copied/deployed
/// anywhere and the assets simply live alongside it in an `assets/`
/// folder. Swapping in a new logo or background is just replacing the
/// PNG on disk -- no recompilation needed, and the next run picks it up
/// automatically.
///
/// Falls back to `./assets` (relative to CWD) if the binary's own
/// `assets` folder doesn't exist, which is convenient when running via
/// `cargo run` during development. Override with the `RD_ASSETS_DIR`
/// env var if you need a different location (e.g. for cron jobs with an
/// unusual working directory).
pub fn assets_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("RD_ASSETS_DIR") {
        return PathBuf::from(dir);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join("assets");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }

    PathBuf::from("assets")
}

/// Branding images for the generated covers, loaded once per run and
/// inlined as base64 data URIs.
///
/// Data URIs (rather than a `file://` background or a relative `<img
/// src>`) are used deliberately: headless_chrome navigates to a
/// `file://` URL for the generated HTML, and relative image paths from
/// that context are easy to get wrong (working directory vs. HTML
/// location vs. asset location). Inlining the image bytes directly into
/// the HTML sidesteps all of that -- the page is fully self-contained.
///
/// Missing files are not a hard error: a logo or background you haven't
/// added yet just doesn't render (see the CSS fallbacks in
/// `templates.rs`), and a warning is logged so it's easy to notice.
#[derive(Debug, Clone, Default)]
pub struct BrandingAssets {
    pub logo_data_uri: Option<String>,
    pub background_data_uri: Option<String>,
}

impl BrandingAssets {
    pub fn load() -> Self {
        let dir = assets_dir();
        Self {
            logo_data_uri: load_as_data_uri(&dir.join("logo.png")),
            background_data_uri: load_as_data_uri(&dir.join("background.png")),
        }
    }
}

fn load_as_data_uri(path: &Path) -> Option<String> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            Some(format!("data:image/png;base64,{}", encoded))
        }
        Err(e) => {
            log::warn!(
                "Branding asset not found at {}: {} (continuing without it)",
                path.display(),
                e
            );
            None
        }
    }
}