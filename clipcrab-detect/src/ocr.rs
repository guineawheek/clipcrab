
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
        let size = img.size().unwrap();
        let src = ocrs::ImageSource::from_bytes(
            img.data_bytes().unwrap(),
            (size.width as u32, size.height as u32)
        ).unwrap();
        let input = self.engine.prepare_input(src).unwrap();
        self.engine.get_text(&input).unwrap()
    }
}

impl core::fmt::Debug for Ocr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Ocr").field(&self.allowed_chars).finish()
    }
}