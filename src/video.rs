use std::path::Path;
use std::process::Command;

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