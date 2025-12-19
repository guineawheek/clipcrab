
use ocrs::{OcrEngine, OcrEngineParams};
use opencv::core::{Mat, MatTraitConst, MatTraitConstManual};

const TEXT_DETECTION: &[u8] = include_bytes!("../text_models/text-detection.rten");
const TEXT_RECOGNITION: &[u8] = include_bytes!("../text_models/text-recognition.rten");

pub struct Ocr {
    engine: OcrEngine,
    allowed_chars: Option<String>,
}

impl Ocr {
    pub fn new(allowed_chars: Option<&str>) -> Self {
        let allowed_chars = allowed_chars.map(|s| s.to_string());
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(rten::Model::load_static_slice(TEXT_DETECTION).unwrap()),
            recognition_model: Some(rten::Model::load_static_slice(TEXT_RECOGNITION).unwrap()),
            debug: false,
            decode_method: ocrs::DecodeMethod::Greedy,
            alphabet: None,
            allowed_chars: allowed_chars.clone(),
        }).unwrap();
        Self { engine, allowed_chars }
    }

    /// Extracts text from an RGBu8 ordered mat.
    /// Lines are separated by newlines.
    pub fn extract_text(&self, img: &Mat) -> String {
        let input = self.rgb2input(img);
        self.engine.get_text(&input).unwrap_or_default()
    }

    pub fn extract_text_debug(&self, img: &Mat) {
        // this doesn't work as reliably as extract_text, don't use
        let size = img.size().unwrap();
        let (size_fx, size_fy) = (size.width as f32, size.height as f32);
        let input = self.rgb2input(img);
        //let word_rects = self.engine.detect_words(&input).unwrap();
        let rect = rten_imageproc::RotatedRect::new(
            rten_imageproc::Point { x: size_fx / 2.0, y: size_fy / 2.0 },
            rten_imageproc::Vec2 { x: 0.0, y: -1.0 },
            size_fx,
            size_fy,
        );
        let word_rects = vec![rect; 1];

        tracing::info!("Word rects: {word_rects:?}");

        let line_rects = self.engine.find_text_lines(&input, &word_rects);
        let text = self
            .engine
            .recognize_text(&input, &line_rects).unwrap()
            .into_iter()
            .filter_map(|line| line.map(|l| l.to_string()))
            .collect::<Vec<_>>()
            .join("\n");

        tracing::info!("Text detected: {text}");
    }

    fn rgb2input(&self, img: &Mat) -> ocrs::OcrInput {
        let size = img.size().unwrap();
        self.engine.prepare_input(
            ocrs::ImageSource::from_bytes(
                img.data_bytes().unwrap(),
                (size.width as u32, size.height as u32)
            ).unwrap()
        ).unwrap()
    }
}

impl core::fmt::Debug for Ocr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Ocr").field(&self.allowed_chars).finish()
    }
}
