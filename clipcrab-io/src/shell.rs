
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct DurationJson {
    #[serde(rename = "format")]
    data: DurationJsonInner
}

#[derive(Debug, Clone, Deserialize)]
struct DurationJsonInner {
    duration: String,
}

fn new_ffmpeg() -> Command {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-hide_banner", "-loglevel", "error"]);
    cmd 
}

pub fn video_duration_us(fname: &Path) -> i64 {
    let ffprobe_output = Command::new("ffprobe")
        .args(["-v", "quiet", "-print_format", "json=c=1", "-show_entries", "format=duration"])
        .arg(fname)
        .output()
        .unwrap()
        .stdout;

    let duration = serde_json::from_slice::<DurationJson>(&ffprobe_output).unwrap().data.duration.parse::<f64>().unwrap() as i64;
    duration * 1_000_000

}

/// Splits a video file into frames in 15 second increments.
pub fn ingest(fname: impl AsRef<Path>, output: impl AsRef<Path>) -> Result<(), anyhow::Error> {
    const STEP: usize = 15;
    const MAX_SLICES: u64 = 300;
    let ffprobe_output = Command::new("ffprobe")
        .args(["-v", "quiet", "-print_format", "json=c=1", "-show_entries", "format=duration"])
        .arg(fname.as_ref())
        .output()?
        .stdout;

    let duration = serde_json::from_slice::<DurationJson>(&ffprobe_output)?.data.duration.parse::<f64>()? as u64;
    println!("{duration:?}", );

    std::fs::create_dir_all(output.as_ref())?;

    let mut ffmpeg_incantation_of_god = new_ffmpeg();
    let mut video_in = 0_u64;

    for i in (0..duration).step_by(15) {
        ffmpeg_incantation_of_god.arg("-ss");
        ffmpeg_incantation_of_god.arg(format!("{i}"));
        ffmpeg_incantation_of_god.arg("-i");
        ffmpeg_incantation_of_god.arg(fname.as_ref());
        ffmpeg_incantation_of_god.args(["-frames:v", "1", "-map"]);
        ffmpeg_incantation_of_god.arg(format!("{video_in}:v:0"));
        ffmpeg_incantation_of_god.arg(output.as_ref().join(format!("frame_{i:06}.bmp")));
        video_in += 1;

        if video_in >= MAX_SLICES || (i + STEP as u64) >= duration {
            println!("Extracting {video_in} frames from {i}...");
            ffmpeg_incantation_of_god.spawn()?.wait()?;
            ffmpeg_incantation_of_god = new_ffmpeg();
            video_in = 0;
        }
    }

    Ok(())
}

pub fn clip_segments(
    input_file: &Path,
    output_file: &Path,
    pairs: &[(i64, i64)], // start, duration
) {
    let mut ffmpeg_incantation_of_god = new_ffmpeg();

    for (start, duration) in pairs {

        ffmpeg_incantation_of_god.arg("-ss");
        ffmpeg_incantation_of_god.arg(format!("{}", *start as f64 / 1_000_000.0));
        ffmpeg_incantation_of_god.arg("-t");
        ffmpeg_incantation_of_god.arg(format!("{}", *duration as f64 / 1_000_000.0));
        ffmpeg_incantation_of_god.arg("-i");
        ffmpeg_incantation_of_god.arg(input_file);
    }

    ffmpeg_incantation_of_god.arg("-filter_complex");
    let filter: String = (0..pairs.len())
        .map(|i| format!("[{i}]"))
        .chain([format!("concat=n={}:v=1:a=1[out];[out]setpts=PTS-STARTPTS", pairs.len())])
        .collect();

    ffmpeg_incantation_of_god.arg(&filter);
    ffmpeg_incantation_of_god.args(["-c:a", "libopus", "-b", "96000", "-c:v", "libsvtav1", "-crf", "23"]);
    ffmpeg_incantation_of_god.arg(output_file);
}