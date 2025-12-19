# clipcrab

matchinator/clipfarm CV/OCR backend, written in rust

## why oxidation

* matchinator python code doesn't have type hints and is hard to understand
* matchinator code is really slow and hard to debug because it doesn't separate job assignments from the CV pipelines
* i discovered faster ways to extract frames from a video by using ffmpeg seeking
* actually binding to native libraries lets me shove image data directly into `ocrs` without going through tempfiles. supposedly it's better
* greater maintenance interest from people if its in rust, which i write for a living anyway
* python sucks tspmo

## how build

Install ffmpeg and opencv

`cd` to `clipcrab-detect/text_models` and run `./download-models.sh` to get the OCR models

Then you can do a `cargo build --release` in wherever

## how use

### `clipcrab-detect`

`clipcrab-detect` contains the main match display detector.
It's both a library and a standalone application. 

When run as a standalone application, it spits out JSON on a detect, or `null` otherwise.

```shell
$ cargo run -p clipcrab-detect --release -- season-2025decode frames/frame_001395.bmp
{
  "name": "Qualification 1 of 63",
  "time": 114,
  "phase": "Teleop",
  "display_info": {
    "red_alliance": [
      24089,
      31784
    ],
    "blue_alliance": [
      31243,
      18306
    ],
    "display_flipped": false
  }
}
```

## unwrap usage

this code unwraps pretty liberally. if you panic there it's a bug anyway that needs to get fixed.

## license

just assume it's MIT/Apache2 like everything else in Rust