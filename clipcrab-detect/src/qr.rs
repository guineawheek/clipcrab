use opencv::{core::{Mat, MatTraitConst}, imgproc};

use crate::utils::{self, Point, Size};

pub fn detect_qr(mat: &Mat) -> Option<FTCEventsQR> {
    // the coordinates assume that the qr code is in the same spot every time in a full-screen setting. 
    // this is mostly a good assumption.
    let roi = utils::relative_extract_roi(
        mat,
        mat.size().unwrap().into(),
        Point::new(724./1920.,788./1080.),
        Size::new(155./1920., 155./1080.)
    );

    let gray = utils::cvt_color(&roi, imgproc::COLOR_RGB2GRAY);
    // we want to actually upscale the image as rqrr works better if the qr code's pixels aren't 5 pixels across
    // additionally, using nearest neighbor interpolation gives even results
    let mut big_gray = Mat::default();
    imgproc::resize(&gray,
        &mut big_gray,
        opencv::core::Size_::new(400, 400),
        0.,
        0.,
        imgproc::INTER_NEAREST
    ).unwrap();
    // we do an adaptive binary threshold here. 
    let mut bin = Mat::default();
    imgproc::threshold(&big_gray, &mut bin, 0.0, 255.0, imgproc::THRESH_BINARY | imgproc::THRESH_OTSU).unwrap();

    //utils::imwrite("../target/thresh.png", &bin);
    let mut img = rqrr::PreparedImage::prepare_from_bitmap(
         400,
         400,
        |x, y| {
            *bin.at_2d::<u8>(y as i32, x as i32).unwrap() == 0
        }
    );
    for grid in img.detect_grids() {
        match grid.decode() {
            Ok((_, value)) => {
                tracing::trace!("Found QR code: {value}");
                match FTCEventsQR::new(&value) {
                    Ok(result) => return Some(result),
                    Err(e) => {
                        tracing::warn!("Could not parse QR code for URL {value}: {e}");
                    }
                }

            }
            Err(e) => {
                tracing::warn!("Found undecodable QR code: {e}");
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MatchType {
    Qualification {
        /// Match number
        num: u64
    },
    Playoff {
        /// Match number
        num: u64,
        /// Tiebreaker count. Tiebreaker matches are those with a count greater than 1.
        tiebreaker: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FTCEventsQR {
    pub event_code: String,
    pub match_type: MatchType,
}

impl FTCEventsQR {
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let url = url::Url::parse(url)?;
        let host = url.host_str().ok_or(anyhow::anyhow!("No host string!"))?;
        if host != "ftc.events" {
            anyhow::bail!("Not a valid host: {host}");
        }
        let path = url.path_segments().ok_or(anyhow::anyhow!("No path segments!"))?.collect::<Vec<&str>>();
        let (event_code, match_type) = match path[..] {
            [event_code, "qualifications", num, ..] => {
                (event_code.to_string(), MatchType::Qualification { num: num.parse()? })
            }
            [event_code, "playoffs", num, tiebreaker, ..] => {
                (event_code.to_string(), MatchType::Playoff { num: num.parse()?, tiebreaker: tiebreaker.parse()? })
            }
            _ => {
                tracing::warn!("Unparsable FTC-Events URL `{url}`, possibly a bug!!!");
                anyhow::bail!("Unparseable FTC-Events url");
            }
        };

        Ok(Self {
            event_code,
            match_type,
        })
    }
}