//! using ffmpeg cli to externally cut up video into hundreds of frames is expensive, as tearing up and down an entire
//! ffmpeg process with hundreds of command line arguments is expensive and wasteful on compute.
//! 
//! so we just open the file once and seek around in it, which, per the ffmpeg source code, is identical to what we'd get
//! by using the `-ss` flag.
//! 
//! we use a swscale context to ensure the frames are in rgb24; this isn't _strictly_ necessary
//! (and might be better for perf if opencv does it) but the perf impact relative to the ease of use impact is likely negligible.
//! 
//! ffmpeg's base unit of time (defined in [`AV_TIME_BASE`]) is one microsecond.
//! that's what it lists the duration and seek timestep as.
extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use ffmpeg_next::ffi::AV_TIME_BASE;
use std::path::Path;
use std::time::Instant;

/// Seek test
pub fn seek_test(p: impl AsRef<Path>) -> Result<(), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let mut ictx = input(&p)?;
    let duration_us = ictx.duration();

    let mut force_opt = 0_usize;
    let start = Instant::now();

    let mut cnt = 0_u64;

    for sec in (0..duration_us).step_by(AV_TIME_BASE as usize * 15) {
        ictx.seek(sec, ..sec)?;
        let frame = extract_frame(&mut ictx)?;
        let _ = conv_to_mat(&frame).unwrap();
        force_opt = force_opt.wrapping_add(frame.data(0).len());

        if cnt % 60 == 0 {
            println!("seek: {:.06}, ts: {:.06}", (Instant::now() - start).as_secs_f64(), (sec as f64) / AV_TIME_BASE as f64);
        }
        cnt += 1;
    }
    println!("{cnt} frames in {:.06} seconds force_opt: {force_opt}", (Instant::now() - start).as_secs_f64());


    Ok(())
}

pub struct FFMpegger {
    pub ictx: ffmpeg::format::context::Input,
    pub duration_us: i64,
}

impl FFMpegger {
    pub fn new(p: &Path) -> Result<Self, ffmpeg::Error> {
        let ictx = input(p)?;
        let duration_us = ictx.duration();
        Ok(
            Self {
                ictx,
                duration_us,
            }
        )
    }

    pub fn duration_us(&self) -> i64 {
        self.duration_us
    }

    pub fn extract_mat(&mut self, ts: i64) -> Result<opencv::core::Mat, anyhow::Error> {
        self.ictx.seek(ts, ..ts)?;
        let frame = extract_frame(&mut self.ictx)?;
        let mat = conv_to_mat(&frame)?;
        Ok(mat)
    }
}

/// Extracts a frame from the current position in the input file.
pub fn extract_frame(ictx: &mut ffmpeg::format::context::Input) -> Result<Video, ffmpeg::Error> {
    let input = ictx
        .streams()
        .best(Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;
    let video_stream_index = input.index();

    let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;

    // this has the nice side effect of making sure everything is rgb24
    let mut scaler = Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::FAST_BILINEAR,
    )?;

    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;
            let mut decoded = Video::empty();
            decoder.receive_frame(&mut decoded)?;
            let mut rgb_frame = Video::empty();
            scaler.run(&decoded, &mut rgb_frame)?;
            decoder.send_eof()?;

            return Ok(rgb_frame);
        }
    }
    Err(ffmpeg::Error::StreamNotFound)
}

/// Converts a video to an opencv mat
pub fn conv_to_mat(video: &Video) -> Result<opencv::core::Mat, opencv::Error> {
    opencv::core::Mat::new_rows_cols_with_bytes::<opencv::core::Point3_<u8>>(
        video.height() as i32,
        video.width() as i32,
        video.data(0)
    ).map(|r| r.clone_pointee())
}