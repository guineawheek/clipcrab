use clap::Parser;


#[derive(clap::Parser)]
struct Cli {
    fname: String,
}

fn main() {
    let cli = Cli::parse();
    tracing_subscriber::fmt::init();
    let detector = clipcrab_detect::seasons::s2025_decode::DecodeDetector::new();
    let frame = opencv::imgcodecs::imread(
        &cli.fname,
        opencv::imgcodecs::IMREAD_COLOR_RGB
    ).unwrap();

    detector.detect(&frame);
}
