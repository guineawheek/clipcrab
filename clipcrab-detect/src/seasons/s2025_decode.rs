//! Decode matcher
//! 
//! In general, we try to keep things in relative coordinates. That is, in proportions between 0.0..1.0
//! where (0.0, 0.0) is the top-left, and (1.0, 1.0) is the bottom right of the input image.
//! 
//! `scale_x` and `scale_y` have their inputs in 1080p pixel counts, but their actual values are in proportional/relative coordinates.
//! 
//! We use `s2025_decode.png` to locate if this is a match display or not.
//! - It is the full height of the name bar.
//! - It extends right up to the edge between it and the match name box on the left, and 72 pixels away from the right edge.
//! 
use opencv::{core::{self as cvcore, Mat, MatTraitConst}, imgcodecs};

use crate::{matchers::{MatchPhaseDetector, TemplateMatcher}, ocr::Ocr, utils::{self, Point, Size}};

// 1080p-relative coordinates
macro_rules! scale_x {
    ($name:ident = $value:expr) => {
        const $name: f64 = ($value as f64) / 1920.0;
    };
}

// 1080p-relative coordinates
macro_rules! scale_y {
    ($name:ident = $value:expr) => {
        const $name: f64 = ($value as f64) / 1080.0;
    };
}

// Distance from the logo template to the right edge of the match display.
scale_x!(LOGO_DIST_TO_RIGHT_EDGE = 72);

// Height of bar that contains the season logo, match name, and event name.
scale_y!(NAME_BAR_HEIGHT = 75);

// X-position of the match name relative to the left side of the screen.
scale_x!(MATCH_NAME_X = 980);
// Width of match name ROI
scale_x!(MATCH_NAME_WIDTH = 670);

// Height of the scoring display proper.
scale_y!(SCORING_DISPLAY_HEIGHT = 180);

// Width of an alliance-specific scoring display (from edge of screen to the team alliance list)
scale_x!(ALLIANCE_SCORING_WIDTH = 480);

// X-offset from left of scoring display to timer ROI
scale_x!(TIMER_X = 860);
// Width of timer text ROI
scale_x!(TIMER_WIDTH = 200);

// Y-offset from top of scoring display to timer ROI
scale_y!(TIMER_Y = 50);
// Height of timer text ROI
scale_y!(TIMER_HEIGHT = 80);
// Height of timer phase ROI
scale_y!(TIMER_PHASE_HEIGHT = 56);


#[derive(Debug)]
pub struct DecodeDetector {
    logo_detector: TemplateMatcher,
    not_a_preview_detector: TemplateMatcher,
    match_phase_detector: MatchPhaseDetector,
    match_name_ocr: Ocr,
    text_ocr: Ocr,
    number_ocr: Ocr,
    match_time_ocr: Ocr,
}

impl DecodeDetector {
    pub fn new() -> Self {
        // this already reads as grayscale
        let template_img = imgcodecs::imdecode(include_bytes!("../../templates/s2025_decode.png"), imgcodecs::IMREAD_GRAYSCALE).unwrap();
        let blue_score_img = imgcodecs::imdecode(include_bytes!("../../templates/s2025_blue_score.png"), imgcodecs::IMREAD_GRAYSCALE).unwrap();

        Self {
            logo_detector: TemplateMatcher::new(template_img, Size::res_1080p(), Size::new(1280.0, 720.0), 0.7),
            not_a_preview_detector: TemplateMatcher::new(blue_score_img, Size::res_1080p(), Size::new(1280.0, 720.0), 0.5),
            match_phase_detector: MatchPhaseDetector::new(),
            match_name_ocr: Ocr::new(Some("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")),
            text_ocr: Ocr::new(None),
            number_ocr: Ocr::new(Some("0123456789")),
            match_time_ocr: Ocr::new(Some("012345679:")),
        }
    }

    fn extract_display_data(&self, scoring_display: &Mat) {
        // Left side of scoring display
        let left_display = utils::relative_extract_roi(
            scoring_display, 
            None,
            Point::new(0.0, 0.0),
            Size::new(ALLIANCE_SCORING_WIDTH, 1.0)
        );
        // Right side of scoring display
        let left_display = utils::relative_extract_roi(
            scoring_display, 
            None,
            Point::new(1.0, 0.0),
            Size::new(0.0, ALLIANCE_SCORING_WIDTH)
        );
    }

    pub fn detect(&self, frame: &Mat) -> Option<()> {
        // Step 1: find the logo.
        let Some(logo) = self.logo_detector.matches(frame) else {
            tracing::trace!("No match found!");
            return None;
        };
        tracing::trace!("Found logo!");
        let frame_size = frame.size().unwrap();

        // Step 2: extract the scoring display. 
        // The scoring display always appears below the season logo, and does not include the match name.
        let match_display_left = logo.rel_x + logo.rel_size.width() + LOGO_DIST_TO_RIGHT_EDGE - 1.0;
        let match_display_top = logo.rel_y + NAME_BAR_HEIGHT;
        let match_display_tl = Point::new(match_display_left, match_display_top);
        let scoring_display = utils::relative_extract_roi(
            frame,
            None,
            match_display_tl,
            Size::new(1.0, SCORING_DISPLAY_HEIGHT)
        );

        // Step 3: check if the match is a preview match
        if self.not_a_preview_detector.matches(frame).is_none() {
            tracing::trace!("Found scoring display, but this is not a match!");
            return None;
        }

        // Step 4: extract the match name
        let roi = utils::relative_extract_roi(
            frame,
            None,
            Point::new(MATCH_NAME_X, match_display_tl.y - NAME_BAR_HEIGHT + 10.0/1080.0),
            Size::new(MATCH_NAME_WIDTH, NAME_BAR_HEIGHT - 15.0/1080.0)
        );
        let match_name = self.text_ocr.extract_text(&roi);

        tracing::trace!("Detected match name: {match_name:?}");
        if match_name.contains("Example") {
            // skip the example match display
            return None;
        }
        // Step 5: extract the match time
        let roi = utils::relative_extract_roi(
            &scoring_display,
            Some(frame_size),
            Point::new(TIMER_X, TIMER_Y),
            Size::new(TIMER_WIDTH, TIMER_HEIGHT)
        );
        imgcodecs::imwrite_def("target/test.png", &roi).unwrap();

        let match_time = self.match_time_ocr.extract_text(&roi);
        tracing::trace!("Detected match time: {match_time}");
        let match_seconds = utils::match_time_to_seconds(&match_time)?;
        tracing::trace!("Detected match seconds: {match_seconds}");
        // Step 6: determine the phase of the match
        let roi = utils::relative_extract_roi(
            &scoring_display,
            Some(frame_size),
            Point::new(TIMER_X, 0.0),
            Size::new(TIMER_WIDTH, TIMER_PHASE_HEIGHT)
        );
        let phase = self.match_phase_detector.detect_match_phase(&roi, match_seconds)?;
        tracing::trace!("Detected match phase: {phase:?}");

        // Step 7: extract the teams in this match


        Some(())
    }
}


