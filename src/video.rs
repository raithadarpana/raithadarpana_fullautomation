use std::fs;
use std::path::Path;
use std::process::Command;

/// Speed multiplier applied to both the still-image video and (in
/// `generate_video`) the audio track. Kept as a constant so the animated
/// path (`generate_animated_video`) can scale its frame timings by the
/// same factor and stay in sync with the sped-up audio.
const PLAYBACK_SPEED: f64 = 1.2;

/// One frame of the row-reveal animation: a screenshot with `path`
/// stays on screen from `start_secs` to `end_secs` (both measured
/// against the *original*, not-yet-sped-up, audio timeline).
pub struct RevealFrame {
    pub path: std::path::PathBuf,
    pub start_secs: f64,
    pub end_secs: f64,
}

/// Builds a video where each cover-image frame (one per revealed table
/// row) is shown for the portion of the narration that talks about that
/// row, instead of holding a single static image for the whole clip.
/// `frames` must be in display order and cover the whole timeline (i.e.
/// `frames[0].start_secs == 0.0` and consecutive frames share their
/// start/end boundary) -- see `voiceover::row_reveal_offsets`, which is
/// what produces them.
///
/// Implementation note: an earlier version fed the still images straight
/// into ffmpeg's `concat` *demuxer* with per-file `duration` directives,
/// which is a commonly recommended "slideshow from images" recipe -- but
/// in practice, once the concatenated stream is routed through a filter
/// graph for re-encoding (as it is here, for scale/pad/fps), the
/// per-image `duration` metadata is unreliable and can silently collapse
/// to effectively one frame held for the whole output, which is exactly
/// the "just shows the full table the whole time" symptom this was
/// rewritten to fix. This version instead renders each frame into its
/// own short silent .mp4 (via `-loop 1 -t <duration>`, which is a
/// reliable, well-supported ffmpeg feature) and concatenates those
/// *videos* -- concatenating same-codec video files via the concat
/// demuxer is the case it's actually designed for and doesn't have this
/// failure mode -- before muxing in the audio track as a final step.
pub fn generate_animated_video(
    frames: &[RevealFrame],
    audio_path: &Path,
    output_path: &Path,
    is_portrait: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if frames.is_empty() {
        return Err("generate_animated_video called with no frames".into());
    }
    for frame in frames {
        if !frame.path.exists() {
            return Err(format!("Frame image not found at: {}", frame.path.display()).into());
        }
    }

    let (width, height) = if is_portrait { (1080, 1920) } else { (1920, 1080) };
    let ffmpeg_path = crate::ffdeps::ffmpeg_path();
    let work_dir = output_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let segment_stem = output_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "segment".to_string());

    let vf = format!(
        "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,setsar=1",
        width, height, width, height
    );

    // Pass 1: one silent .mp4 per frame, held for its (speed-adjusted)
    // duration. Each uses identical codec/pixel format/resolution so the
    // concat step below can just as easily stream-copy them together.
    let mut segment_paths = Vec::with_capacity(frames.len());
    for (i, frame) in frames.iter().enumerate() {
        let duration = ((frame.end_secs - frame.start_secs) / PLAYBACK_SPEED).max(0.05);
        let segment_path = work_dir.join(format!("{}.seg{:02}.mp4", segment_stem, i));

        let output = Command::new(&ffmpeg_path)
            .arg("-y")
            .arg("-loop")
            .arg("1")
            .arg("-t")
            .arg(format!("{:.6}", duration))
            .arg("-i")
            .arg(&frame.path)
            .arg("-vf")
            .arg(&vf)
            .arg("-r")
            .arg("25")
            .arg("-c:v")
            .arg("libx264")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-an")
            .arg(&segment_path)
            .output()?;

        if !output.status.success() {
            let _ = fs::remove_file(&segment_path);
            for p in &segment_paths {
                let _ = fs::remove_file(p);
            }
            return Err(format!(
                "ffmpeg failed rendering reveal frame {} ({}): {}",
                i,
                frame.path.display(),
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        segment_paths.push(segment_path);
    }

    // Pass 2: concatenate the (now genuinely same-codec) video segments.
    // This is the concat demuxer's actual well-supported use case, so a
    // plain `-c copy` here is fast and reliable.
    let concat_list_path = work_dir.join(format!("{}.segments.txt", segment_stem));
    let mut list = String::new();
    for segment_path in &segment_paths {
        let path_str = segment_path.to_string_lossy().replace('\\', "/");
        list.push_str(&format!("file '{}'\n", path_str));
    }
    fs::write(&concat_list_path, list)?;

    let silent_video_path = work_dir.join(format!("{}.silent.mp4", segment_stem));
    let concat_output = Command::new(&ffmpeg_path)
        .arg("-y")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(&concat_list_path)
        .arg("-c")
        .arg("copy")
        .arg(&silent_video_path)
        .output()?;

    let cleanup = |extra: &[&std::path::Path]| {
        for p in &segment_paths {
            let _ = fs::remove_file(p);
        }
        let _ = fs::remove_file(&concat_list_path);
        for p in extra {
            let _ = fs::remove_file(p);
        }
    };

    if !concat_output.status.success() {
        cleanup(&[&silent_video_path]);
        return Err(format!(
            "ffmpeg failed concatenating reveal-frame segments: {}",
            String::from_utf8_lossy(&concat_output.stderr)
        )
        .into());
    }

    // Pass 3: mux in the narration audio, sped up to match the frame
    // timings above (which were already divided by PLAYBACK_SPEED).
    let mux_output = Command::new(&ffmpeg_path)
        .arg("-y")
        .arg("-i")
        .arg(&silent_video_path)
        .arg("-i")
        .arg(audio_path)
        .arg("-filter:a")
        .arg(format!("atempo={}", PLAYBACK_SPEED))
        .arg("-c:v")
        .arg("copy")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("192k")
        .arg("-shortest")
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path)
        .output()?;

    cleanup(&[&silent_video_path]);

    if !mux_output.status.success() {
        return Err(format!(
            "ffmpeg failed muxing audio into the animated video: {}",
            String::from_utf8_lossy(&mux_output.stderr)
        )
        .into());
    }

    Ok(())
}

pub fn generate_video(
    png_path: &Path,
    audio_path: &Path,
    output_path: &Path,
    is_portrait: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    
    if !png_path.exists() {
        return Err(format!("PNG image not found at: {}", png_path.display()).into());
    }

    let (width, height) = if is_portrait { (1080, 1920) } else { (1920, 1080) };
    let ffmpeg_path = crate::ffdeps::ffmpeg_path();
    let filter_complex = format!(
        "[0:v]scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,setsar=1,setpts=PTS/1.2[v];[1:a]atempo=1.2[a]",
        width, height, width, height
    );

    let output = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-loop")
        .arg("1")
        .arg("-i")
        .arg(&png_path)
        .arg("-i")
        .arg(audio_path)
        .arg("-filter_complex")
        .arg(&filter_complex)
        .arg("-map")
        .arg("[v]")
        .arg("-map")
        .arg("[a]")
        .arg("-c:v")
        .arg("libx264")
        .arg("-tune")
        .arg("stillimage")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("192k")
        .arg("-shortest")
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "ffmpeg video generation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}