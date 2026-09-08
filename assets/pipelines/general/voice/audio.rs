//! Voice audio preparation — the capture ↔ backend contract.
//!
//! Captures arrive in whatever format the source produced (MediaRecorder
//! webm/opus, AudioContext WAV/PCM, uploaded files). Each transcription
//! backend needs a DIFFERENT input:
//!
//! | backend      | input contract                          |
//! |--------------|------------------------------------------|
//! | whisper_cpp  | 16 kHz mono PCM16 WAV **file on disk**   |
//! | whisper_rs   | 16 kHz mono i16 samples **in memory**    |
//! | api          | provider-transcoded (base64 passthrough)|
//!
//! This module normalizes ANY arriving format to the backend's contract:
//! WAV parse/write (std-only), linear resample to 16 kHz, stereo→mono
//! downmix, and ffmpeg transcode for compressed containers (webm/opus/mp3)
//! when ffmpeg is available.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Whisper's one true sample rate.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// A parsed, in-memory PCM capture.
#[derive(Debug, Clone)]
pub struct PcmAudio {
    /// Mono samples.
    pub samples: Vec<i16>,
    pub sample_rate: u32,
}

impl PcmAudio {
    /// Linear-interpolation resample to `target` Hz + guaranteed mono.
    /// Naive but genuine for speech (linear is fine at 16 kHz targets;
    /// windowed-sinc is overkill for ASR preprocessing).
    pub fn to_rate(&self, target: u32) -> PcmAudio {
        if self.sample_rate == target {
            return PcmAudio { samples: self.samples.clone(), sample_rate: target };
        }
        let src = self.sample_rate as f64;
        let dst = target as f64;
        let ratio = src / dst;
        let out_len = (self.samples.len() as f64 / ratio).floor() as usize;
        let mut out = Vec::with_capacity(out_len);
        for i in 0..out_len {
            let pos = i as f64 * ratio;
            let i0 = pos.floor() as usize;
            let i1 = (i0 + 1).min(self.samples.len() - 1);
            let frac = pos - i0 as f64;
            let a = self.samples[i0] as f64;
            let b = self.samples[i1] as f64;
            out.push((a + (b - a) * frac) as i16);
        }
        PcmAudio { samples: out, sample_rate: target }
    }

    /// Wrap into a canonical 16 kHz mono PCM16 WAV byte stream.
    pub fn to_wav_16k(&self) -> Vec<u8> {
        let normalized = if self.sample_rate == WHISPER_SAMPLE_RATE {
            self.clone()
        } else {
            self.to_rate(WHISPER_SAMPLE_RATE)
        };
        write_wav_pcm16(&normalized.samples, WHISPER_SAMPLE_RATE)
    }
}

/// Parse a PCM16 WAV file (mono or stereo; any rate). Returns the samples.
pub fn parse_wav(bytes: &[u8]) -> Result<PcmAudio, String> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE stream".into());
    }
    let mut pos = 12usize;
    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut bits = 0u16;
    let mut data: Option<&[u8]> = None;

    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes([
            bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7],
        ]) as usize;
        let body_start = pos + 8;
        let body_end = (body_start + size).min(bytes.len());
        match id {
            b"fmt " => {
                let b = &bytes[body_start..body_end];
                if b.len() < 16 {
                    return Err("fmt chunk too small".into());
                }
                channels = u16::from_le_bytes([b[2], b[3]]);
                sample_rate = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
                bits = u16::from_le_bytes([b[14], b[15]]);
            }
            b"data" => data = Some(&bytes[body_start..body_end]),
            _ => {}
        }
        pos = body_start + size + (size & 1); // chunks are word-aligned
    }

    let data = data.ok_or("WAV has no data chunk")?;
    if bits != 16 {
        return Err(format!("only PCM16 WAV supported, got {bits}-bit"));
    }
    if channels == 0 {
        return Err("WAV has zero channels".into());
    }

    let mut samples = Vec::with_capacity(data.len() / 2);
    for chunk in data.chunks_exact(2) {
        samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
    }
    // Downmix stereo (and higher) to mono by averaging.
    if channels > 1 {
        let ch = channels as usize;
        samples = samples.chunks(ch).map(|frame| {
            let sum: i32 = frame.iter().map(|s| *s as i32).sum();
            (sum / ch as i32) as i16
        }).collect();
    }

    Ok(PcmAudio { samples, sample_rate })
}

/// Write mono PCM16 samples as a canonical WAV stream (44-byte header).
pub fn write_wav_pcm16(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let channels: u16 = 1;
    let bits: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * (bits / 8) as u32;
    let block_align = channels * (bits / 8);
    let data_len = (samples.len() * 2) as u32;

    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Transcode any ffmpeg-readable container (webm/opus/mp3/m4a/…) to
/// 16 kHz mono PCM16 WAV bytes. Returns Err if ffmpeg is absent or fails —
/// the caller decides whether to instruct WAV capture instead.
pub fn transcode_to_wav_16k_via_ffmpeg(
    input_bytes: &[u8],
    input_format: &str,
    ffmpeg_path: &str,
) -> Result<Vec<u8>, String> {
    let tmp_in = std::env::temp_dir().join(format!(
        "oz-voice-in-{}.{}",
        std::process::id(),
        input_format.trim_start_matches('.')
    ));
    let tmp_out = std::env::temp_dir().join(format!("oz-voice-out-{}.wav", std::process::id()));
    std::fs::write(&tmp_in, input_bytes).map_err(|e| e.to_string())?;

    let result = Command::new(ffmpeg_path)
        .args([
            "-y", "-i",
            tmp_in.to_string_lossy().as_ref(),
            "-ac", "1", // mono
            "-ar", &WHISPER_SAMPLE_RATE.to_string(),
            "-c:a", "pcm_s16le",
            tmp_out.to_string_lossy().as_ref(),
        ])
        .output();

    let out = result.map_err(|e| {
        let _ = std::fs::remove_file(&tmp_in);
        format!("ffmpeg spawn failed ({e}): is ffmpeg installed and on PATH?")
    });
    let _ = std::fs::remove_file(&tmp_in);

    match out {
        Ok(o) if o.status.success() => {
            let wav = std::fs::read(&tmp_out).map_err(|e| e.to_string());
            let _ = std::fs::remove_file(&tmp_out);
            wav
        }
        Ok(o) => {
            let _ = std::fs::remove_file(&tmp_out);
            Err(format!(
                "ffmpeg transcode failed: {}",
                String::from_utf8_lossy(&o.stderr)
            ))
        }
        Err(e) => Err(e),
    }
}

/// Prepare a temp WAV file for the whisper_cpp CLI backend.
pub fn prepare_wav_file_for_cli(
    audio_bytes: &[u8],
    format: &str,
    ffmpeg_path: &str,
) -> Result<PathBuf, String> {
    let wav = match format.to_lowercase().as_str() {
        "wav" | "wave" => audio_bytes.to_vec(),
        other => transcode_to_wav_16k_via_ffmpeg(audio_bytes, other, ffmpeg_path)?,
    };
    // Validate what we wrote/arrived with so CLI failures are precise.
    parse_wav(&wav).map_err(|e| format!("audio preparation failed: {e}"))?;
    let path = std::env::temp_dir().join(format!("oz-whisper-{}.wav", std::process::id()));
    std::fs::write(&path, &wav).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Load samples for the whisper_rs integrated backend (any supported capture).
pub fn prepare_samples_for_rs(
    audio_bytes: &[u8],
    format: &str,
    ffmpeg_path: &str,
) -> Result<PcmAudio, String> {
    let wav = match format.to_lowercase().as_str() {
        "wav" | "wave" => audio_bytes.to_vec(),
        other => transcode_to_wav_16k_via_ffmpeg(audio_bytes, other, ffmpeg_path)?,
    };
    let pcm = parse_wav(&wav).map_err(|e| format!("audio preparation failed: {e}"))?;
    Ok(pcm.to_rate(WHISPER_SAMPLE_RATE))
}

/// Best-effort path cleanup helper for CLI temp files.
pub fn cleanup_temp_wav(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f64, rate: u32, secs: f64) -> Vec<i16> {
        (0..(rate as f64 * secs) as usize)
            .map(|i| ((freq * 2.0 * std::f64::consts::PI * i as f64 / rate as f64).sin() * 8000.0) as i16)
            .collect()
    }

    #[test]
    fn wav_round_trip() {
        let samples = tone(440.0, 16_000, 0.1);
        let bytes = write_wav_pcm16(&samples, 16_000);
        let parsed = parse_wav(&bytes).unwrap();
        assert_eq!(parsed.sample_rate, 16_000);
        assert_eq!(parsed.samples.len(), samples.len());
        assert_eq!(parsed.samples[100], samples[100]);
    }

    #[test]
    fn resample_48k_to_16k() {
        let pcm = PcmAudio { samples: tone(440.0, 48_000, 0.1), sample_rate: 48_000 };
        let down = pcm.to_rate(16_000);
        assert_eq!(down.sample_rate, 16_000);
        // 3:1 downsample → roughly a third of the samples.
        assert!((down.samples.len() as i64 - (pcm.samples.len() / 3) as i64).abs() <= 2);
    }

    #[test]
    fn rejects_non_wav() {
        assert!(parse_wav(b"not a wav at all........").is_err());
    }
}
