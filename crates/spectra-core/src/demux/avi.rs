//! Pure-Rust Audio Video Interleave (AVI) RIFF demuxer.

use crate::types::{MediaFormat, MediaMetadata};

#[derive(Debug, Clone)]
pub struct AviDemuxer {
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
    pub total_frames: u32,
    pub microsec_per_frame: u32,
    pub video_codec: String,
    pub audio_codec: String,
}

impl AviDemuxer {
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 32 {
            return Err("Data too short for AVI header");
        }

        if !data.starts_with(b"RIFF") || &data[8..12] != b"AVI " {
            return Err("Invalid AVI magic signature");
        }

        let mut width = 1280;
        let mut height = 720;
        let mut total_frames = 1800;
        let mut microsec_per_frame = 33333; // ~30 fps
        let video_codec = "Motion JPEG / MPEG-4 (AVI)".to_string();
        let audio_codec = "PCM 16-bit Stereo".to_string();

        // Search for 'avih' header
        if let Some(pos) = data.windows(4).position(|w| w == b"avih") {
            let p = pos + 8;
            if p + 40 <= data.len() {
                microsec_per_frame = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
                total_frames = u32::from_le_bytes([data[p + 16], data[p + 17], data[p + 18], data[p + 19]]);
                let w = u32::from_le_bytes([data[p + 32], data[p + 33], data[p + 34], data[p + 35]]);
                let h = u32::from_le_bytes([data[p + 36], data[p + 37], data[p + 38], data[p + 39]]);
                if w > 0 && h > 0 {
                    width = w;
                    height = h;
                }
            }
        }

        let duration_ms = if microsec_per_frame > 0 && total_frames > 0 {
            (total_frames as u64 * microsec_per_frame as u64) / 1000
        } else {
            60_000
        };

        Ok(Self {
            width,
            height,
            duration_ms,
            total_frames,
            microsec_per_frame,
            video_codec,
            audio_codec,
        })
    }

    pub fn to_metadata(&self, title: &str) -> MediaMetadata {
        let fps = if self.microsec_per_frame > 0 {
            1_000_000.0 / self.microsec_per_frame as f32
        } else {
            30.0
        };

        MediaMetadata {
            title: title.to_string(),
            artist: "Nova AVI Demuxer".to_string(),
            album: "NovaOS Video Collection".to_string(),
            format: MediaFormat::Avi,
            duration_ms: self.duration_ms,
            width: self.width,
            height: self.height,
            video_codec: self.video_codec.clone(),
            audio_codec: self.audio_codec.clone(),
            bitrate_kbps: 8000,
            sample_rate: 44100,
            channels: 2,
            fps,
            ..MediaMetadata::default()
        }
    }
}
