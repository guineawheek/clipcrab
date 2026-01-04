#![warn(redundant_imports)]
extern crate ffmpeg_next as ffmpeg;

use ffmpeg::{codec, encoder};
use ffmpeg_next::{Packet, format};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

struct VideoTranscoder {
    ost_index: usize,
    decoder: ffmpeg::decoder::Video,
    input_time_base: ffmpeg::Rational,
    output_time_base: ffmpeg::Rational,

    encoder: ffmpeg::encoder::Video,
    timestamp: f64,

    logging_enabled: bool,
    frame_count: usize,
    last_log_frame_count: usize,
    starting_time: Instant,
    last_log_time: Instant,
}

impl VideoTranscoder {
    fn new(
        ist: &ffmpeg::format::stream::Stream,
        octx: &mut ffmpeg::format::context::Output,
        ost_index: usize,
        enable_logging: bool,
    ) -> Result<Self, ffmpeg::Error> {
        let global_header = octx.format().flags().contains(ffmpeg::format::Flags::GLOBAL_HEADER);
        let decoder = ffmpeg::codec::context::Context::from_parameters(ist.parameters())?
            .decoder()
            .video()?;

        let codec = ffmpeg::encoder::find_by_name("libsvtav1");
        let mut ost = octx.add_stream(codec)?;

        let mut encoder =
            ffmpeg::codec::context::Context::new_with_codec(codec.expect("libsvtav1 not supported!!!"))
                .encoder()
                .video()?;
        
        //println!("decoder.height {}", decoder.height());
        //println!("decoder.width {}", decoder.width());
        //println!("decoder.aspect_ratio {}", decoder.aspect_ratio());
        //println!("decoder.format {:?}", decoder.format());
        //println!("decoder.frame_rate {:?}", ist.avg_frame_rate());
        //println!("decoder.time_base {}", ist.time_base());

        ost.set_parameters(&encoder);
        encoder.set_height(decoder.height());
        encoder.set_width(decoder.width());
        encoder.set_aspect_ratio(decoder.aspect_ratio());
        encoder.set_format(decoder.format());
        encoder.set_frame_rate(Some(ist.avg_frame_rate()));
        encoder.set_time_base(ist.time_base());

        if global_header {
            encoder.set_flags(ffmpeg::codec::Flags::GLOBAL_HEADER);
        }

        let mut encoder_opts = ffmpeg::Dictionary::new();
        encoder_opts.set("crf", "23");

        let opened_encoder = encoder
            .open_with(encoder_opts)
            .expect("error opening libsvtav1 with supplied settings");
        ost.set_parameters(&opened_encoder);
        Ok(Self {
            ost_index,
            decoder,
            input_time_base: ist.time_base(),
            output_time_base: ffmpeg::Rational(0, 1),
            encoder: opened_encoder,
            logging_enabled: enable_logging,
            timestamp: 0.0,
            frame_count: 0,
            last_log_frame_count: 0,
            starting_time: Instant::now(),
            last_log_time: Instant::now(),
        })
    }

    fn receive_and_process_decoded_frames(
        &mut self,
        octx: &mut ffmpeg::format::context::Output,
    ) {
        let mut frame = ffmpeg::frame::Video::empty();
        while self.decoder.receive_frame(&mut frame).is_ok() {
            self.frame_count += 1;
            let timestamp = frame.timestamp();
            //self.log_progress(f64::from(
            //    ffmpeg::Rational(timestamp.unwrap_or(0) as i32, 1) * self.input_time_base,
            //));
            self.timestamp = (ffmpeg::Rational(timestamp.unwrap_or(0) as i32, 1) / self.input_time_base).into();

            todo!("coal");
            
            //frame.set_pts(ffmpeg::Rational(self.frame_count));
            frame.set_kind(ffmpeg::picture::Type::None);
            self.encoder.send_frame(&frame).unwrap();
            self.receive_and_process_encoded_packets(octx);
        }
    }

    fn receive_and_process_encoded_packets(
        &mut self,
        octx: &mut ffmpeg::format::context::Output,
    ) {
        let mut encoded = ffmpeg::Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(self.ost_index);
            encoded.rescale_ts(self.input_time_base, self.output_time_base);
            encoded.write_interleaved(octx).unwrap();
        }
    }
}

impl Transcoder for VideoTranscoder {
    fn output_index(&self) -> usize {
        self.ost_index
    }
    fn set_output_time_base(&mut self, tb: ffmpeg::Rational) {
        self.output_time_base = tb;
    }

    fn handle_packet(&mut self, packet: Packet, octx: &mut format::context::Output) -> Option<f64> {
        self.decoder.send_packet(&packet).unwrap();
        self.receive_and_process_decoded_frames(octx);
        Some(self.timestamp)
    }
    fn finalize(&mut self, octx: &mut format::context::Output) {
        self.decoder.send_eof().unwrap();
        self.receive_and_process_decoded_frames(octx);
        self.encoder.send_eof().unwrap();
        self.receive_and_process_encoded_packets(octx);
    }
}

pub struct StreamCopier {
    ost_index: usize,
    input_time_base: ffmpeg::Rational,
    output_time_base: ffmpeg::Rational,
}

impl StreamCopier {
    pub fn new(
        ist: &ffmpeg::format::stream::Stream,
        octx: &mut ffmpeg::format::context::Output,
        ost_index: usize,
    ) -> Self {

        let mut ost = octx.add_stream(encoder::find(codec::Id::None)).unwrap();
        ost.set_parameters(ist.parameters());
        // We need to set codec_tag to 0 lest we run into incompatible codec tag
        // issues when muxing into a different container format. Unfortunately
        // there's no high level API to do this (yet).
        unsafe {
            (*ost.parameters().as_mut_ptr()).codec_tag = 0;
        }


        Self {
            ost_index,
            input_time_base: ist.time_base(),
            output_time_base: ffmpeg::Rational(0, 1),
        }
    }
}

impl Transcoder for StreamCopier {
    fn output_index(&self) -> usize {
        self.ost_index
    }
    fn set_output_time_base(&mut self, tb: ffmpeg_next::Rational) {
        self.output_time_base = tb;
    }

    fn handle_packet(&mut self, mut packet: Packet, octx: &mut format::context::Output) -> Option<f64> {
        // Do stream copy on non-video streams.
        packet.rescale_ts(self.input_time_base, self.output_time_base);
        packet.set_position(-1);
        packet.set_stream(self.ost_index as _);
        packet.write_interleaved(octx).unwrap();
        None
    }

    fn finalize(&mut self, _octx: &mut format::context::Output) {}
}

pub trait Transcoder {
    fn output_index(&self) -> usize;
    fn set_output_time_base(&mut self, tb: ffmpeg::Rational);
    fn handle_packet(&mut self, packet: Packet, octx: &mut format::context::Output) -> Option<f64>;
    fn finalize(&mut self, octx: &mut format::context::Output);
}


pub fn clip_test(
    input_file: &Path,
    output_file: &Path,
    //pairs: &[(i64, i64)],
    start: i64,
    end: i64,
) {

    ffmpeg::init().unwrap();

    let mut ictx = ffmpeg::format::input(&input_file).unwrap();
    let mut octx = ffmpeg::format::output(&output_file).unwrap();

    // Seek to start
    ictx.seek(start, ..start).unwrap();

    //let mut stream_mapping = vec![None; ictx.nb_streams() as _];
    //let mut ist_time_bases = vec![ffmpeg::Rational(0, 1); ictx.nb_streams() as _];
    //let mut ost_time_bases = vec![ffmpeg::Rational(0, 1); ictx.nb_streams() as _];
    let mut ost_index = 0;
    let mut transcoders: BTreeMap<usize, Box<dyn Transcoder>> = BTreeMap::new();

    for (ist_index, ist) in ictx.streams().enumerate() {
        let ist_medium = ist.parameters().medium();
        match ist_medium {
            //ffmpeg_next::media::Type::Audio => {
            //}
            ffmpeg_next::media::Type::Video => {
                transcoders.insert(ist_index, Box::new(VideoTranscoder::new(
                    &ist, &mut octx, ost_index, false
                ).unwrap()));
            }
            ffmpeg_next::media::Type::Subtitle => {
                transcoders.insert(ist_index, Box::new(StreamCopier::new(
                    &ist, &mut octx, ost_index
                )));
            }
            _ => {
                continue;
            }
        }
        //stream_mapping[ist_index] = Some(ost_index);
        //ist_time_bases[ist_index] = ist.time_base();
        ost_index += 1;
    }


    octx.set_metadata(ictx.metadata().to_owned());
    format::context::output::dump(&octx, 0, Some(&output_file.as_os_str().to_string_lossy()));
    octx.write_header().unwrap();

    for (ost_index, ost) in octx.streams().enumerate() {
        for transcoder in transcoders.values_mut() {
            if transcoder.output_index() == ost_index {
                transcoder.set_output_time_base(ost.time_base());
            }
        }
    }

    let bar = indicatif::ProgressBar::new(end as u64);
    bar.set_position(start as u64);
    for (stream, packet) in ictx.packets() {
        let ist_index = stream.index();
        if let Some(transcoder) = transcoders.get_mut(&ist_index) {
            if let Some(ts) = transcoder.handle_packet(packet, &mut octx) {
                let ts = ts as i64 * 1_000_000;
                if ts >= end {
                    break;
                }
                bar.set_position(ts as u64);
            }
        }
    }

    // Flush encoders and decoders.
    for transcoder in transcoders.values_mut() {
        transcoder.finalize(&mut octx);
    }

    octx.write_trailer().unwrap();
}
