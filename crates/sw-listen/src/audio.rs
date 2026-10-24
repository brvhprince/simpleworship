//! Turning a microphone's sound into what Whisper needs: one channel at
//! 16 000 samples a second ([`Resampler`]), cut into pieces at the pauses
//! between sentences ([`Segmenter`]).
//!
//! Cutting at pauses keeps words whole and gives each sentence to Whisper
//! as soon as it is finished, which is what makes a reference appear a
//! second or two after it is said.

/// Whisper's sample rate.
pub const SAMPLE_RATE: u32 = 16_000;

/// Samples in one 30 ms frame, the unit the [`Segmenter`] listens in.
const FRAME: usize = SAMPLE_RATE as usize * 30 / 1000;
/// Frames kept from before speech starts, so its first word is not cut.
const PRE_ROLL_FRAMES: usize = 10;
/// A pause this long (in frames, 600 ms) ends a piece.
const PAUSE_FRAMES: usize = 20;
/// A piece with less speech than this (in frames, 300 ms) is a cough or a
/// click, not words.
const MIN_SPEECH_FRAMES: usize = 10;
/// The longest piece, in samples (10 s): a preacher who never pauses still
/// gets heard.
const MAX_PIECE: usize = SAMPLE_RATE as usize * 10;
/// Quieter than this is never speech, however quiet the room.
const MIN_SPEECH_LEVEL: f32 = 0.004;
/// Speech is this many times louder than the room's background.
const SPEECH_OVER_ROOM: f32 = 3.0;

/// Mixes interleaved samples (`channels` per frame) down to one channel.
pub fn to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    let channels = channels.max(1);
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Converts one channel of sound from any sample rate to 16 000 a second,
/// a buffer at a time. Going down, each new sample is the average of the
/// ones it replaces (which also filters out what 16 000 cannot hold);
/// going up, it is drawn between its neighbours.
#[derive(Debug)]
pub struct Resampler {
    /// Input samples per output sample.
    step: f64,
    /// Where the next output sample starts, in input samples from the
    /// start of the next buffer (going down), or between the last input
    /// sample and the next (going up).
    position: f64,
    sum: f32,
    count: u32,
    previous: f32,
}

impl Resampler {
    /// A resampler from `rate` samples a second.
    pub fn new(rate: u32) -> Self {
        let rate = rate.max(1);
        Self {
            step: f64::from(rate) / f64::from(SAMPLE_RATE),
            position: 0.0,
            sum: 0.0,
            count: 0,
            previous: 0.0,
        }
    }

    /// The 16 kHz samples for the next buffer of input.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        let mut output = Vec::with_capacity((input.len() as f64 / self.step) as usize + 1);
        if self.step >= 1.0 {
            // Down: average each window of `step` input samples.
            for &sample in input {
                self.sum += sample;
                self.count += 1;
                self.position += 1.0;
                if self.position >= self.step {
                    output.push(self.sum / self.count as f32);
                    self.sum = 0.0;
                    self.count = 0;
                    self.position -= self.step;
                }
            }
        } else {
            // Up: draw between the previous sample and this one.
            for &sample in input {
                while self.position < 1.0 {
                    let t = self.position as f32;
                    output.push(self.previous + (sample - self.previous) * t);
                    self.position += self.step;
                }
                self.position -= 1.0;
                self.previous = sample;
            }
        }
        output
    }
}

/// How loud a stretch of sound is (root mean square, 0 to about 1).
pub fn level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Cuts 16 kHz sound into pieces of speech, at the pauses.
///
/// It listens in 30 ms frames. A frame is speech when it is clearly louder
/// than the room (whose background level it follows, so a fan or quiet
/// music does not count). A piece starts with the first speech, keeps a
/// little from before it, and ends at a 600 ms pause or after 10 seconds.
#[derive(Debug)]
pub struct Segmenter {
    /// Samples not yet a whole frame.
    partial: Vec<f32>,
    /// The last few quiet frames, kept for the start of the next piece.
    pre_roll: std::collections::VecDeque<Vec<f32>>,
    /// The piece so far, if speech has started.
    piece: Vec<f32>,
    speaking: bool,
    speech_frames: usize,
    quiet_frames: usize,
    /// The room's background level.
    room: f32,
}

impl Default for Segmenter {
    fn default() -> Self {
        Self {
            partial: Vec::with_capacity(FRAME),
            pre_roll: std::collections::VecDeque::with_capacity(PRE_ROLL_FRAMES + 1),
            piece: Vec::new(),
            speaking: false,
            speech_frames: 0,
            quiet_frames: 0,
            room: 0.01,
        }
    }
}

impl Segmenter {
    /// Takes the next 16 kHz samples; returns any pieces they finished.
    pub fn push(&mut self, samples: &[f32]) -> Vec<Vec<f32>> {
        let mut finished = Vec::new();
        for &sample in samples {
            self.partial.push(sample);
            if self.partial.len() == FRAME {
                let frame = std::mem::replace(&mut self.partial, Vec::with_capacity(FRAME));
                if let Some(piece) = self.frame(frame) {
                    finished.push(piece);
                }
            }
        }
        finished
    }

    /// One 30 ms frame; a finished piece, if this frame ends one.
    fn frame(&mut self, frame: Vec<f32>) -> Option<Vec<f32>> {
        let loudness = level(&frame);
        let speech = loudness > (self.room * SPEECH_OVER_ROOM).max(MIN_SPEECH_LEVEL);
        // The room's level drops quickly to quiet frames and rises slowly,
        // so speech itself barely moves it.
        if loudness < self.room {
            self.room = loudness.max(1e-5);
        } else if !speech {
            self.room += (loudness - self.room) * 0.01;
        } else {
            self.room += (loudness - self.room) * 0.0005;
        }

        if !self.speaking {
            if speech {
                self.speaking = true;
                self.speech_frames = 1;
                self.quiet_frames = 0;
                self.piece = self.pre_roll.drain(..).flatten().collect();
                self.piece.extend_from_slice(&frame);
            } else {
                self.pre_roll.push_back(frame);
                if self.pre_roll.len() > PRE_ROLL_FRAMES {
                    self.pre_roll.pop_front();
                }
            }
            return None;
        }

        self.piece.extend_from_slice(&frame);
        if speech {
            self.speech_frames += 1;
            self.quiet_frames = 0;
        } else {
            self.quiet_frames += 1;
        }
        // Another frame would go over the longest piece.
        if self.quiet_frames >= PAUSE_FRAMES || self.piece.len() + FRAME > MAX_PIECE {
            return self.finish();
        }
        None
    }

    /// Ends the piece now (also when listening stops). `None` if it held
    /// too little speech to be words.
    pub fn finish(&mut self) -> Option<Vec<f32>> {
        let piece = std::mem::take(&mut self.piece);
        let enough = self.speech_frames >= MIN_SPEECH_FRAMES;
        self.speaking = false;
        self.speech_frames = 0;
        self.quiet_frames = 0;
        enough.then_some(piece)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `seconds` of a tone at `loudness`, at 16 kHz.
    fn tone(seconds: f32, loudness: f32) -> Vec<f32> {
        let count = (SAMPLE_RATE as f32 * seconds) as usize;
        (0..count)
            .map(|i| loudness * (i as f32 * 0.05).sin() * std::f32::consts::SQRT_2)
            .collect()
    }

    #[test]
    fn mixes_down_to_one_channel() {
        assert_eq!(to_mono(&[1.0, 0.0, 0.5, 0.5], 2), [0.5, 0.5]);
    }

    #[test]
    fn resamples_to_16_khz() {
        let mut down = Resampler::new(48_000);
        assert_eq!(down.process(&vec![0.5; 48_000]).len(), 16_000);
        let mut up = Resampler::new(8_000);
        assert_eq!(up.process(&vec![0.5; 8_000]).len(), 16_000);
        // In pieces, nothing is lost or doubled.
        let mut odd = Resampler::new(44_100);
        let total: usize = (0..10).map(|_| odd.process(&vec![0.0; 4_410]).len()).sum();
        assert!((15_999..=16_001).contains(&total), "{total}");
    }

    #[test]
    fn cuts_at_the_pause_after_speech() {
        let mut segmenter = Segmenter::default();
        let mut pieces = segmenter.push(&tone(1.0, 0.001));
        pieces.extend(segmenter.push(&tone(2.0, 0.2)));
        assert!(pieces.is_empty(), "still speaking");
        pieces.extend(segmenter.push(&tone(1.0, 0.001)));
        assert_eq!(pieces.len(), 1);
        // The speech, a little from before it and the pause after it.
        let seconds = pieces[0].len() as f32 / SAMPLE_RATE as f32;
        assert!((2.2..3.0).contains(&seconds), "{seconds}");
    }

    #[test]
    fn a_click_is_not_speech() {
        let mut segmenter = Segmenter::default();
        let mut pieces = segmenter.push(&tone(1.0, 0.001));
        pieces.extend(segmenter.push(&tone(0.1, 0.3)));
        pieces.extend(segmenter.push(&tone(1.0, 0.001)));
        assert!(pieces.is_empty());
    }

    #[test]
    fn someone_who_never_pauses_is_still_heard() {
        let mut segmenter = Segmenter::default();
        segmenter.push(&tone(0.5, 0.001));
        let pieces = segmenter.push(&tone(25.0, 0.2));
        assert!(pieces.len() >= 2, "{}", pieces.len());
        assert!(pieces.iter().all(|piece| piece.len() <= MAX_PIECE));
    }
}
