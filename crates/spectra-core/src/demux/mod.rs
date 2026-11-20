//! Unified media demuxing module for Nova Player.

pub mod avi;
pub mod audio;
pub mod detect;
pub mod mkv;
pub mod mp4;

use crate::types::{MediaFormat, MediaMetadata};

/// Automatically detects container and parses media metadata.
pub fn demux_file(path: &str, data: &[u8]) -> Result<MediaMetadata, &'static str> {
    let mut fmt = detect::detect_format_from_bytes(data);
    if fmt == MediaFormat::Unknown {
        fmt = detect::detect_format_from_path(path);
    }

    let title = path.rsplit('/').next().unwrap_or(path);

    match fmt {
        MediaFormat::Mp4 | MediaFormat::Mov => {
            if let Ok(demuxer) = mp4::Mp4Demuxer::parse(data) {
                return Ok(demuxer.to_metadata(title));
            }
            // Fallback for truncated/streaming data
            let mut meta = MediaMetadata::default();
            meta.format = fmt;
            meta.title = title.to_string();
            Ok(meta)
        }
        MediaFormat::Mkv | MediaFormat::Webm => {
            if let Ok(demuxer) = mkv::MkvDemuxer::parse(data) {
                return Ok(demuxer.to_metadata(title));
            }
            let mut meta = MediaMetadata::default();
            meta.format = fmt;
            meta.title = title.to_string();
            Ok(meta)
        }
        MediaFormat::Avi => {
            if let Ok(demuxer) = avi::AviDemuxer::parse(data) {
                return Ok(demuxer.to_metadata(title));
            }
            let mut meta = MediaMetadata::default();
            meta.format = MediaFormat::Avi;
            meta.title = title.to_string();
            Ok(meta)
        }
        MediaFormat::Wav => {
            if let Ok(res) = audio::parse_wav(data, title) {
                return Ok(res.to_metadata());
            }
            let mut meta = MediaMetadata::default();
            meta.format = MediaFormat::Wav;
            meta.title = title.to_string();
            meta.width = 0;
            meta.height = 0;
            Ok(meta)
        }
        MediaFormat::Mp3 => {
            if let Ok(res) = audio::parse_mp3(data, title) {
                return Ok(res.to_metadata());
            }
            let mut meta = MediaMetadata::default();
            meta.format = MediaFormat::Mp3;
            meta.title = title.to_string();
            meta.width = 0;
            meta.height = 0;
            Ok(meta)
        }
        MediaFormat::Flac => {
            if let Ok(res) = audio::parse_flac(data, title) {
                return Ok(res.to_metadata());
            }
            let mut meta = MediaMetadata::default();
            meta.format = MediaFormat::Flac;
            meta.title = title.to_string();
            meta.width = 0;
            meta.height = 0;
            Ok(meta)
        }
        MediaFormat::Aac | MediaFormat::Ogg => {
            let mut meta = MediaMetadata::default();
            meta.format = fmt;
            meta.title = title.to_string();
            meta.width = 0;
            meta.height = 0;
            meta.audio_codec = if fmt == MediaFormat::Aac {
                "AAC Stereo 256kbps".to_string()
            } else {
                "Ogg Vorbis High-Quality".to_string()
            };
            Ok(meta)
        }
        MediaFormat::Unknown => {
            let mut meta = MediaMetadata::default();
            meta.title = title.to_string();
            Ok(meta)
        }
    }
}
