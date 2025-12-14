use opencv::{imgproc, core as cvcore, prelude::*};
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
    /// Size of screenshot the template was taken from (typically 1920x1080)
    reference_size: Size,
    /// Size that both input and reference will be resized to for comparison (typically 1280x720)
    match_size: Size,
    /// Match template threshold
    threshold: f64,
}

impl TemplateMatcher {
    pub fn new(template_img: Mat, ref_size: Size, match_size: Size, threshold: f64) -> Self {
        let (compare_x_ratio, compare_y_ratio) = (match_size.width() / ref_size.width(), match_size.width() / ref_size.width());

        let template_gray = cvt_color(&template_img, imgproc::COLOR_BGR2GRAY);
        let scaled_template = resize(&template_gray, compare_x_ratio, compare_y_ratio);
        let template_size = template_img.size().unwrap();
        Self {
            template: scaled_template,
            rel_template_size: Size::new(
                template_size.width as f64 / ref_size.width(),
                template_size.height as f64 / ref_size.height(),
            ),
            reference_size: ref_size,
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
        };

        todo!()
    }

    /// Raw runs [`imgproc::match_template`] with the template.
    /// - frame: Mat of a full color frame
    pub fn match_template_raw(&self, frame: &Mat) -> Mat {
        let size: Size = frame.size().unwrap().into();
        let frame_gray = cvt_color(frame, imgproc::COLOR_BGR2GRAY);
        let frame_resize = resize(&frame_gray, self.match_size.width() / size.width(), self.match_size.height() / size.height());
        
        let mut result = Mat::default();
        imgproc::match_template_def(&frame_resize, &self.template, &mut result,  imgproc::TM_CCOEFF_NORMED).unwrap();
        result
    }
}