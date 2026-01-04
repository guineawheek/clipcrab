use std::{io::Write as _, time::Instant};

use clap::Parser;

#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
enum Detector {
    /// match-result-qr
    MatchResultQR,
    /// seasson2025-decode
    Season2025Decode,
}
#[derive(Debug, Clone, PartialEq, Eq, clap::Subcommand)]
enum FileInput {
    Image {
        fname: String,
    },
    Frame {
        fname: String,
        start: String,
    }
}

#[derive(clap::Parser)]
struct Cli {
    detector: Detector,
    #[command(subcommand)]
    input: FileInput,
}

fn main() {
    let cli = Cli::parse();
    tracing_subscriber::fmt::init();
    let start = Instant::now();
    let frame = match cli.input {
        FileInput::Image { fname } => {
            opencv::imgcodecs::imread(
                &fname,
                opencv::imgcodecs::IMREAD_COLOR_RGB
            ).unwrap()
        }
        FileInput::Frame { fname, start } => {
            clipcrab_io::init().unwrap();
            clipcrab_io::seek::FFMpegger::new(fname.as_ref()).unwrap().extract_mat(clipcrab_io::time::parse_time(&start).unwrap()).unwrap()
        }
    };

    tracing::trace!("Load image: {:.3} ms", (Instant::now() - start).as_secs_f64() * 1000.0);

    let start = Instant::now();

    let detection = match cli.detector {
        Detector::MatchResultQR => {
            let detection = clipcrab_detect::qr::detect_qr(&frame);
    
            tracing::trace!("Process image total: {:.3} ms", (Instant::now() - start).as_secs_f64() * 1000.0);
            write!(std::io::stdout(), "{}", serde_json::to_string_pretty(&detection).unwrap()).unwrap();
            return;
        }
        Detector::Season2025Decode => {
            let detector = clipcrab_detect::seasons::s2025_decode::DecodeDetector::new();
            detector.detect(&frame)
        }
    };

    tracing::trace!("Process image total: {:.3} ms", (Instant::now() - start).as_secs_f64() * 1000.0);

    write!(std::io::stdout(), "{}", serde_json::to_string_pretty(&detection).unwrap()).unwrap();
}
