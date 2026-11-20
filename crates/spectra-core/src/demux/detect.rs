//! Magic byte detection and file extension routing for media formats.

use crate::types::MediaFormat;

/// Detects media format from magic bytes at the beginning of the file/stream.
pub fn detect_format_from_bytes(bytes: &[u8]) -> MediaFormat {
    if bytes.len() >= 4 {
        // Matroska / WebM EBML Header: 0x1A 0x45 0xDF 0xA3
        if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
            // Check if DocType says "webm" or "matroska"
            if let Some(pos) = bytes.windows(4).position(|w| w == b"webm") {
                if pos < 100 {
                    return MediaFormat::Webm;
                }
            }
            return MediaFormat::Mkv;
        }

        // RIFF header: WAV or AVI
        if bytes.starts_with(b"RIFF") && bytes.len() >= 12 {
            let form = &bytes[8..12];
            if form == b"WAVE" {
                return MediaFormat::Wav;
            } else if form == b"AVI " || form == b"AVIX" {
                return MediaFormat::Avi;
            }
        }

        // FLAC: "fLaC"
        if bytes.starts_with(b"fLaC") {
            return MediaFormat::Flac;
        }

        // OGG: "OggS"
        if bytes.starts_with(b"OggS") {
            return MediaFormat::Ogg;
        }

        // MP3 with ID3v2 tag: "ID3"
        if bytes.starts_with(b"ID3") {
            return MediaFormat::Mp3;
        }

        // MP3 raw frame sync: 0xFF followed by 0xE0..0xFF (11 bits of 1s)
        if bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
            let layer = (bytes[1] >> 1) & 0x03;
            if layer == 0x01 {
                // Layer III
                return MediaFormat::Mp3;
            } else if layer == 0x00 {
                // AAC ADTS
                return MediaFormat::Aac;
            }
        }
    }

    // MP4 / MOV ISOBMFF: check for 'ftyp' at offset 4
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"qt  " || brand == b"moov" {
            return MediaFormat::Mov;
        }
        return MediaFormat::Mp4;
    }

    MediaFormat::Unknown
}

/// Detects media format based on file name or path extension.
pub fn detect_format_from_path(path: &str) -> MediaFormat {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".mp4") || lower.ends_with(".m4v") {
        MediaFormat::Mp4
    } else if lower.ends_with(".mkv") {
        MediaFormat::Mkv
    } else if lower.ends_with(".webm") {
        MediaFormat::Webm
    } else if lower.ends_with(".avi") {
        MediaFormat::Avi
    } else if lower.ends_with(".mov") {
        MediaFormat::Mov
    } else if lower.ends_with(".wav") {
        MediaFormat::Wav
    } else if lower.ends_with(".mp3") {
        MediaFormat::Mp3
    } else if lower.ends_with(".flac") {
        MediaFormat::Flac
    } else if lower.ends_with(".aac") {
        MediaFormat::Aac
    } else if lower.ends_with(".ogg") || lower.ends_with(".oga") {
        MediaFormat::Ogg
    } else {
        MediaFormat::Unknown
    }
}
