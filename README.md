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

Install ffmpeg (we call this externally because nobody wants to call ffmpeg from the API)
Go to `clipcrab-detect/text_models` and run `download-models.sh` to get the OCR models

Then you can do a `cargo build --release`

## license

just assume it's MIT/Apache2 like everything else in Rust