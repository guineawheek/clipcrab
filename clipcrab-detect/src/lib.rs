//! Detectors
//! 
//! ## Some conventions
//! - Always load images as RGB 3-channel U8 Mats (yes, OpenCV typically does BGR, but we think that's lame and it makes OCR loads more annoying)
//! 
pub mod matchers;
pub mod utils;
pub mod seasons;
pub mod ocr;