//! Crash protection for the schedule.
//!
//! Every change to the schedule is written to `autosave.sws` in the app data
//! folder, with a small `autosave.json` beside it that remembers which file
//! the schedule came from. Saving, opening or starting a new schedule
//! removes the autosave again.
//!
//! So **an autosave only exists while there are unsaved changes**. If one is
//! found at startup, the app did not close normally, and the operator is
//! offered the schedule back.
//!
//! Writing happens on a background thread, so the operator never waits for
//! the disk. When changes arrive faster than they can be written, only the
//! newest one is written.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sw_core::Schedule;
use sw_core::time::{self, Timestamp};
use ts_rs::TS;

use crate::files::write_atomically;

const SCHEDULE_FILE: &str = "autosave.sws";
const INFO_FILE: &str = "autosave.json";

/// How long [`Autosave::discard_and_wait`] waits for the writer.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(3);

/// What is known about an autosaved schedule, shown in the restore prompt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct AutosaveInfo {
    /// When the autosave was written.
    #[ts(type = "number")]
    pub saved_at: Timestamp,
    /// The file the schedule was opened from, if it had been saved before.
    pub original_path: Option<String>,
}

/// A request for the background writer.
enum Job {
    Save {
        schedule: Schedule,
        original_path: Option<PathBuf>,
    },
    Discard,
    /// Reply on the channel once every earlier job is done.
    Flush(Sender<()>),
}

/// Handle to the autosave writer. Cheap to call from any thread.
pub struct Autosave {
    directory: PathBuf,
    jobs: Sender<Job>,
}

impl Autosave {
    /// Starts the background writer, keeping its files in `directory`.
    pub fn start(directory: PathBuf) -> Self {
        let (jobs, queue) = mpsc::channel();
        let worker_directory = directory.clone();
        thread::Builder::new()
            .name("autosave".into())
            .spawn(move || run_worker(&worker_directory, queue))
            .expect("could not start the autosave thread");

        Self { directory, jobs }
    }

    /// Remembers `schedule` in case the app does not close normally.
    pub fn save(&self, schedule: Schedule, original_path: Option<PathBuf>) {
        self.send(Job::Save {
            schedule,
            original_path,
        });
    }

    /// Forgets the autosave, because the schedule was saved or replaced.
    pub fn discard(&self) {
        self.send(Job::Discard);
    }

    /// Forgets the autosave and waits until the files are really gone. Used
    /// just before quitting, when the background thread might otherwise not
    /// get to run.
    pub fn discard_and_wait(&self) {
        let (done, wait) = mpsc::channel();
        self.send(Job::Discard);
        self.send(Job::Flush(done));
        let _ = wait.recv_timeout(FLUSH_TIMEOUT);
    }

    /// The autosave left behind by a previous run, if any. Only meaningful
    /// at startup, before anything new has been autosaved.
    pub fn leftover(&self) -> Option<AutosaveInfo> {
        let json = fs::read(self.directory.join(INFO_FILE)).ok()?;
        serde_json::from_slice(&json).ok()
    }

    /// Reads back the autosaved schedule and the file it came from.
    pub fn load(&self) -> Result<(Schedule, Option<PathBuf>), crate::error::AppError> {
        let bytes = fs::read(self.directory.join(SCHEDULE_FILE))?;
        let schedule = sw_files::read_schedule(&bytes)?;
        let original_path = self
            .leftover()
            .and_then(|info| info.original_path)
            .map(PathBuf::from);
        Ok((schedule, original_path))
    }

    fn send(&self, job: Job) {
        // The worker only stops when the app quits, so a send cannot fail
        // while it matters.
        let _ = self.jobs.send(job);
    }
}

/// The background writer: waits for jobs, skips to the newest save or
/// discard queued, carries it out, then answers any flush requests.
fn run_worker(directory: &Path, queue: Receiver<Job>) {
    while let Ok(first) = queue.recv() {
        let mut newest = None;
        let mut flushes = Vec::new();
        for job in std::iter::once(first).chain(queue.try_iter()) {
            match job {
                Job::Flush(done) => flushes.push(done),
                job => newest = Some(job),
            }
        }

        let result = match newest {
            Some(Job::Save {
                schedule,
                original_path,
            }) => write(directory, &schedule, original_path),
            Some(Job::Discard) => remove(directory).map_err(Into::into),
            Some(Job::Flush(_)) | None => Ok(()),
        };
        if let Err(error) = result {
            eprintln!("autosave failed: {error}");
        }

        for done in flushes {
            let _ = done.send(());
        }
    }
}

fn write(
    directory: &Path,
    schedule: &Schedule,
    original_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let info = AutosaveInfo {
        saved_at: time::now(),
        original_path: original_path.map(|p| p.display().to_string()),
    };
    write_atomically(
        &directory.join(SCHEDULE_FILE),
        &sw_files::write_schedule(schedule)?,
    )?;
    // The info file goes last: it is what marks the autosave as complete.
    write_atomically(&directory.join(INFO_FILE), &serde_json::to_vec(&info)?)?;
    Ok(())
}

fn remove(directory: &Path) -> std::io::Result<()> {
    for name in [INFO_FILE, SCHEDULE_FILE] {
        match fs::remove_file(directory.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
