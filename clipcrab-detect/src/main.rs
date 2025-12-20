use std::io::Write as _;

use clap::Parser;

#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
enum Detector {
    /// match-result-qr
    MatchResultQR,
    /// seasson2025-decode
    Season2025Decode,
}

#[derive(clap::Parser)]
struct Cli {
    detector: Detector,
    fname: String,
}

fn main() {
    let cli = Cli::parse();
    tracing_subscriber::fmt::init();
    let frame = opencv::imgcodecs::imread(
        &cli.fname,
        opencv::imgcodecs::IMREAD_COLOR_RGB
    ).unwrap();

    let detection = match cli.detector {
        Detector::MatchResultQR => {
            let detection = clipcrab_detect::qr::detect_qr(&frame);
            write!(std::io::stdout(), "{}", serde_json::to_string_pretty(&detection).unwrap()).unwrap();
            return;
        }
        Detector::Season2025Decode => {
            let detector = clipcrab_detect::seasons::s2025_decode::DecodeDetector::new();
            detector.detect(&frame)
        }
    };

    write!(std::io::stdout(), "{}", serde_json::to_string_pretty(&detection).unwrap()).unwrap();
}
