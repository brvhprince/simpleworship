//! Speech to text with Whisper (whisper.cpp), on this computer.

use std::path::Path;

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::Error;
use crate::audio::SAMPLE_RATE;

/// Nudges Whisper to write references the way the scripture finder reads
/// them best ("John 3:16", not "John three sixteen").
const PROMPT: &str = "The preacher reads from the Bible: John 3:16, Romans 8:28-30, \
                      1 Corinthians 13:4-7, Psalm 23, Isaiah 40:31.";

/// A segment Whisper is this sure is not speech is dropped.
const NOT_SPEECH: f32 = 0.6;

/// A loaded Whisper model, ready to turn pieces of speech into text.
pub struct Transcriber {
    _context: WhisperContext,
    state: WhisperState,
    threads: i32,
}

impl Transcriber {
    /// Loads the model at `path` (a whisper.cpp `ggml-*.bin` file). Takes
    /// a second or two.
    pub fn load(path: &Path) -> Result<Self, Error> {
        whisper_rs::install_logging_hooks();
        let context = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|error| Error::Model(error.to_string()))?;
        let state = context
            .create_state()
            .map_err(|error| Error::Model(error.to_string()))?;
        // Half the computer's processors, from 2 to 6: measured on an
        // 8-core Intel Mac, 4 threads are 1.4× faster than 2, and more than
        // 6 add nothing. Whisper runs in its own process at a lower
        // priority (see `worker`), so the sound and the operator window
        // still come first when the computer is busy.
        let processors = std::thread::available_parallelism().map_or(2, usize::from);
        let threads = (processors / 2).clamp(2, 6) as i32;
        Ok(Self {
            _context: context,
            state,
            threads,
        })
    }

    /// The words in a piece of 16 kHz speech.
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<String, Error> {
        // Sanitize any non-finite floats (NaN/Inf) which can cause GGML assertions to abort.
        let mut sanitized;
        let audio = if audio.iter().any(|s| !s.is_finite()) {
            sanitized = audio.to_vec();
            for s in &mut sanitized {
                if !s.is_finite() {
                    *s = 0.0;
                } else {
                    *s = s.clamp(-1.0, 1.0);
                }
            }
            &sanitized[..]
        } else {
            audio
        };

        // Whisper needs at least a second.
        let mut padded;
        let audio = if audio.len() < SAMPLE_RATE as usize {
            padded = audio.to_vec();
            padded.resize(SAMPLE_RATE as usize + SAMPLE_RATE as usize / 10, 0.0);
            &padded
        } else {
            audio
        };

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_n_threads(self.threads);
        params.set_audio_ctx(audio_context(audio.len()));
        params.set_initial_prompt(PROMPT);
        params.set_no_context(true);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        self.state
            .full(params, audio)
            .map_err(|error| Error::Transcribe(error.to_string()))?;

        let mut text = String::new();
        for segment in self.state.as_iter() {
            if segment.no_speech_probability() > NOT_SPEECH {
                continue;
            }
            if let Ok(words) = segment.to_str_lossy() {
                text.push_str(&words);
            }
        }
        Ok(clean(&text))
    }
}

/// How much of its 30-second window Whisper looks at for a piece of
/// `samples` samples: the piece itself (50 positions a second), plus a
/// little to spare.
///
/// By default Whisper always works through the whole 30 seconds, however
/// short the piece. Fitting the window to the piece made a 10-second piece
/// twice as fast and a 1-second one four times as fast (small.en on an
/// 8-core Intel Mac), with the same words.
fn audio_context(samples: usize) -> i32 {
    /// Positions in Whisper's window for each second of sound.
    const PER_SECOND: usize = 50;
    /// About a second more, so the end of a sentence is not cut off.
    const SPARE: usize = 64;
    /// The whole window: 30 seconds.
    const FULL: usize = 1500;
    let positions = samples * PER_SECOND / SAMPLE_RATE as usize + SPARE;
    positions.min(FULL) as i32
}

/// Whisper's own notes ("[BLANK_AUDIO]", "(music)") are not words.
fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{SAMPLE_RATE, audio_context, clean};

    #[test]
    fn the_window_fits_the_piece() {
        let seconds = |s: u32| (s * SAMPLE_RATE) as usize;
        assert_eq!(audio_context(seconds(10)), 564);
        assert_eq!(audio_context(seconds(1)), 114);
        assert_eq!(audio_context(seconds(45)), 1500, "never past the full 30 s");
    }

    #[test]
    fn drops_whispers_notes() {
        assert_eq!(clean(" [BLANK_AUDIO] "), "");
        assert_eq!(clean(" (music) John 3:16  says"), "John 3:16 says");
    }
}
