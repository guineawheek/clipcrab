use std::{path::PathBuf, time::Duration};

use clap::Parser;

pub mod model;
pub mod worker;

#[derive(clap::Parser)]
struct Cli {
    fname: PathBuf,
    out_dir: PathBuf,
    workers: u64,
    #[arg(short, long)]
    start_ts: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    tracing_subscriber::fmt::init();
    clipcrab_io::init().unwrap();

    let duration_us = clipcrab_io::shell::video_duration_us(&cli.fname);
    let start = cli.start_ts.and_then(|s| clipcrab_io::time::parse_time(&s)).unwrap_or(0);

    let mut proj = worker::OfflineEventProject::new(start, duration_us);

    let (task_send, task_recv) = crossbeam_channel::unbounded();
    let (result_send, result_recv) = crossbeam_channel::unbounded();

    let mut workers = vec![];
    for _ in 0..cli.workers {

        let fname = cli.fname.clone();
        let out_dir = cli.out_dir.clone();
        let tasks = task_recv.clone();
        let results = result_send.clone();
        workers.push(std::thread::spawn(|| {
            worker(fname, out_dir, tasks, results);
        }));
    }

    drop(task_recv);
    drop(result_send);

    loop {

        match proj.next() {
            Some(task) => {
                task_send.send(task).unwrap();
            }
            None => {
                while proj.waiting_on_result() {
                    if let Ok(submission) = result_recv.recv_timeout(Duration::from_millis(1000)) {
                        tracing::debug!("{} -> {:?}", submission.task, submission.result);
                        proj.process_submission(submission);  
                    } else {
                        tracing::info!("Waiting on {} tasks...", proj.in_flight().len());
                    }
                }
            }
        }
    }
    
    //let detector = clipcrab_detect::seasons::s2025_decode::DecodeDetector::new();
    //let frame = opencv::imgcodecs::imread(
    //    &cli.fname,
    //    opencv::imgcodecs::IMREAD_COLOR_RGB
    //).unwrap();

    //detector.detect(&frame);
}


fn worker(
    fname: PathBuf,
    out_dir: PathBuf,
    tasks: crossbeam_channel::Receiver<worker::Task>,
    results: crossbeam_channel::Sender<worker::TaskSubmission>
) {
    let mut seeker = clipcrab_io::seek::FFMpegger::new(&fname).unwrap();
    let display_det = clipcrab_detect::seasons::s2025_decode::DecodeDetector::new();

    while let Ok(task) = tasks.recv() {
        tracing::trace!("Processing {:?}", task);
        let result = match task {
            worker::Task::AnalyzeFrame(ts) => {
                analyze_frame(&mut seeker, &display_det, ts)
            }
            worker::Task::ClipMatch(clip_match_request) => {
                let mut pairs = vec![];
                pairs.push((clip_match_request.match_segment.start, clip_match_request.match_segment.duration()));
                if let Some(result_screen) = clip_match_request.result_segment {
                    pairs.push((result_screen.start, result_screen.duration()));
                }

                clipcrab_io::shell::clip_segments(
                    &fname,
                    &out_dir.join(format!("{}.mkv", clip_match_request.key)),
                    &pairs
                );
                worker::TaskResult::ClipDone
            }
            worker::Task::Done => {
                return;
            }
        };

        results.send(worker::TaskSubmission { task, result, }).unwrap();
    }
}

fn analyze_frame(
    seeker: &mut clipcrab_io::seek::FFMpegger,
    display_det: &dyn clipcrab_detect::Detector,
    ts: i64
) -> worker::TaskResult {
    let frame = match seeker.extract_mat(ts) {
        Ok(frame) => frame,
        Err(e) => {
            return worker::TaskResult::Error(format!("{e}"));
        }
    };
    if let Some(det) = display_det.detect(&frame) {
        return worker::TaskResult::MatchDetection(ts, det);
    }
    if let Some(det) = clipcrab_detect::qr::detect_qr(&frame) {
        return worker::TaskResult::MatchResultQR(ts, det);
    }
    worker::TaskResult::None
}