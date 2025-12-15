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
use opencv::{core::{self as cvcore, Mat, MatTraitConst}, imgcodecs, imgproc};

use crate::{matchers::{MatchPhaseDetector, TemplateMatcher}, ocr::Ocr, utils::{self, MatchDisplayInfo, Point, Size}};

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
scale_x!(ALLIANCE_SCORING_WIDTH = 490);

// Width of the alliance listing window
scale_x!(ALLIANCE_NUMBER_WIDTH = 142);

// Height of the lip between the top of the scoring display and the actual display
const SCORING_BAR_LIP_HEIGHT: f64 = 30.0 / 180.0;

// Threshold at which a scoring box is considered blue.
const SCORE_BLUE_THRESHOLD: f64 = 0.7;

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

    fn extract_display_data(&self, scoring_display: &Mat) -> MatchDisplayInfo {
        // Left alliance
        let left_alliance = utils::relative_extract_roi(
            scoring_display, 
            None,
            Point::new(ALLIANCE_SCORING_WIDTH, SCORING_BAR_LIP_HEIGHT),
            Size::new(ALLIANCE_NUMBER_WIDTH, 1.0 - SCORING_BAR_LIP_HEIGHT)
        );
        // Right alliance
        let right_alliance = utils::relative_extract_roi(
            scoring_display, 
            None,
            Point::new(1.0 - ALLIANCE_SCORING_WIDTH - ALLIANCE_NUMBER_WIDTH, SCORING_BAR_LIP_HEIGHT),
            Size::new(ALLIANCE_NUMBER_WIDTH, 1.0 - SCORING_BAR_LIP_HEIGHT)
        );
        let left_teams = self.number_ocr
            .extract_text(&left_alliance)
            .split('\n')
            .map(|f| f.parse::<u64>().unwrap_or(0))
            .collect::<Vec<u64>>();
        let right_teams = self.number_ocr
            .extract_text(&right_alliance)
            .split('\n')
            .map(|f| f.parse::<u64>().unwrap_or(0))
            .collect::<Vec<u64>>();

        // To determine red-blue switch, we need to determine whether blue is flipped to the other side or not.
        // We do this by determining how much blue there is in the left total score box, which is usually red.
        let scoring_box = utils::relative_extract_roi(
            scoring_display, 
            None, 
            Point::new(ALLIANCE_SCORING_WIDTH + ALLIANCE_NUMBER_WIDTH, 0.0), 
            //Point::new(0.5 + TIMER_WIDTH / 2.0, 0.0),
            Size::new(0.5 - TIMER_WIDTH / 2.0 - ALLIANCE_SCORING_WIDTH - ALLIANCE_NUMBER_WIDTH, 1.0)
        );
        utils::imwrite("target/test.png", &scoring_box);

        let hsv = utils::cvt_color(&scoring_box, imgproc::COLOR_RGB2HSV);
        let mut thr = Mat::default();
        cvcore::in_range(&hsv, &[98_u8, 0_u8, 0_u8], &[108_u8, 255_u8, 255_u8], &mut thr).unwrap();
        let non_zero = cvcore::count_non_zero(&thr).unwrap() as f64;
        let blue_score = non_zero / (scoring_box.size().unwrap().area() as f64);

        tracing::trace!("Blue score: {blue_score}");

        if blue_score > SCORE_BLUE_THRESHOLD {
            MatchDisplayInfo { red_alliance: right_teams, blue_alliance: left_teams, display_flipped: true }
        } else {
            MatchDisplayInfo { red_alliance: left_teams, blue_alliance: right_teams, display_flipped: false }
        }


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

        let match_time = self.match_time_ocr.extract_text(&roi);
        tracing::trace!("Detected match time: {match_time:?}");
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
        let display_info = self.extract_display_data(&scoring_display);
        tracing::trace!("Display info: {display_info:?}");

        Some(())
    }
}


