//! Samples every second of an input video.
//! 

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
    cmd.args(["-y", "-hide_banner", "-loglevel", "warning"]);
    cmd 
}

pub fn ingest(fname: impl AsRef<Path>, output: impl AsRef<Path>) -> Result<(), anyhow::Error> {
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

    for i in 0..duration {
        ffmpeg_incantation_of_god.arg("-ss");
        ffmpeg_incantation_of_god.arg(format!("{i}"));
        ffmpeg_incantation_of_god.arg("-i");
        ffmpeg_incantation_of_god.arg(fname.as_ref());
        ffmpeg_incantation_of_god.args(["-frames:v", "1", "-map"]);
        ffmpeg_incantation_of_god.arg(format!("{video_in}:v:0"));
        ffmpeg_incantation_of_god.arg(output.as_ref().join(format!("frame_{i:06}.bmp")));
        video_in += 1;


        if video_in >= 300 {
            println!("Decoding frames {}..{}", i-300+1, i);
            ffmpeg_incantation_of_god.spawn()?.wait()?;
            ffmpeg_incantation_of_god = new_ffmpeg();
            video_in = 0;
        }
    }

    Ok(())
}