//! Writes down a recording and lists the Bible references in it, the way
//! Listen would: a way to try a speech model without a microphone.
//!
//! ```sh
//! cargo run -p sw-listen --example transcribe -- ggml-base.en.bin sermon.wav
//! ```
//!
//! The recording must be a 16-bit WAV file (any sample rate, mono or
//! stereo). On a Mac, `say -o sermon.wav --data-format=LEI16@16000 "…"`
//! makes one.

use std::path::Path;
use std::time::Instant;

use sw_listen::audio::{Resampler, Segmenter, to_mono};
use sw_listen::transcribe::Transcriber;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [model, recording] = args.as_slice() else {
        eprintln!("usage: transcribe <ggml model> <recording.wav>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(recording).expect("read the recording");
    let (rate, channels, samples) = read_wav(&bytes).expect("a 16-bit WAV file");
    let samples = Resampler::new(rate).process(&to_mono(&samples, channels));

    let loading = Instant::now();
    let mut transcriber = Transcriber::load(Path::new(model)).expect("load the model");
    println!("model loaded in {:.1?}", loading.elapsed());

    // As Listen does: cut at the pauses, then a second of silence to end
    // the last sentence.
    let mut segmenter = Segmenter::default();
    let mut pieces = segmenter.push(&samples);
    pieces.extend(segmenter.push(&vec![0.0; 16_000]));
    let mut spotter = sw_bible::spoken::Spotter::default();
    for piece in pieces {
        let started = Instant::now();
        let words = transcriber.transcribe(&piece).expect("transcribe");
        let seconds = piece.len() as f32 / 16_000.0;
        println!(
            "[{seconds:.1} s of speech, {:.1?}] {words}",
            started.elapsed()
        );
        for heard in spotter.hear(&words) {
            println!("    → {}  (“{}”)", heard.passage, heard.said);
        }
    }
}

/// The sample rate, channels and samples of a 16-bit PCM WAV file.
fn read_wav(bytes: &[u8]) -> Option<(u32, usize, Vec<f32>)> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let (mut rate, mut channels, mut at) = (0, 0, 12);
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32_at(at + 4)? as usize;
        let body = at + 8;
        if id == b"fmt " {
            channels = usize::from(u16_at(body + 2)?);
            rate = u32_at(body + 4)?;
            if u16_at(body + 14)? != 16 {
                return None;
            }
        } else if id == b"data" {
            let data = bytes.get(body..(body + size).min(bytes.len()))?;
            let samples = data
                .chunks_exact(2)
                .map(|pair| f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32_768.0)
                .collect();
            return Some((rate, channels, samples));
        }
        at = body + size + (size & 1);
    }
    None
}
