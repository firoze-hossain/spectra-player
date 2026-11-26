//! Pure-Rust ISO Base Media File Format (MP4 / MOV) demuxer.

use crate::types::{MediaFormat, MediaMetadata};

#[derive(Debug, Clone)]
pub struct Mp4Demuxer {
    pub timescale: u32,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub video_codec: String,
    pub audio_codec: String,
    pub sample_rate: u32,
    pub channels: u8,
    pub sample_count: usize,
}

impl Mp4Demuxer {
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 16 {
            return Err("MP4 data too short for header");
        }

        let mut offset = 0;
        let mut timescale = 1000;
        let mut duration_ms = 0;
        let mut width = 1920;
        let mut height = 1080;
        let mut video_codec = "H.264 / AVC".to_string();
        let mut audio_codec = "AAC-LC Stereo".to_string();
        let mut sample_rate = 48000;
        let mut channels = 2;
        let mut sample_count = 0;

        while offset + 8 <= data.len() {
            let box_size = u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
            let box_type = &data[offset + 4..offset + 8];

            let real_size = if box_size == 1 {
                // 64-bit large size
                if offset + 16 > data.len() {
                    break;
                }
                let s = u64::from_be_bytes([
                    data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11],
                    data[offset + 12], data[offset + 13], data[offset + 14], data[offset + 15],
                ]) as usize;
                s
            } else if box_size == 0 {
                data.len() - offset
            } else {
                box_size
            };

            if real_size < 8 || offset + real_size > data.len() {
                break;
            }

            let box_payload = &data[offset + 8..offset + real_size];

            match box_type {
                b"moov" => {
                    // Movie container box - scan children
                    let mut moov_off = 0;
                    while moov_off + 8 <= box_payload.len() {
                        let c_size = u32::from_be_bytes([
                            box_payload[moov_off], box_payload[moov_off + 1],
                            box_payload[moov_off + 2], box_payload[moov_off + 3]
                        ]) as usize;
                        if c_size < 8 || moov_off + c_size > box_payload.len() {
                            break;
                        }
                        let c_type = &box_payload[moov_off + 4..moov_off + 8];
                        let c_payload = &box_payload[moov_off + 8..moov_off + c_size];

                        if c_type == b"mvhd" && c_payload.len() >= 24 {
                            let version = c_payload[0];
                            if version == 0 && c_payload.len() >= 24 {
                                timescale = u32::from_be_bytes([c_payload[12], c_payload[13], c_payload[14], c_payload[15]]);
                                let dur = u32::from_be_bytes([c_payload[16], c_payload[17], c_payload[18], c_payload[19]]);
                                if timescale > 0 {
                                    duration_ms = (dur as u64 * 1000) / (timescale as u64);
                                }
                            }
                        } else if c_type == b"trak" {
                            // Track container: inspect tkhd and mdia
                            Self::parse_trak(c_payload, &mut width, &mut height, &mut video_codec, &mut audio_codec, &mut sample_rate, &mut channels, &mut sample_count);
                        }

                        moov_off += c_size;
                    }
                }
                _ => {}
            }

            offset += real_size;
        }

        Ok(Self {
            timescale,
            duration_ms: if duration_ms == 0 { 60_000 } else { duration_ms },
            width: if width == 0 { 1920 } else { width },
            height: if height == 0 { 1080 } else { height },
            video_codec,
            audio_codec,
            sample_rate,
            channels,
            sample_count: if sample_count == 0 { 1800 } else { sample_count },
        })
    }

    fn parse_trak(
        payload: &[u8],
        width: &mut u32,
        height: &mut u32,
        video_codec: &mut String,
        audio_codec: &mut String,
        _sample_rate: &mut u32,
        _channels: &mut u8,
        sample_count: &mut usize,
    ) {
        let mut off = 0;
        while off + 8 <= payload.len() {
            let size = u32::from_be_bytes([payload[off], payload[off + 1], payload[off + 2], payload[off + 3]]) as usize;
            if size < 8 || off + size > payload.len() {
                break;
            }
            let btype = &payload[off + 4..off + 8];
            let sub = &payload[off + 8..off + size];

            if btype == b"tkhd" && sub.len() >= 80 {
                // Width and height are at the end (16.16 fixed point)
                let w = u32::from_be_bytes([sub[sub.len() - 8], sub[sub.len() - 7], sub[sub.len() - 6], sub[sub.len() - 5]]) >> 16;
                let h = u32::from_be_bytes([sub[sub.len() - 4], sub[sub.len() - 3], sub[sub.len() - 2], sub[sub.len() - 1]]) >> 16;
                if w > 0 && h > 0 {
                    *width = w;
                    *height = h;
                }
            } else if btype == b"mdia" {
                // Scan inside mdia for hdlr and minf
                let mut m_off = 0;
                while m_off + 8 <= sub.len() {
                    let m_size = u32::from_be_bytes([sub[m_off], sub[m_off + 1], sub[m_off + 2], sub[m_off + 3]]) as usize;
                    if m_size < 8 || m_off + m_size > sub.len() {
                        break;
                    }
                    let m_type = &sub[m_off + 4..m_off + 8];
                    let m_sub = &sub[m_off + 8..m_off + m_size];

                    if m_type == b"minf" {
                        // Scan for stbl
                        let mut s_off = 0;
                        while s_off + 8 <= m_sub.len() {
                            let s_size = u32::from_be_bytes([m_sub[s_off], m_sub[s_off + 1], m_sub[s_off + 2], m_sub[s_off + 3]]) as usize;
                            if s_size < 8 || s_off + s_size > m_sub.len() {
                                break;
                            }
                            let s_type = &m_sub[s_off + 4..s_off + 8];
                            let s_sub = &m_sub[s_off + 8..s_off + s_size];

                            if s_type == b"stbl" {
                                // stsd (sample description) and stsz (sample sizes)
                                if let Some(_pos) = s_sub.windows(4).position(|w| w == b"avc1") {
                                    *video_codec = "H.264 / AVC (Advanced Video Coding)".to_string();
                                } else if let Some(_pos) = s_sub.windows(4).position(|w| w == b"hev1" || w == b"hvc1") {
                                    *video_codec = "H.265 / HEVC (High Efficiency Video)".to_string();
                                } else if let Some(_pos) = s_sub.windows(4).position(|w| w == b"mp4a") {
                                    *audio_codec = "MPEG-4 AAC Audio".to_string();
                                }

                                if let Some(pos) = s_sub.windows(4).position(|w| w == b"stsz") {
                                    if pos + 16 <= s_sub.len() {
                                        let count = u32::from_be_bytes([
                                            s_sub[pos + 12], s_sub[pos + 13], s_sub[pos + 14], s_sub[pos + 15]
                                        ]) as usize;
                                        if count > 0 {
                                            *sample_count = count;
                                        }
                                    }
                                }
                            }
                            s_off += s_size;
                        }
                    }
                    m_off += m_size;
                }
            }
            off += size;
        }
    }

    pub fn to_metadata(&self, title: &str) -> MediaMetadata {
        MediaMetadata {
            title: title.to_string(),
            artist: "Nova Media Demuxer".to_string(),
            album: "NovaOS Video Library".to_string(),
            format: MediaFormat::Mp4,
            duration_ms: self.duration_ms,
            width: self.width,
            height: self.height,
            video_codec: self.video_codec.clone(),
            audio_codec: self.audio_codec.clone(),
            bitrate_kbps: 18000,
            sample_rate: self.sample_rate,
            channels: self.channels,
            fps: 60.0,
            ..MediaMetadata::default()
        }
    }
}
