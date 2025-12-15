
use std::ops::Mul;

use opencv::{core as cvcore, highgui, imgproc, prelude::*};

use crate::matchers::TemplateMatch;
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Size {
    pub x: f64,
    pub y: f64
}

impl Size {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub const fn res_1080p() -> Self {
        Self::new(1920.0, 1080.0)
    }

    pub const fn width(&self) -> f64 {
        self.x
    }

    pub const fn height(&self) -> f64 {
        self.y
    }
}

impl Into<cvcore::Size> for Size {
    fn into(self) -> cvcore::Size {
        cvcore::Size_ { width: self.width() as i32, height: self.height() as i32 }
    }
}

impl From<cvcore::Size> for Size {
    fn from(value: cvcore::Size) -> Self {
        Self::new(value.width as f64, value.height as f64)
    }
}

impl Mul<f64> for Size {
    type Output = Size;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Helper function for [`imgproc::cvt_color`]
pub fn cvt_color(src: &Mat, code: i32) -> Mat {
    let mut out = Mat::default();
    imgproc::cvt_color(src, &mut out, code, 0, cvcore::AlgorithmHint::ALGO_HINT_DEFAULT).unwrap();
    out
}

/// Helper function for [`imgproc::resize`]
pub fn resize(src: &Mat, fx: f64, fy: f64) -> Mat {
    let mut out = Mat::default();
    imgproc::resize(src, &mut out, cvcore::Size { width: 0, height: 0 }, fx, fy, imgproc::INTER_AREA).unwrap();
    out
}

/// Match display location
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MatchDisplayLocation {
    Top,
    Bottom,
}

pub fn match_display_location(logo: &TemplateMatch) -> MatchDisplayLocation {
    if logo.rel_y > 0.5 {
        MatchDisplayLocation::Bottom
    } else {
        MatchDisplayLocation::Top
    }
}

pub fn relative_extract_roi(frame: &Mat, ref_dims: Option<opencv::core::Size>, rel_pos: Point, rel_size: Size) -> Mat {
    let size = ref_dims.unwrap_or_else(|| frame.size().unwrap());
    let fsize_w = size.width as f64;
    let fsize_h = size.height as f64;
    let x = (fsize_w * rel_pos.x).round() as i32;
    let y = (fsize_h * rel_pos.y).round() as i32;
    let w = (fsize_w * rel_size.width()).round() as i32;
    let h = (fsize_h * rel_size.height()).round() as i32;

    RoiPad::calc(size, x, y, w, h).apply(frame)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct RoiPad {
    roi_x: i32,
    roi_y: i32,
    roi_width: i32,
    roi_height: i32,
    pad_left: i32,
    pad_right: i32,
    pad_top: i32,
    pad_bottom: i32,
}

impl RoiPad {
    fn calc(in_size: opencv::core::Size, x: i32, y: i32, w: i32, h: i32) -> Self {

        let roi_x = x.max(0).min(in_size.width - 1);
        let roi_y = y.max(0).min(in_size.height - 1);
        let roi_width = (x + w).max(x).min(in_size.width) - x;
        let roi_height = (y + h).max(y).min(in_size.height) - y;

        let (pad_left, pad_right) = if roi_width < w {
            if x < 0 {
                (-x, (w - roi_width + x).max(0))
            } else {
                // we assume we go offscreen to the right 
                (0, (w - roi_width).max(0))
            }
        } else {
            (0, 0)
        };

        let (pad_top, pad_bottom) = if roi_height < h {
            if y < 0 {
                (-y, (h - roi_height + y).max(0))
            } else {
                // we assume we go offscreen to the right 
                (0, (h - roi_height).max(0))
            }
        } else {
            (0, 0)
        };

        Self {
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            pad_top,
            pad_bottom,
            pad_left,
            pad_right,
        }
    }

    fn apply(&self, frame: &Mat) -> Mat {
        let extract = frame.roi(opencv::core::Rect_::new(
            self.roi_x, 
            self.roi_y,
            self.roi_width,
            self.roi_height
        ))
        .unwrap();
        if (self.pad_left, self.pad_right, self.pad_top, self.pad_bottom) == (0, 0, 0, 0) {
            extract.clone_pointee()
        } else {
            let mut result = Mat::default();
            opencv::core::copy_make_border_def(
                &extract,
                &mut result,
                self.pad_top,
                self.pad_bottom,
                self.pad_left,
                self.pad_right,
                cvcore::BORDER_REPLICATE).unwrap();
            result
        }

    }
}

pub fn match_time_to_seconds(time: &str) -> Option<u64> {
    let time = time.replace("\n", "");
    match time.split(':').collect::<Vec<&str>>().as_slice() {
        [min, sec, ..] => {
            let min = min.parse::<u64>().ok()?;
            let sec = sec.parse::<u64>().ok()?;
            Some(min * 60 + sec)
        }
        _ => None,
    }
}

pub fn display_mat(name: &str, mat: &Mat) {
    highgui::named_window(name, highgui::WINDOW_AUTOSIZE).unwrap();
    highgui::imshow(name, mat).unwrap();
    highgui::wait_key_def().unwrap();
}