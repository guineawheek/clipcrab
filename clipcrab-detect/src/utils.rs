
use std::ops::Mul;

use opencv::{imgproc, core as cvcore, prelude::*};
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


pub fn cvt_color(src: &Mat, code: i32) -> Mat {
    let mut out = Mat::default();
    imgproc::cvt_color(src, &mut out, code, 0, cvcore::AlgorithmHint::ALGO_HINT_DEFAULT).unwrap();
    out
}

pub fn resize(src: &Mat, fx: f64, fy: f64) -> Mat {
    let mut out = Mat::default();
    imgproc::resize(src, &mut out, cvcore::Size { width: 0, height: 0 }, fx, fy, imgproc::INTER_AREA).unwrap();
    out
}