use std::collections::BTreeSet;

use clipcrab_detect::{MatchDetection, MatchKey};

#[derive(Debug, Clone)]
pub struct WithTime<T> {
    pub frame_ts_us: i64,
    pub value: T
}

impl<T> WithTime<T> {
    pub fn new(frame_ts_us: i64, value: T) -> Self {
        Self { frame_ts_us, value }
    }
}

impl<T> core::ops::Deref for WithTime<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Ord for WithTime<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.frame_ts_us.cmp(&other.frame_ts_us)
    }
}

impl<T> PartialOrd for WithTime<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.frame_ts_us.partial_cmp(&other.frame_ts_us)
    }
}

impl<T> PartialEq for WithTime<T> {
    fn eq(&self, other: &Self) -> bool {
        self.frame_ts_us == other.frame_ts_us
    }
}
impl<T> Eq for WithTime<T> {}


#[derive(Debug)]
pub struct Match {
    pub key: MatchKey,
    /// Pre-match detects
    pub before_detects: BTreeSet<WithTime<MatchDetection>>,
    /// During-match detects
    pub during_detects: BTreeSet<WithTime<MatchDetection>>,
    /// After-match detects
    pub after_detects: BTreeSet<WithTime<MatchDetection>>,
    /// Result screen detects
    pub result_screen_detects: BTreeSet<i64>,
    /// Determined match start
    pub start: Option<i64>,
    /// Earliest results screen
    pub result_screen_earliest: Option<i64>,
    /// Latest results screen
    pub result_screen_latest: Option<i64>,
}

impl Match {
    pub fn new(key: MatchKey) -> Self {
        Self {
            key,
            before_detects: BTreeSet::new(),
            during_detects: BTreeSet::new(),
            after_detects: BTreeSet::new(),
            result_screen_detects: BTreeSet::new(),
            start: None,
            result_screen_earliest: None,
            result_screen_latest: None,
        }
    }

    pub fn add_detection(&mut self, detection: WithTime<MatchDetection>) {
        match detection.phase {
            clipcrab_detect::MatchPhase::NotStarted => {
                self.before_detects.insert(detection);
            }
            clipcrab_detect::MatchPhase::Autonomous |
            clipcrab_detect::MatchPhase::Transition |
            clipcrab_detect::MatchPhase::Teleop => {
                self.during_detects.insert(detection);
            }
            clipcrab_detect::MatchPhase::Ended => {
                self.after_detects.insert(detection);
            }
        }
    }

    pub fn add_results_screen(&mut self, time_us: i64) {
        self.result_screen_detects.insert(time_us);
        //self.result_screen_earliest = Some(self.result_screen_earliest.map(|v| v.min(time_us)).unwrap_or(time_us));
        //self.result_screen_latest = Some(self.result_screen_latest.map(|v| v.max(time_us)).unwrap_or(time_us));
    }

    pub fn calc_start(&mut self) {
        let mut est_starts = self.during_detects.iter().map(|det| {
            det.frame_ts_us - 1_000_000 * match det.phase {
                clipcrab_detect::MatchPhase::Autonomous => 150 - det.time,
                clipcrab_detect::MatchPhase::Transition => 38 - det.time,
                clipcrab_detect::MatchPhase::Teleop => 158 - det.time,
                clipcrab_detect::MatchPhase::NotStarted |
                clipcrab_detect::MatchPhase::Ended => panic!("these branches should not be reached!!!"),
            }
        }).collect::<Vec<i64>>();
        est_starts.sort();
        if est_starts.is_empty() {
            tracing::warn!("Match {self:?} has no during-match detects!");
            return;
        }

        if (est_starts.last().unwrap() - est_starts.first().unwrap()).abs() < 10_000_000 {
            // no replay detected, pick the median
            self.start = Some(est_starts[est_starts.len() / 2]);
        } else {
            // we need to do some clustering
            // we group everything into 5-second bins
            let clusters = cluster_times(
                est_starts.iter(),
                |start, cluster| (start - cluster[0]).abs() > 5_000_000
            );
            tracing::warn!("Possible replay detected in `{}` clusters {:?}", self.key, clusters);

            self.start = if clusters.iter().all(|v| v.len() < 5) {
                // if the clusters each have less than 5 detects, pick the one with the biggest median start
                clusters
                    .iter()
                    .map(|c| c[c.len()/2])
                    .max()
            } else {
                // otherwise take groups with >= 5 detects and pick the later one (the replay)
                clusters
                    .iter()
                    .filter_map(|c| (c.len() >= 5).then(|| c[c.len()/2]))
                    .max()
            };
        }
    }

    pub fn calc_result_screen_search_space(&self) -> (Option<i64>, Option<i64>) {
        if self.result_screen_detects.is_empty() {
            // pick the last known match detect (if any) for start, unbounded for end
            (self.during_detects.last().map(|d| d.frame_ts_us), None)
        } else {
            // here we cluster to find segments where each point is less than 5 seconds apart
            let clusters = cluster_times(
                self.result_screen_detects.iter(),
                |value, cluster| {
                    (value - cluster.last().unwrap()).abs() > 5_000_000
                }
            );
            todo!()
        }
    }
}

fn cluster_times<'a>(
    times: impl Iterator<Item = &'a i64>,
    mut sep_criteria: impl FnMut(i64, &Vec<i64>) -> bool
) -> Vec<Vec<i64>> {
    let mut clusters = Vec::new();
    let mut current_cluster: Option<Vec<i64>> = None;
    for value in times {
        let value = *value;
        current_cluster = match current_cluster {
            Some(mut c) => {
                if sep_criteria(value, &c) {
                    clusters.push(c);
                    Some(vec![value; 1])
                } else {
                    c.push(value);
                    Some(c)
                }
            }
            None => Some(vec![value; 1]),
        }
    }
    if let Some(c) = current_cluster {
        clusters.push(c);
    }

    clusters
}