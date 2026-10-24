//! # sw-listen
//!
//! SimpleWorship's Listen: hears the preacher through a microphone and
//! writes down what is said, on this computer, so the scripture they read
//! can be offered to the operator (see `sw_bible::spoken`).
//!
//! - [`input_devices`]: the microphones and audio inputs to choose from.
//! - [`Listener::start`]: listens to one, and reports [`Event`]s: how loud
//!   it is (for a level meter), and the words of each sentence.
//!
//! Speech to text is Whisper (whisper.cpp), run on this computer: it works
//! without the internet, and no sound leaves the church. The model file is
//! downloaded by the app (see the desktop app's `listen.rs`).
//!
//! Inside, a capture thread takes sound from the device (in whatever
//! format it gives) and a worker thread turns it into 16 kHz pieces of
//! speech ([`audio`]) and then into words ([`transcribe`]).

pub mod audio;
pub mod transcribe;

use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

use crate::audio::{Resampler, Segmenter};
use crate::transcribe::Transcriber;

/// Something went wrong listening.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No microphone, or not the one asked for.
    #[error("no microphone found{}", .0.as_deref().map(|name| format!(" called “{name}”")).unwrap_or_default())]
    NoDevice(Option<String>),
    /// The microphone could not be opened (in use, or not allowed).
    #[error("the microphone could not be used: {0}")]
    Device(String),
    /// The speech model could not be loaded.
    #[error("the speech model could not be loaded: {0}")]
    Model(String),
    /// Whisper failed on a piece of speech.
    #[error("speech to text failed: {0}")]
    Transcribe(String),
}

/// A microphone or audio input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDevice {
    /// Identifies it to [`Listener::start`].
    pub id: String,
    /// Its name, as the system shows it.
    pub name: String,
    /// Whether it is the system's default input.
    pub default: bool,
}

/// The audio inputs on this computer.
pub fn input_devices() -> Vec<InputDevice> {
    let host = cpal::default_host();
    let default = host
        .default_input_device()
        .and_then(|device| device.id().ok())
        .map(|id| id.to_string());
    let Ok(devices) = host.input_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|device| {
            let id = device.id().ok()?.to_string();
            let name = device
                .description()
                .map(|description| description.name().to_owned())
                .unwrap_or_else(|_| id.clone());
            Some(InputDevice {
                default: default.as_deref() == Some(id.as_str()),
                id,
                name,
            })
        })
        .collect()
}

/// What a [`Listener`] reports.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// How loud the input is, 0 to 1, about ten times a second.
    Level(f32),
    /// The words of a sentence (or ten seconds of one).
    Words(String),
    /// Listening stopped by itself: the microphone was unplugged, or
    /// speech to text failed.
    Failed(String),
}

/// How often to report the level.
const LEVEL_EVERY: Duration = Duration::from_millis(100);
/// How long a device may take to start.
const START_TIMEOUT: Duration = Duration::from_secs(10);

/// Listening to a microphone. Stops when dropped.
pub struct Listener {
    stop: Option<mpsc::Sender<()>>,
}

impl Listener {
    /// Starts listening to `device` (an [`InputDevice::id`], or the
    /// system's default), with the Whisper model at `model`. Loading the
    /// model takes a second or two, so call this off the main thread.
    pub fn start(
        device: Option<&str>,
        model: &Path,
        on_event: impl Fn(Event) + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        let transcriber = Transcriber::load(model)?;
        let on_event = std::sync::Arc::new(on_event);

        let (stop, stopped) = mpsc::channel::<()>();
        // Bounded channel so capture thread never leaks memory or hangs:
        let (sound, heard) = mpsc::sync_channel::<Vec<f32>>(128);
        let (ready, started) = mpsc::channel::<Result<u32, Error>>();
        let device = device.map(str::to_owned);
        let failed = on_event.clone();
        std::thread::Builder::new()
            .name("listen-capture".into())
            .spawn(move || {
                // The stream lives on this thread (it cannot move between
                // threads on every system) until told to stop.
                match open(device.as_deref(), sound, move |message| {
                    failed(Event::Failed(message));
                }) {
                    Ok((stream, rate)) => {
                        let _ = ready.send(Ok(rate));
                        let _ = stopped.recv();
                        drop(stream);
                    }
                    Err(error) => {
                        let _ = ready.send(Err(error));
                    }
                }
            })
            .map_err(|error| Error::Device(error.to_string()))?;

        let rate = started
            .recv_timeout(START_TIMEOUT)
            .map_err(|_| Error::Device("it did not start".into()))??;

        // Bounded speech queue: if whisper takes time, don't lag behind real-time.
        let (speech_tx, speech_rx) = mpsc::sync_channel::<Vec<f32>>(2);
        let on_event_transcribe = on_event.clone();

        std::thread::Builder::new()
            .name("listen-transcribe".into())
            .spawn(move || transcribe_worker(speech_rx, transcriber, on_event_transcribe))
            .map_err(|error| Error::Device(error.to_string()))?;

        std::thread::Builder::new()
            .name("listen-audio".into())
            .spawn(move || audio_worker(rate, heard, speech_tx, on_event))
            .map_err(|error| Error::Device(error.to_string()))?;

        Ok(Self { stop: Some(stop) })
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

/// Opens the device and starts its stream, sending its sound (mixed to one
/// channel) to `sound`. Returns the stream and its sample rate.
fn open(
    device: Option<&str>,
    sound: mpsc::SyncSender<Vec<f32>>,
    on_error: impl Fn(String) + Send + 'static,
) -> Result<(cpal::Stream, u32), Error> {
    let host = cpal::default_host();
    let device = match device {
        Some(id) => {
            let parsed = id
                .parse()
                .map_err(|_| Error::NoDevice(Some(id.to_owned())))?;
            host.device_by_id(&parsed)
                .ok_or_else(|| Error::NoDevice(Some(id.to_owned())))?
        }
        None => host.default_input_device().ok_or(Error::NoDevice(None))?,
    };
    let config = device
        .default_input_config()
        .map_err(|error| Error::Device(error.to_string()))?;
    let channels = usize::from(config.channels()).max(1);
    let rate = config.sample_rate().max(16_000);
    let format = config.sample_format();
    let config = config.config();

    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, config, channels, sound, on_error),
        SampleFormat::I16 => build::<i16>(&device, config, channels, sound, on_error),
        SampleFormat::U16 => build::<u16>(&device, config, channels, sound, on_error),
        SampleFormat::I32 => build::<i32>(&device, config, channels, sound, on_error),
        SampleFormat::U8 => build::<u8>(&device, config, channels, sound, on_error),
        other => {
            return Err(Error::Device(format!(
                "its sound format ({other}) is not supported"
            )));
        }
    }?;
    stream
        .play()
        .map_err(|error| Error::Device(error.to_string()))?;
    Ok((stream, rate))
}

/// Builds an input stream for samples of type `T`.
fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    sound: mpsc::SyncSender<Vec<f32>>,
    on_error: impl Fn(String) + Send + 'static,
) -> Result<cpal::Stream, Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST_GLITCH: AtomicU64 = AtomicU64::new(0);

    let channels = channels.max(1);
    device
        .build_input_stream::<T, _, _>(
            config,
            move |data: &[T], _| {
                if data.is_empty() {
                    return;
                }
                let samples: Vec<f32> = data
                    .iter()
                    .map(|&s| {
                        let f = s.to_sample::<f32>();
                        if f.is_finite() {
                            f.clamp(-1.0, 1.0)
                        } else {
                            0.0
                        }
                    })
                    .collect();
                let _ = sound.try_send(audio::to_mono(&samples, channels));
            },
            move |error| {
                let msg = error.to_string();
                let lower = msg.to_lowercase();
                if lower.contains("underrun") || lower.contains("overrun") {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs());
                    let last = LAST_GLITCH.load(Ordering::Relaxed);
                    if now.saturating_sub(last) >= 5 {
                        LAST_GLITCH.store(now, Ordering::Relaxed);
                        log::warn!("listen: transient buffer glitch ignored: {msg}");
                    }
                } else {
                    on_error(msg);
                }
            },
            None,
        )
        .map_err(|error| Error::Device(error.to_string()))
}

/// The audio worker: sound in, levels out, cuts into pieces of speech.
fn audio_worker(
    rate: u32,
    heard: mpsc::Receiver<Vec<f32>>,
    speech_tx: mpsc::SyncSender<Vec<f32>>,
    on_event: std::sync::Arc<dyn Fn(Event) + Send + Sync>,
) {
    let mut resampler = Resampler::new(rate);
    let mut segmenter = Segmenter::default();
    let mut loudest = 0.0_f32;
    let mut last_level = Instant::now();

    while let Ok(sound) = heard.recv() {
        let samples = resampler.process(&sound);
        loudest = loudest.max(audio::level(&samples));
        if last_level.elapsed() >= LEVEL_EVERY {
            // Speech is around 0.05–0.3; show it across most of the meter.
            on_event(Event::Level((loudest * 4.0).min(1.0)));
            loudest = 0.0;
            last_level = Instant::now();
        }
        for piece in segmenter.push(&samples) {
            // If the transcriber is still busy with previous speech and queue is full,
            // try_send will drop this piece to avoid falling behind real-time.
            let _ = speech_tx.try_send(piece);
        }
    }
}

/// The transcription worker: pieces of speech in, words out via Whisper.
fn transcribe_worker(
    speech_rx: mpsc::Receiver<Vec<f32>>,
    mut transcriber: Transcriber,
    on_event: std::sync::Arc<dyn Fn(Event) + Send + Sync>,
) {
    while let Ok(piece) = speech_rx.recv() {
        match transcriber.transcribe(&piece) {
            Ok(words) if !words.is_empty() => on_event(Event::Words(words)),
            Ok(_) => {}
            Err(error) => {
                on_event(Event::Failed(error.to_string()));
                return;
            }
        }
    }
}
