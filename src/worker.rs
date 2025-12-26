use std::collections::{BTreeMap, HashSet, VecDeque};
use clipcrab_detect::{MatchDetection, MatchKey, qr::FTCEventsQR};

use crate::model::{Match, WithTime};

/*
Basic flow:
Ingest video

1. Decimate by 10 seconds, run pipeline in massive parallel. 
2. Get a handful of detections.

For each detection, 
* coalesce the detections
* For each detection, queue another frame to process, based on its timestamp. Winning explanation is RANSAC majority vote.

Once we have these, we go through our detection groups and see if we're missing match end screens.
If we have a screen, then we step around it to find its start.
If we don't have a screen, seek from the end of the match until we find one.

We poll these once/second in the relevant regions until we find good start/end points.

Once we have all of these, we call it a day.


Event struct:
- able to manage what things exist


*/

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Task {
    /// Analyze a frame at the microsecond timestamp.
    AnalyzeFrame(i64),
    ClipMatch(),
    Done,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TaskResult {
    None,
    MatchDetection(MatchDetection),
    MatchResultQR(FTCEventsQR),
}

#[derive(Debug, PartialEq, Clone)]
pub struct TaskSubmission {
    pub task: Task,
    pub time_us: i64,
    pub result: TaskResult,
}


#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
enum ProjectState {
    InitialScan,
    ClipMatches,
    Done,
}

pub struct OfflineEventProject {
    state: ProjectState,
    duration_us: i64,
    next_tasks: VecDeque<Task>,
    in_flight: HashSet<Task>,

    matches: BTreeMap<MatchKey, Match>,

}

impl OfflineEventProject {
    pub fn new(duration_us: i64) -> Self {
        Self {
            state: ProjectState::InitialScan,
            duration_us,
            next_tasks: (0..duration_us)
                // every 1s screw it 
                // efficient? no. but it's gonna take weeks to figure out the logic to do this more efficiently
                // and scaling up compute is easy
                .step_by(1_000_000)
                .map(Task::AnalyzeFrame)
                .collect(),
            in_flight: HashSet::new(),
            matches: BTreeMap::new(),
        }
    }

    /// Pumps the state machine to attempt to produce output.
    pub fn next(&mut self) -> Option<Task> {

        if let Some(next) = self.next_tasks.pop_front() {
            self.in_flight.insert(next);
            return Some(next);
        }
        if !self.in_flight.is_empty() {
            return None;
        }

        let state = self.state;
        match state {
            ProjectState::InitialScan => {
                self.state = ProjectState::ClipMatches;
            }
            ProjectState::ClipMatches => {
                todo!()
            }
            ProjectState::Done => {}
        }
        if let Some(next) = self.next_tasks.pop_front() {
            self.in_flight.insert(next);
            Some(next)
        } else {
            None
        }
    }

    pub fn process_submission(&mut self, submission: TaskSubmission) {
        self.in_flight.remove(&submission.task);
        let state = self.state;
        match state {
            ProjectState::InitialScan => {
                match submission.result {
                    TaskResult::None => {}
                    TaskResult::MatchDetection(match_detection) => {
                        if let Ok(key) = match_detection.name.parse::<MatchKey>() {
                            if !self.matches.contains_key(&key) {
                                self.matches.insert(key, Match::new(key));
                            }
                            if let Some(ent) = self.matches.get_mut(&key) {
                                ent.add_detection(WithTime::new(submission.time_us, match_detection));
                            }
                        }
                    }
                    TaskResult::MatchResultQR(qr) => {
                        let key = qr.key;
                        if !self.matches.contains_key(&key) {
                            self.matches.insert(key, Match::new(key));
                        }
                        if let Some(ent) = self.matches.get_mut(&key) {
                            ent.add_results_screen(submission.time_us);
                        }
                    }
                }
            }
            ProjectState::ClipMatches => todo!(),
            ProjectState::Done => todo!(),
        }
    }

}

pub trait WorkerConnection {
    fn next_job(&mut self) -> anyhow::Result<Task>;
    fn submit(&mut self, submission: TaskSubmission) -> anyhow::Result<()>;
}