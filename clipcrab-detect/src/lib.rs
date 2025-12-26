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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, serde::Serialize, serde::Deserialize, Hash)]
pub enum MatchKey {
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

impl core::str::FromStr for MatchKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.trim().split_whitespace().collect::<Vec<&str>>();
        match parts[..] {
            ["Qualification", n, ..] => {
                Ok(Self::Qualification { num: n.parse()? })
            }
            // For whatever baffling reason, ITD prefixes all its matches with "Playoff" but Decode doesn't.
            // Also da Vinci exists.
            [.., "Match", n] => {
                Ok(Self::Playoff { num: n.parse()?, tiebreaker: 1 })
            }
            [.., "Match", n, "Tiebreaker"] => {
                Ok(Self::Playoff { num: n.parse()?, tiebreaker: 2 })
            }
            [.., "Match", n, "Tiebreaker", t] => {
                Ok(Self::Playoff { num: n.parse()?, tiebreaker: t.parse::<u64>()? + 1_u64 })
            }
            _ => {
                anyhow::bail!("Unknown format");
            }
        }
    }
}

impl Ord for MatchKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match (self, other) {
            (MatchKey::Qualification { num }, MatchKey::Qualification { num: other_num }) => num.cmp(other_num),
            (MatchKey::Qualification { .. }, MatchKey::Playoff { .. }) => core::cmp::Ordering::Less,
            (MatchKey::Playoff { .. }, MatchKey::Qualification { .. }) => core::cmp::Ordering::Greater,
            (MatchKey::Playoff { num, tiebreaker }, MatchKey::Playoff { num: other_num, tiebreaker: other_tiebreaker }) => {
                if num == other_num {
                    tiebreaker.cmp(other_tiebreaker)
                } else {
                    num.cmp(other_num)
                }
            }
        }
    }
}

impl core::fmt::Display for MatchKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchKey::Qualification { num } => write!(f, "Qualification {num}"),
            MatchKey::Playoff { num, tiebreaker: 1 } => write!(f, "Playoff Match {num}"),
            MatchKey::Playoff { num, tiebreaker: 2 } => write!(f, "Playoff Match {num} Tiebreaker"),
            MatchKey::Playoff { num, tiebreaker } => write!(f, "Playoff Match {num} Tiebreaker {}", *tiebreaker - 1),
        }
    }
}


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
    pub time: i64,
    /// match phase
    pub phase: MatchPhase,
    /// match display info
    pub display_info: MatchDisplayInfo,
}