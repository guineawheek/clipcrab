//! Detectors
//! 
//! ## Some conventions
//! - Always load images as RGB 3-channel U8 Mats (yes, OpenCV typically does BGR, but we think that's lame and it makes OCR loads more annoying)
//! 

pub mod matchers;
pub mod utils;
pub mod seasons;
pub mod ocr;
pub mod qr;


#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct MatchDisplayInfo {
    /// Red alliance teams
    pub red_alliance: Vec<u64>,
    /// Blue alliance teams
    pub blue_alliance: Vec<u64>,
    /// Whether the display is flipped
    pub display_flipped: bool,
}

/// Final match result struct
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct MatchDetection {
    /// match name, e.g. Qualification X of Y
    pub name: String,
    /// match time
    pub time: u64,
    /// match phase
    pub phase: MatchPhase,
    /// match display info
    pub display_info: MatchDisplayInfo,
}