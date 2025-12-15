use opencv::{core as cvcore, imgcodecs, imgproc, prelude::*};
use crate::utils::*;

/// Template match. All values are scaled 0.0..1.0 input image lengths as to be resolution agnostic.

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TemplateMatch {
    pub rel_x: f64,
    pub rel_y: f64,
    pub rel_size: Size,
}

#[derive(Debug)]
pub struct TemplateMatcher {
    /// template to match to
    template: Mat,
    /// Relative size (0..1.0) of the template.
    rel_template_size: Size,
    /// Size that both input and reference will be resized to for comparison (typically 1280x720)
    match_size: Size,
    /// Match template threshold
    threshold: f64,
}

impl TemplateMatcher {
    pub fn new(template_gray: Mat, ref_size: Size, match_size: Size, threshold: f64) -> Self {
        let (compare_x_ratio, compare_y_ratio) = (match_size.width() / ref_size.width(), match_size.width() / ref_size.width());
        let scaled_template = resize(&template_gray, compare_x_ratio, compare_y_ratio);
        let template_size = template_gray.size().unwrap();
        Self {
            template: scaled_template,
            rel_template_size: Size::new(
                template_size.width as f64 / ref_size.width(),
                template_size.height as f64 / ref_size.height(),
            ),
            match_size,
            threshold,
        }
    }

    /// Checks if a frame matches the template per the threshold.
    pub fn matches(&self, frame: &Mat) -> Option<TemplateMatch> {
        let template_mat = self.match_template_raw(frame);
        let mut max_val = 0_f64;
        let mut max_loc = cvcore::Point::new(-1, -1);
        cvcore::min_max_loc(&template_mat, None, Some(&mut max_val), None, Some(&mut max_loc), &Mat::default()).unwrap();
        if max_val >= self.threshold {
            Some(TemplateMatch {
                rel_x: max_loc.x as f64 / self.match_size.width(),
                rel_y: max_loc.y as f64 / self.match_size.height(),
                rel_size: self.rel_template_size,
            })
        } else {
            None
        }
    }

    /// Raw runs [`imgproc::match_template`] with the template.
    /// - frame: Mat of a full color frame
    pub fn match_template_raw(&self, frame: &Mat) -> Mat {
        let size: Size = frame.size().unwrap().into();
        let frame_gray = cvt_color(frame, imgproc::COLOR_RGB2GRAY);
        let frame_resize = resize(&frame_gray, self.match_size.width() / size.width(), self.match_size.height() / size.height());
        
        let mut result = Mat::default();
        imgproc::match_template_def(&frame_resize, &self.template, &mut result,  imgproc::TM_CCOEFF_NORMED).unwrap();
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchPhase {
    /// Match not started yet.
    NotStarted,
    /// Autonomous period
    Autonomous,
    /// Auto-teleop transition
    Transition,
    /// Teleoperated period
    Teleop,
    /// Match ended.
    Ended,
}

/// ITD onwards have similar sprites used for match phase signalling.
#[derive(Debug)]
pub struct MatchPhaseDetector {
    autonomous_detector: TemplateMatcher,
    transition_detector: TemplateMatcher,
}

impl MatchPhaseDetector {
    pub fn new() -> Self {
        let autonomous = imgcodecs::imdecode(include_bytes!("../templates/autonomous.png"), imgcodecs::IMREAD_GRAYSCALE).unwrap();
        let transition = imgcodecs::imdecode(include_bytes!("../templates/transition.png"), imgcodecs::IMREAD_GRAYSCALE).unwrap();
        Self {
            autonomous_detector: TemplateMatcher::new(autonomous, Size::res_1080p(), Size::res_1080p(), 0.6),
            transition_detector: TemplateMatcher::new(transition, Size::res_1080p(), Size::res_1080p(), 0.6),
        }
    }

    /// Detect match phase.
    /// 
    /// The logic here tries to avoid using CV if possible.
    /// 
    /// `roi` - ROI where match phase symbols get displayed
    /// `timestamp` - Detected timestamp, in seconds. E.g. 2:15 gets turned into 120 + 15 = 135
    pub fn detect_match_phase(&self, roi: &Mat, timestamp: u64) -> Option<MatchPhase> {
        Some(match timestamp {
            151.. => {
                // Timestamp is above 2 minutes 30 seconds (invalid)
                return None;
            }
            150 => {
                // Match timer still shows 2:30
                if self.autonomous_detector.matches(roi).is_some() {
                    MatchPhase::Autonomous
                } else {
                    MatchPhase::NotStarted
                }
            }
            121..150 => {
                // Match timer is between 2:30 and 2:01 inclusive.
                MatchPhase::Autonomous
            }
            1..=8 => {
                // Possibly the 8-second transition period, need to check explicitly.
                if self.transition_detector.matches(roi).is_some() {
                    MatchPhase::Transition
                } else {
                    MatchPhase::Teleop
                }
            }
            0 => {
                // Zero always displays as end of match.
                MatchPhase::Ended
            }
            _ => {
                // Anything else from 0:09..2:00 is likely teleop
                MatchPhase::Teleop
            }
        })
    }
}
