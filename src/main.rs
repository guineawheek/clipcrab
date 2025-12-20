use std::collections::{HashSet, VecDeque};

use clap::Parser;


#[derive(clap::Parser)]
struct Cli {
    fname: String,
}

fn main() {
    let cli = Cli::parse();
    tracing_subscriber::fmt::init();
    let detector = clipcrab_detect::seasons::s2025_decode::DecodeDetector::new();
    let frame = opencv::imgcodecs::imread(
        &cli.fname,
        opencv::imgcodecs::IMREAD_COLOR_RGB
    ).unwrap();

    detector.detect(&frame);
}

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
enum Task {
    AnalyzeFrame(i64),
    CheckQROnly(i64),
    Done,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
enum ProjectState {
    InitialScan,
    FindResultScreens(i64),
    Done,
}

struct EventProject {
    state: ProjectState,
    duration_us: i64,
    next_tasks: VecDeque<Task>,
    in_flight: HashSet<Task>,
}

impl EventProject {
    pub fn new(duration_us: i64) -> Self {
        Self {
            state: ProjectState::InitialScan,
            duration_us,
            next_tasks: (0..duration_us)
                .step_by(10_000_000)
                .map(Task::AnalyzeFrame)
                .collect(),
            in_flight: HashSet::new(),
        }
    }

    pub fn next(&mut self) -> Option<Task> {
        let state = self.state;
        match state {
            ProjectState::InitialScan => {
                let next = self.next_tasks.pop_front()?;
                self.in_flight.insert(next);
                Some(next)
            }
            ProjectState::FindResultScreens(_) => todo!(),
            ProjectState::Done => todo!(),
        }
    }

    
}