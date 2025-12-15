use opencv::core::{MatTraitConst, MatTraitConstManual};

fn main() {
    tracing_subscriber::fmt::init();
    let detector = clipcrab_detect::seasons::s2025_decode::DecodeDetector::new();
    let frame = opencv::imgcodecs::imread("frames/frame_001395.bmp", opencv::imgcodecs::IMREAD_COLOR_RGB).unwrap();
    //let img = image::open("frames/frame_001320.bmp").unwrap().into_rgb8();
    //let img_raw = img.as_raw().as_slice();
    //let frame_raw = frame.data_bytes().unwrap();
    //println!("{:?}", frame.size().unwrap());
    //println!("{:02x?}", frame.data_bytes().unwrap().len());
    //println!("eq? {}", img_raw == frame_raw);

    detector.detect(&frame);
}
