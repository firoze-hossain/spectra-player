//! Unified media demuxing module for Spectra Media Studio.

pub mod audio;
pub mod avi;
pub mod detect;
pub mod mkv;
pub mod mp4;

use crate::types::{DecoderBackend, MediaFormat, MediaMetadata};

/// Automatically detects container and parses media metadata from bytes/path.
pub fn demux_file(path: &str, data: &[u8]) -> Result<MediaMetadata, &'static str> {
    let mut fmt = detect::detect_format_from_bytes(data);
    if fmt == MediaFormat::Unknown {
        fmt = detect::detect_format_from_path(path);
    }

    let title = path.rsplit(['/', '\\']).next().unwrap_or(path);

    match fmt {
        MediaFormat::Mp4 | MediaFormat::Mov | MediaFormat::M4a | MediaFormat::ThreeGp => {
            if let Ok(demuxer) = mp4::Mp4Demuxer::parse(data) {
                let mut meta = demuxer.to_metadata(title);
                meta.format = fmt;
                meta.path = path.to_string();
                if fmt == MediaFormat::M4a {
                    meta.width = 0;
                    meta.height = 0;
                }
                return Ok(meta);
            }
            let mut meta = MediaMetadata::default();
            meta.path = path.to_string();
            meta.format = fmt;
            meta.title = title.to_string();
            if fmt == MediaFormat::M4a {
                meta.width = 0;
                meta.height = 0;
            }
            Ok(meta)
        }
        MediaFormat::Mkv | MediaFormat::Webm => {
            if let Ok(demuxer) = mkv::MkvDemuxer::parse(data) {
                let mut meta = demuxer.to_metadata(title);
                meta.path = path.to_string();
                return Ok(meta);
            }
            let mut meta = MediaMetadata::default();
            meta.path = path.to_string();
            meta.format = fmt;
            meta.title = title.to_string();
            Ok(meta)
        }
        MediaFormat::Avi => {
            if let Ok(demuxer) = avi::AviDemuxer::parse(data) {
                let mut meta = demuxer.to_metadata(title);
                meta.path = path.to_string();
                return Ok(meta);
            }
            let mut meta = MediaMetadata::default();
            meta.path = path.to_string();
            meta.format = MediaFormat::Avi;
            meta.title = title.to_string();
            Ok(meta)
        }
        MediaFormat::Wav => {
            if let Ok(res) = audio::parse_wav(data, title) {
                let mut meta = res.to_metadata();
                meta.path = path.to_string();
                return Ok(meta);
            }
            audio_fallback(path, title, MediaFormat::Wav, "Linear PCM")
        }
        MediaFormat::Mp3 => {
            if let Ok(res) = audio::parse_mp3(data, title) {
                let mut meta = res.to_metadata();
                meta.path = path.to_string();
                return Ok(meta);
            }
            audio_fallback(path, title, MediaFormat::Mp3, "MPEG-1 Layer III")
        }
        MediaFormat::Flac => {
            if let Ok(res) = audio::parse_flac(data, title) {
                let mut meta = res.to_metadata();
                meta.path = path.to_string();
                return Ok(meta);
            }
            audio_fallback(path, title, MediaFormat::Flac, "FLAC Lossless")
        }
        MediaFormat::Aac | MediaFormat::Ogg | MediaFormat::Opus | MediaFormat::Wma | MediaFormat::Alac | MediaFormat::Aiff | MediaFormat::Ac3 => {
            let codec = match fmt {
                MediaFormat::Aac => "AAC Stereo",
                MediaFormat::Ogg => "Ogg Vorbis",
                MediaFormat::Opus => "Opus Interactive Audio",
                MediaFormat::Wma => "Windows Media Audio",
                MediaFormat::Alac => "Apple Lossless (ALAC)",
                MediaFormat::Aiff => "AIFF Uncompressed PCM",
                MediaFormat::Ac3 => "Dolby Digital (AC3 / E-AC3)",
                _ => "Audio",
            };
            audio_fallback(path, title, fmt, codec)
        }
        MediaFormat::Flv | MediaFormat::Ts | MediaFormat::Mpeg | MediaFormat::Wmv | MediaFormat::Ogv | MediaFormat::NetworkStream => {
            let mut meta = MediaMetadata::default();
            meta.path = path.to_string();
            meta.format = fmt;
            meta.title = title.to_string();
            meta.artist = if fmt == MediaFormat::NetworkStream {
                "Network Stream".to_string()
            } else {
                "Media Stream".to_string()
            };
            meta.backend = DecoderBackend::FfmpegUniversal;
            Ok(meta)
        }
        MediaFormat::Unknown => {
            let mut meta = MediaMetadata::default();
            meta.path = path.to_string();
            meta.title = title.to_string();
            Ok(meta)
        }
    }
}

fn audio_fallback(path: &str, title: &str, format: MediaFormat, codec: &str) -> Result<MediaMetadata, &'static str> {
    let mut meta = MediaMetadata::default();
    meta.path = path.to_string();
    meta.format = format;
    meta.title = title.to_string();
    meta.width = 0;
    meta.height = 0;
    meta.fps = 0.0;
    meta.video_codec = "None (Audio Only)".to_string();
    meta.audio_codec = codec.to_string();
    Ok(meta)
}
