//! Detects whether `ffmpeg`/`ffprobe` are available on the system, and,
//! if not, can download platform-appropriate static builds into an
//! app-managed directory so the rest of the pipeline (see `video.rs` and
//! `voiceover.rs`) can use them without requiring the user to install
//! anything system-wide.
//!
//! Resolution order, used by [`ffmpeg_path`]/[`ffprobe_path`]:
//! 1. A previously auto-downloaded binary under `rd_media/bin/`.
//! 2. Whatever `ffmpeg`/`ffprobe` resolves to on the system `PATH`.
//!
//! Download sources:
//! - Windows / Linux: static builds from the `BtbN/FFmpeg-Builds`
//!   GitHub releases ("latest" tag), which include both `ffmpeg` and
//!   `ffprobe` in the same archive.
//! - macOS: static builds from evermeet.cx, which are distributed as
//!   two separate archives (one for `ffmpeg`, one for `ffprobe`).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Directory (under `rd_media/`) where auto-downloaded binaries are
/// placed, kept separate from the media output so it's obvious what's
/// app-managed vs. user content.
const BIN_DIR: &str = "rd_media/bin";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Windows,
    Linux,
    Mac,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    Arm64,
}

/// Detects the current OS. Returns `None` for anything not covered by
/// our downloadable builds (BSDs, etc.) -- callers should fall back to
/// asking the user to install ffmpeg manually in that case.
pub fn detect_os() -> Option<OperatingSystem> {
    if cfg!(target_os = "windows") {
        Some(OperatingSystem::Windows)
    } else if cfg!(target_os = "linux") {
        Some(OperatingSystem::Linux)
    } else if cfg!(target_os = "macos") {
        Some(OperatingSystem::Mac)
    } else {
        None
    }
}

/// Detects the current CPU architecture. Returns `None` for anything
/// not covered by our downloadable builds.
pub fn detect_arch() -> Option<Architecture> {
    if cfg!(target_arch = "x86_64") {
        Some(Architecture::X86_64)
    } else if cfg!(target_arch = "aarch64") {
        Some(Architecture::Arm64)
    } else {
        None
    }
}

fn bin_dir() -> PathBuf {
    PathBuf::from(BIN_DIR)
}

fn managed_ffmpeg_path() -> PathBuf {
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    bin_dir().join(name)
}

fn managed_ffprobe_path() -> PathBuf {
    let name = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" };
    bin_dir().join(name)
}

/// Returns true if a command resolves on the system `PATH` (i.e.
/// running it with a harmless flag doesn't immediately fail to spawn).
fn is_on_path(command: &str) -> bool {
    Command::new(command)
        .arg("-version")
        .output()
        .is_ok()
}

/// Status of the ffmpeg/ffprobe dependency check, surfaced to the CLI
/// prompt and the web UI's confirmation dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependencyStatus {
    pub ffmpeg_available: bool,
    pub ffprobe_available: bool,
}

impl DependencyStatus {
    pub fn all_available(&self) -> bool {
        self.ffmpeg_available && self.ffprobe_available
    }
}

/// Checks whether ffmpeg/ffprobe are usable, either via a previously
/// auto-downloaded copy or via the system `PATH`.
pub fn check_status() -> DependencyStatus {
    DependencyStatus {
        ffmpeg_available: managed_ffmpeg_path().is_file() || is_on_path("ffmpeg"),
        ffprobe_available: managed_ffprobe_path().is_file() || is_on_path("ffprobe"),
    }
}

/// Resolves the path (or bare command name) that should be used to
/// invoke ffmpeg: prefers a previously auto-downloaded copy, falling
/// back to the bare `ffmpeg`/`ffmpeg.exe` command that PATH resolution
/// will handle.
pub fn ffmpeg_path() -> String {
    resolve_binary(managed_ffmpeg_path(), "ffmpeg")
}

/// Resolves the path (or bare command name) that should be used to
/// invoke ffprobe. See [`ffmpeg_path`].
pub fn ffprobe_path() -> String {
    resolve_binary(managed_ffprobe_path(), "ffprobe")
}

fn resolve_binary(managed_path: PathBuf, bare_name: &str) -> String {
    if managed_path.is_file() {
        managed_path.to_string_lossy().to_string()
    } else if cfg!(windows) {
        format!("{}.exe", bare_name)
    } else {
        bare_name.to_string()
    }
}

/// Whether the current OS/architecture combination has a known
/// download source. Used to decide whether "auto-download" can even be
/// offered as an option, vs. only "install manually".
pub fn download_supported() -> bool {
    matches!(
        (detect_os(), detect_arch()),
        (Some(OperatingSystem::Windows), Some(Architecture::X86_64))
            | (Some(OperatingSystem::Linux), Some(Architecture::X86_64))
            | (Some(OperatingSystem::Linux), Some(Architecture::Arm64))
            | (Some(OperatingSystem::Mac), Some(Architecture::X86_64))
            | (Some(OperatingSystem::Mac), Some(Architecture::Arm64))
    )
}

/// Downloads and installs ffmpeg + ffprobe for the current platform
/// into `rd_media/bin/`, reporting progress via `on_progress`.
///
/// Returns an error if the platform/architecture isn't supported, or if
/// the download/extraction fails for any reason.
pub async fn download_ffmpeg(mut on_progress: impl FnMut(&str)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let os = detect_os().ok_or("Unsupported operating system for ffmpeg auto-download")?;
    let arch = detect_arch().ok_or("Unsupported CPU architecture for ffmpeg auto-download")?;

    if !download_supported() {
        return Err(format!(
            "No pre-built ffmpeg is available for this platform/architecture ({:?}/{:?}). Please install ffmpeg and ffprobe manually and ensure they're on your PATH.",
            os, arch
        )
        .into());
    }

    let dest_dir = bin_dir();
    std::fs::create_dir_all(&dest_dir)?;

    match os {
        OperatingSystem::Windows | OperatingSystem::Linux => {
            download_from_btbn(os, arch, &dest_dir, &mut on_progress).await?;
        }
        OperatingSystem::Mac => {
            download_from_evermeet(&dest_dir, &mut on_progress).await?;
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [managed_ffmpeg_path(), managed_ffprobe_path()] {
            if path.is_file() {
                let mut perms = std::fs::metadata(&path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&path, perms)?;
            }
        }
    }

    on_progress("ffmpeg and ffprobe are ready.");
    Ok(())
}

/// Asset naming for `BtbN/FFmpeg-Builds`'s floating "latest" release,
/// which bundles both `ffmpeg` and `ffprobe` binaries in one archive.
fn btbn_asset_name(os: OperatingSystem, arch: Architecture) -> Result<&'static str, String> {
    match (os, arch) {
        (OperatingSystem::Windows, Architecture::X86_64) => Ok("ffmpeg-master-latest-win64-gpl.zip"),
        (OperatingSystem::Linux, Architecture::X86_64) => Ok("ffmpeg-master-latest-linux64-gpl.tar.xz"),
        (OperatingSystem::Linux, Architecture::Arm64) => Ok("ffmpeg-master-latest-linuxarm64-gpl.tar.xz"),
        _ => Err("BtbN/FFmpeg-Builds has no asset for this platform/architecture".to_string()),
    }
}

async fn download_from_btbn(
    os: OperatingSystem,
    arch: Architecture,
    dest_dir: &Path,
    on_progress: &mut impl FnMut(&str),
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let asset_name = btbn_asset_name(os, arch)?;
    let url = format!(
        "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/{}",
        asset_name
    );

    on_progress(&format!("Downloading ffmpeg build ({})...", asset_name));
    let bytes = download_bytes(&url).await?;

    on_progress("Extracting ffmpeg build...");
    let extract_dir = dest_dir.join("_extract_tmp");
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)?;
    }
    std::fs::create_dir_all(&extract_dir)?;

    if asset_name.ends_with(".zip") {
        extract_zip(&bytes, &extract_dir)?;
    } else {
        extract_tar_xz(&bytes, &extract_dir)?;
    }

    // Archives contain a single top-level folder (e.g.
    // "ffmpeg-master-latest-win64-gpl/") with a "bin/" subfolder holding
    // the executables.
    let root = find_single_subdir(&extract_dir)?;
    let bin_subdir = root.join("bin");
    let search_dir = if bin_subdir.is_dir() { bin_subdir } else { root };

    let ffmpeg_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let ffprobe_name = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" };

    copy_named_file(&search_dir, ffmpeg_name, &managed_ffmpeg_path())?;
    copy_named_file(&search_dir, ffprobe_name, &managed_ffprobe_path())?;

    std::fs::remove_dir_all(&extract_dir).ok();
    Ok(())
}

/// evermeet.cx distributes `ffmpeg` and `ffprobe` as two separate flat
/// zip archives (each containing just the single executable), so both
/// are fetched independently.
async fn download_from_evermeet(
    dest_dir: &Path,
    on_progress: &mut impl FnMut(&str),
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    on_progress("Downloading ffmpeg (macOS build)...");
    let ffmpeg_zip = download_bytes("https://evermeet.cx/ffmpeg/getrelease/zip").await?;
    let extract_dir = dest_dir.join("_extract_tmp_ffmpeg");
    std::fs::create_dir_all(&extract_dir)?;
    extract_zip(&ffmpeg_zip, &extract_dir)?;
    copy_named_file(&extract_dir, "ffmpeg", &managed_ffmpeg_path())?;
    std::fs::remove_dir_all(&extract_dir).ok();

    on_progress("Downloading ffprobe (macOS build)...");
    let ffprobe_zip = download_bytes("https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip").await?;
    let extract_dir = dest_dir.join("_extract_tmp_ffprobe");
    std::fs::create_dir_all(&extract_dir)?;
    extract_zip(&ffprobe_zip, &extract_dir)?;
    copy_named_file(&extract_dir, "ffprobe", &managed_ffprobe_path())?;
    std::fs::remove_dir_all(&extract_dir).ok();

    Ok(())
}

async fn download_bytes(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(url).await?.error_for_status()?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

fn extract_zip(bytes: &[u8], dest_dir: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let out_path = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out_file)?;
        }
    }
    Ok(())
}

fn extract_tar_xz(bytes: &[u8], dest_dir: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut decompressed = Vec::new();
    let mut decoder = xz2::read::XzDecoder::new(bytes);
    decoder.read_to_end(&mut decompressed)?;

    let mut archive = tar::Archive::new(std::io::Cursor::new(decompressed));
    archive.unpack(dest_dir)?;
    Ok(())
}

/// Extracted archives contain a single top-level directory; this finds
/// it (falling back to `dest_dir` itself if the archive was flat).
fn find_single_subdir(dest_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    for entry in std::fs::read_dir(dest_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            return Ok(entry.path());
        }
    }
    Ok(dest_dir.to_path_buf())
}

/// Searches `search_dir` recursively (shallowly -- a couple of levels
/// is enough for these archive layouts) for a file named `file_name`
/// and copies it to `dest_path`.
fn copy_named_file(
    search_dir: &Path,
    file_name: &str,
    dest_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fn search(dir: &Path, file_name: &str, depth: u32) -> Option<PathBuf> {
        if depth > 4 {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().map(|n| n == file_name).unwrap_or(false) {
                return Some(path);
            }
        }
        // Second pass for subdirectories, so direct matches are preferred.
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = search(&path, file_name, depth + 1) {
                    return Some(found);
                }
            }
        }
        None
    }

    let found = search(search_dir, file_name, 0)
        .ok_or_else(|| format!("Could not find '{}' in downloaded archive", file_name))?;

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&found, dest_path)?;
    Ok(())
}