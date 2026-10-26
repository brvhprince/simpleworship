//! Speech to text in a helper process, so a crash in it cannot take the
//! projector down.
//!
//! Whisper (whisper.cpp) is C++: if it runs out of stack, fails one of its
//! own checks (`GGML_ASSERT`) or reads memory it should not, the process
//! ends there and then, and nothing in Rust can catch it. During a service
//! that must never close SimpleWorship. So Whisper runs in a second copy of
//! the app's own executable, started with [`HELPER_FLAG`] (see the desktop
//! app's `main.rs`), and the two talk over the helper's standard input and
//! output:
//!
//! - The app sends each piece of 16 kHz speech: its number of samples as a
//!   4-byte little-endian integer, then the samples as little-endian `f32`s.
//! - The helper answers each piece with one line: `words <text>` (possibly
//!   empty) or `error <message>`. Once the model has loaded, before any
//!   piece, it sends `ready` (or `error <message>` and stops).
//!
//! What the helper writes to standard error (Rust's message for a panic or
//! a stack overflow, whisper.cpp's for a failed check) goes to the app's
//! log, and so does how it ended, so a crash can be understood afterwards.
//! [`Helper`] is the app's side; [`serve`] is the helper's.

use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, PoisonError};

use crate::Error;
use crate::audio::SAMPLE_RATE;
use crate::transcribe::Transcriber;

/// The command-line flag that starts the executable as the helper:
/// `<app> --listen-helper <model file>`.
pub const HELPER_FLAG: &str = "--listen-helper";

/// Stack for the helper's Whisper thread. whisper.cpp needs far more than
/// the 1 MiB Windows gives a program's threads by default.
const HELPER_STACK_SIZE: usize = 16 * 1024 * 1024;

/// The longest piece of speech accepted, a safety limit against a garbled
/// length (the app sends at most about ten seconds).
const MAX_PIECE_SAMPLES: usize = SAMPLE_RATE as usize * 60;

/// Something that turns 16 kHz speech into words.
pub trait SpeechToText {
    /// The words in a piece of 16 kHz mono speech.
    fn transcribe(&mut self, audio: &[f32]) -> Result<String, Error>;
}

impl SpeechToText for Transcriber {
    fn transcribe(&mut self, audio: &[f32]) -> Result<String, Error> {
        Transcriber::transcribe(self, audio)
    }
}

// ── The helper's side ────────────────────────────────────────────────────

/// Runs the helper: loads the model at `model`, then answers pieces of
/// speech on standard input until it closes. Returns the process's exit
/// code. Whisper runs on a thread with a large stack (see
/// [`HELPER_STACK_SIZE`]).
pub fn serve(model: &Path) -> i32 {
    let model = model.to_owned();
    let worker = std::thread::Builder::new()
        .name("listen-helper".into())
        .stack_size(HELPER_STACK_SIZE)
        .spawn(move || {
            let mut output = io::stdout().lock();
            let mut transcriber = match Transcriber::load(&model) {
                Ok(transcriber) => transcriber,
                Err(error) => {
                    let _ = reply(&mut output, Err(&error));
                    return 1;
                }
            };
            match serve_with(&mut transcriber, io::stdin().lock(), output) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("listen helper: {error}");
                    1
                }
            }
        });
    match worker.map(std::thread::JoinHandle::join) {
        Ok(Ok(code)) => code,
        _ => 1,
    }
}

/// Answers pieces of speech from `input` on `output` with `speech`, after
/// saying it is ready, until `input` closes.
pub fn serve_with(
    speech: &mut impl SpeechToText,
    mut input: impl Read,
    mut output: impl Write,
) -> io::Result<()> {
    writeln!(output, "ready")?;
    output.flush()?;
    while let Some(piece) = read_piece(&mut input)? {
        reply(&mut output, speech.transcribe(&piece).as_deref())?;
    }
    Ok(())
}

/// Writes the answer for one piece (see the [module docs](self)).
fn reply(output: &mut impl Write, result: Result<&str, &Error>) -> io::Result<()> {
    match result {
        Ok(words) => writeln!(output, "words {}", one_line(words))?,
        Err(error) => writeln!(output, "error {}", one_line(&error.to_string()))?,
    }
    output.flush()
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Sends one piece of speech (see the [module docs](self)).
fn write_piece(output: &mut impl Write, audio: &[f32]) -> io::Result<()> {
    let count = u32::try_from(audio.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "piece too long"))?;
    output.write_all(&count.to_le_bytes())?;
    for sample in audio {
        output.write_all(&sample.to_le_bytes())?;
    }
    output.flush()
}

/// Reads one piece of speech, or `None` when the input has closed.
fn read_piece(input: &mut impl Read) -> io::Result<Option<Vec<f32>>> {
    let mut count = [0; 4];
    match input.read_exact(&mut count) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let count = u32::from_le_bytes(count) as usize;
    if count > MAX_PIECE_SAMPLES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("a piece of {count} samples is too long"),
        ));
    }
    let mut bytes = vec![0; count * 4];
    input.read_exact(&mut bytes)?;
    Ok(Some(
        bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect(),
    ))
}

/// One answer from the helper.
#[derive(Debug, PartialEq)]
enum Answer {
    Ready,
    Words(String),
    Failed(String),
}

/// Reads one answer line, or `None` when the helper has gone.
fn read_answer(input: &mut impl BufRead) -> io::Result<Option<Answer>> {
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let line = line.trim_end_matches(['\r', '\n']);
    let (kind, rest) = line.split_once(' ').unwrap_or((line, ""));
    Ok(Some(match kind {
        "ready" => Answer::Ready,
        "words" => Answer::Words(rest.to_owned()),
        "error" => Answer::Failed(rest.to_owned()),
        _ => Answer::Failed(format!("unexpected answer “{line}”")),
    }))
}

// ── The app's side ───────────────────────────────────────────────────────

/// How to start the helper: a program and its arguments.
#[derive(Clone, Debug)]
pub struct HelperCommand {
    /// The executable.
    pub program: PathBuf,
    /// Its arguments, ending with the model file.
    pub args: Vec<OsString>,
}

impl HelperCommand {
    /// This same executable, started as the helper for the model at
    /// `model` (the app's `main` must hand [`HELPER_FLAG`] to [`serve`]).
    pub fn this_executable(model: &Path) -> io::Result<Self> {
        Ok(Self {
            program: std::env::current_exe()?,
            args: vec![HELPER_FLAG.into(), model.as_os_str().to_owned()],
        })
    }
}

/// The helper process currently running, shared so that stopping
/// [`Listener`](crate::Listener) can end it even while a piece is being
/// written down.
pub(crate) type HelperSlot = Arc<Mutex<Option<Child>>>;

/// Ends the helper in `slot`, if there is one, and waits for it.
pub(crate) fn stop_helper(slot: &HelperSlot) {
    let child = slot.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// A running helper, ready for speech (the app's side).
pub struct Helper {
    slot: HelperSlot,
    input: BufWriter<ChildStdin>,
    output: BufReader<ChildStdout>,
    /// The last thing the helper wrote to standard error, for the message
    /// when it stops.
    last_complaint: Arc<Mutex<Option<String>>>,
}

impl Helper {
    /// Starts the helper and waits until its model has loaded (a second or
    /// two). It is kept in `slot`.
    pub(crate) fn start(command: &HelperCommand, slot: HelperSlot) -> Result<Self, Error> {
        let mut child = low_priority(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| Error::Helper(format!("it could not be started: {error}")))?;

        let last_complaint = Arc::new(Mutex::new(None));
        if let Some(stderr) = child.stderr.take() {
            let last = last_complaint.clone();
            let _ = std::thread::Builder::new()
                .name("listen-helper-log".into())
                .spawn(move || forward_complaints(stderr, &last));
        }
        let (Some(input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(Error::Helper("its input and output are missing".into()));
        };
        *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(child);

        let mut helper = Self {
            slot,
            input: BufWriter::new(input),
            output: BufReader::new(output),
            last_complaint,
        };
        match read_answer(&mut helper.output) {
            Ok(Some(Answer::Ready)) => Ok(helper),
            Ok(Some(Answer::Failed(message))) => Err(Error::Model(message)),
            Ok(Some(Answer::Words(_))) => {
                Err(Error::Helper("it answered before it was ready".into()))
            }
            Ok(None) | Err(_) => Err(helper.stopped()),
        }
    }

    /// Why the helper is no longer answering: how it ended, and the last
    /// thing it complained about.
    fn stopped(&mut self) -> Error {
        let child = self
            .slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let ended = match child {
            Some(mut child) => match child.wait() {
                Ok(status) => describe_exit(status),
                Err(error) => error.to_string(),
            },
            // Taken by `stop_helper`: listening was stopped on purpose.
            None => "it was stopped".into(),
        };
        // Give the log thread a moment to read its last words.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let complaint = self
            .last_complaint
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        Error::Helper(match complaint {
            Some(complaint) => format!("{ended} ({complaint})"),
            None => ended,
        })
    }
}

impl SpeechToText for Helper {
    fn transcribe(&mut self, audio: &[f32]) -> Result<String, Error> {
        if write_piece(&mut self.input, audio).is_err() {
            return Err(self.stopped());
        }
        match read_answer(&mut self.output) {
            Ok(Some(Answer::Words(words))) => Ok(words),
            Ok(Some(Answer::Failed(message))) => Err(Error::Transcribe(message)),
            Ok(Some(Answer::Ready)) => Err(Error::Helper("it answered out of turn".into())),
            Ok(None) | Err(_) => Err(self.stopped()),
        }
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        stop_helper(&self.slot);
    }
}

/// The command for `helper`, set to run at a lower priority than the app,
/// so the computer gives the sound and the operator window their time
/// first and Whisper all the rest. On Windows it also has no console
/// window.
fn low_priority(helper: &HelperCommand) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = Command::new(&helper.program);
        command
            .args(&helper.args)
            .creation_flags(BELOW_NORMAL_PRIORITY_CLASS | CREATE_NO_WINDOW);
        command
    }
    #[cfg(unix)]
    {
        // `nice` starts the helper in its own place (same process), at a
        // lower priority. Without it, the helper runs at the normal one.
        const NICE: &str = "/usr/bin/nice";
        let mut command = if Path::new(NICE).exists() {
            let mut command = Command::new(NICE);
            command.args(["-n", "10"]).arg(&helper.program);
            command
        } else {
            Command::new(&helper.program)
        };
        command.args(&helper.args);
        command
    }
}

/// Copies what the helper writes to standard error into the app's log,
/// keeping the last line for [`Helper::stopped`].
fn forward_complaints(stderr: impl Read, last: &Mutex<Option<String>>) {
    for line in BufReader::new(stderr).lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        log::warn!("listen helper: {line}");
        *last.lock().unwrap_or_else(PoisonError::into_inner) = Some(line.to_owned());
    }
}

/// How a helper ended, in words, e.g. "it ran out of stack space".
fn describe_exit(status: ExitStatus) -> String {
    #[cfg(windows)]
    if let Some(code) = status.code() {
        // Windows reports a crash as an NTSTATUS exit code.
        return match code as u32 {
            0xC000_00FD => "it ran out of stack space (0xC00000FD)".into(),
            0xC000_0005 => "it read memory it should not have (0xC0000005)".into(),
            0xC000_0409 => "it stopped itself after a failed check (0xC0000409)".into(),
            0xC000_001D => "it used an instruction this processor lacks (0xC000001D)".into(),
            3 => "it stopped itself after a failed check (abort)".into(),
            other => format!("it ended with code {other:#010X}"),
        };
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return match signal {
                6 => "it stopped itself after a failed check (SIGABRT)".into(),
                11 => "it read memory it should not have (SIGSEGV)".into(),
                4 => "it used an instruction this processor lacks (SIGILL)".into(),
                9 => "it was stopped (SIGKILL)".into(),
                other => format!("it ended with signal {other}"),
            };
        }
    }
    format!("it ended ({status})")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Speech to text that answers with how many samples it heard.
    struct Counting;

    impl SpeechToText for Counting {
        fn transcribe(&mut self, audio: &[f32]) -> Result<String, Error> {
            if audio.is_empty() {
                return Err(Error::Transcribe("nothing\\nto hear".into()));
            }
            Ok(format!("heard {} samples", audio.len()))
        }
    }

    #[test]
    fn pieces_and_answers_go_back_and_forth() {
        let mut sent = Vec::new();
        write_piece(&mut sent, &[0.5; 16_000]).unwrap();
        write_piece(&mut sent, &[]).unwrap();
        write_piece(&mut sent, &[-0.25, 0.125]).unwrap();

        let mut answered = Vec::new();
        serve_with(&mut Counting, sent.as_slice(), &mut answered).unwrap();

        let mut answers = answered.as_slice();
        let mut next = || read_answer(&mut answers).unwrap();
        assert_eq!(next(), Some(Answer::Ready));
        assert_eq!(next(), Some(Answer::Words("heard 16000 samples".into())));
        assert_eq!(
            next(),
            Some(Answer::Failed(
                "speech to text failed: nothing\\nto hear".into()
            ))
        );
        assert_eq!(next(), Some(Answer::Words("heard 2 samples".into())));
        assert_eq!(next(), None, "the helper stops when its input closes");
    }

    #[test]
    fn samples_arrive_exactly() {
        let samples = [0.0, 1.0, -1.0, 0.333, f32::MIN_POSITIVE];
        let mut sent = Vec::new();
        write_piece(&mut sent, &samples).unwrap();
        let mut input = sent.as_slice();
        assert_eq!(read_piece(&mut input).unwrap().unwrap(), samples);
        assert_eq!(read_piece(&mut input).unwrap(), None);
    }

    #[test]
    fn a_garbled_length_is_refused_rather_than_allocated() {
        let mut input = &u32::MAX.to_le_bytes()[..];
        assert!(read_piece(&mut input).is_err());
    }

    #[test]
    fn answers_stay_on_one_line() {
        let mut out = Vec::new();
        reply(&mut out, Ok("John 3:16\nsays")).unwrap();
        assert_eq!(out, b"words John 3:16 says\n");
    }

    /// Runs a real helper: the app's executable with a downloaded model and
    /// a 16 kHz, 16-bit mono WAV recording. Run with
    /// `SW_HELPER_PROGRAM=target/debug/SimpleWorship SW_HELPER_MODEL=…/ggml-small.en.bin
    /// SW_HELPER_WAV=sermon.wav cargo test -p sw-listen -- --ignored`.
    #[test]
    #[ignore = "needs the built app, a speech model and a recording"]
    #[cfg(unix)]
    fn a_real_helper_writes_down_speech_and_reports_a_crash() {
        let var = |name| std::env::var_os(name).unwrap_or_else(|| panic!("set {name:?}"));
        let command = HelperCommand {
            program: PathBuf::from(var("SW_HELPER_PROGRAM")),
            args: vec![HELPER_FLAG.into(), var("SW_HELPER_MODEL")],
        };
        let wav = std::fs::read(var("SW_HELPER_WAV")).unwrap();
        let speech: Vec<f32> = wav[44..]
            .chunks_exact(2)
            .map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32_768.0)
            .collect();

        let slot = HelperSlot::default();
        let mut helper = Helper::start(&command, slot.clone()).unwrap();
        let words = helper.transcribe(&speech).unwrap();
        assert!(words.contains("John"), "heard: {words}");

        // A crash in Whisper, as a failed GGML_ASSERT ends it. (A SIGSEGV
        // sent from outside is no use for this: Rust's own handler, there
        // to catch stack overflows, lets the process carry on.)
        let pid = slot.lock().unwrap().as_ref().unwrap().id();
        let killed = Command::new("kill")
            .args(["-ABRT", &pid.to_string()])
            .status();
        assert!(killed.unwrap().success());
        let after = helper.transcribe(&speech);
        let Err(Error::Helper(why)) = after else {
            panic!("a crashed helper should be reported, got {after:?}");
        };
        assert!(why.contains("SIGABRT"), "reported: {why}");
    }

    #[test]
    #[cfg(unix)]
    fn a_crash_is_described_in_words() {
        use std::os::unix::process::ExitStatusExt;
        // A raw wait status of n (below 128) means "ended by signal n".
        let ended_by = |signal| describe_exit(ExitStatus::from_raw(signal));
        assert!(ended_by(6).contains("failed check"));
        assert!(ended_by(11).contains("memory"));
        assert_eq!(
            describe_exit(ExitStatus::from_raw(1 << 8)),
            "it ended (exit status: 1)"
        );
    }
}
