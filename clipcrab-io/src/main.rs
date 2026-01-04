
const AUS: &str = "/disk/guinea/first/2026/FIRST Tech Challenge Australian National Championship Day 1 [SXgSh_yhRdE].webm";
pub fn main() {
    //clipcrab_io::seek::seek_test(AUS).unwrap();
    clipcrab_io::clip::clip_test(
        AUS.as_ref(),
        "/tmp/clip_test.mkv".as_ref(),
        3254 * 1_000_000, 
        (3254 + 167) * 1_000_000,
    );

    //clipcrab_ingest::ingest(
    //    "/disk/guinea/first/2026/FIRST Tech Challenge Australian National Championship Day 1 [SXgSh_yhRdE].webm",
    //    "../frames"
    //).unwrap();
}