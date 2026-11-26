//! Magic byte detection and file extension routing for all common media formats.

use crate::types::MediaFormat;

pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mkv", "webm", "avi", "mov", "flv", "ts", "m2ts", "mts", "mpg", "mpeg",
    "vob", "wmv", "asf", "3gp", "3g2", "ogv", "divx", "rm", "rmvb", "vortexpart",
];

pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "oga", "opus", "aac", "m4a", "wma", "alac", "aiff", "aif",
    "ape", "wv", "ac3", "dts", "mka", "caf",
];

pub const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "vtt", "ass", "ssa", "sub"];
pub const PLAYLIST_EXTENSIONS: &[&str] = &["m3u", "m3u8", "pls"];

pub fn is_network_url(s: &str) -> bool {
    let lower = s.trim().to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("rtsp://")
        || lower.starts_with("rtmp://")
        || lower.starts_with("udp://")
        || lower.starts_with("rtp://")
        || lower.starts_with("mms://")
}

pub fn file_extension(path: &str) -> Option<String> {
    let clean = path.split('?').next().unwrap_or(path);
    let name = clean.rsplit(['/', '\\']).next().unwrap_or(clean);
    let dot = name.rfind('.')?;
    Some(name[dot + 1..].to_ascii_lowercase())
}

pub fn is_supported_media_path(path: &str) -> bool {
    if is_network_url(path) {
        return true;
    }
    if let Some(ext) = file_extension(path) {
        VIDEO_EXTENSIONS.contains(&ext.as_str()) || AUDIO_EXTENSIONS.contains(&ext.as_str())
    } else {
        false
    }
}

pub fn is_subtitle_path(path: &str) -> bool {
    file_extension(path)
        .map(|ext| SUBTITLE_EXTENSIONS.contains(&ext.as_str()))
        .unwrap_or(false)
}

pub fn is_playlist_path(path: &str) -> bool {
    file_extension(path)
        .map(|ext| PLAYLIST_EXTENSIONS.contains(&ext.as_str()))
        .unwrap_or(false)
}

/// Detects media format from magic bytes at the beginning of the file/stream.
pub fn detect_format_from_bytes(bytes: &[u8]) -> MediaFormat {
    if bytes.len() >= 4 {
        // Matroska / WebM EBML Header: 0x1A 0x45 0xDF 0xA3
        if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
            if let Some(pos) = bytes.windows(4).position(|w| w == b"webm") {
                if pos < 128 {
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

        // AIFF: FORM....AIFF / AIFC
        if bytes.starts_with(b"FORM") && bytes.len() >= 12 {
            let form = &bytes[8..12];
            if form == b"AIFF" || form == b"AIFC" {
                return MediaFormat::Aiff;
            }
        }

        // FLV: "FLV\x01"
        if bytes.starts_with(b"FLV") {
            return MediaFormat::Flv;
        }

        // ASF / WMV / WMA: 30 26 B2 75 8E 66 CF 11
        if bytes.len() >= 8 && bytes.starts_with(&[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11]) {
            return MediaFormat::Wmv;
        }

        // MPEG-PS / VOB: 00 00 01 BA
        if bytes.starts_with(&[0x00, 0x00, 0x01, 0xBA]) {
            return MediaFormat::Mpeg;
        }

        // MPEG-TS: 0x47 sync byte every 188 bytes
        if bytes.len() >= 377 && bytes[0] == 0x47 && bytes[188] == 0x47 && bytes[376] == 0x47 {
            return MediaFormat::Ts;
        }

        // FLAC: "fLaC"
        if bytes.starts_with(b"fLaC") {
            return MediaFormat::Flac;
        }

        // OGG: "OggS" (Opus vs Vorbis vs Theora)
        if bytes.starts_with(b"OggS") {
            if bytes.windows(8).take(128).any(|w| w == b"OpusHead") {
                return MediaFormat::Opus;
            }
            if bytes.windows(7).take(128).any(|w| w == b"\x80theora") {
                return MediaFormat::Ogv;
            }
            return MediaFormat::Ogg;
        }

        // MP3 with ID3v2 tag: "ID3"
        if bytes.starts_with(b"ID3") {
            return MediaFormat::Mp3;
        }

        // AC3 syncword: 0x0B 0x77
        if bytes[0] == 0x0B && bytes[1] == 0x77 {
            return MediaFormat::Ac3;
        }

        // MP3 raw frame sync or AAC ADTS
        if bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
            let layer = (bytes[1] >> 1) & 0x03;
            if layer == 0x01 {
                return MediaFormat::Mp3;
            } else if layer == 0x00 {
                return MediaFormat::Aac;
            }
        }
    }

    // MP4 / MOV / M4A / 3GP ISOBMFF: check for 'ftyp' at offset 4
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"qt  " || brand == b"moov" {
            return MediaFormat::Mov;
        } else if brand == b"M4A " || brand == b"M4B " {
            return MediaFormat::M4a;
        } else if brand.starts_with(b"3gp") || brand.starts_with(b"3g2") {
            return MediaFormat::ThreeGp;
        }
        return MediaFormat::Mp4;
    }

    MediaFormat::Unknown
}

/// Detects media format based on file name, URL, or path extension.
pub fn detect_format_from_path(path: &str) -> MediaFormat {
    if is_network_url(path) {
        if let Some(ext) = file_extension(path) {
            return match ext.as_str() {
                "mp4" | "m4v" => MediaFormat::Mp4,
                "mkv" => MediaFormat::Mkv,
                "webm" => MediaFormat::Webm,
                "mp3" => MediaFormat::Mp3,
                "flac" => MediaFormat::Flac,
                "ogg" | "oga" => MediaFormat::Ogg,
                "opus" => MediaFormat::Opus,
                "aac" => MediaFormat::Aac,
                "m4a" => MediaFormat::M4a,
                "ts" | "m3u8" => MediaFormat::Ts,
                _ => MediaFormat::NetworkStream,
            };
        }
        return MediaFormat::NetworkStream;
    }

    match file_extension(path).as_deref() {
        Some("mp4") | Some("m4v") => MediaFormat::Mp4,
        Some("mkv") | Some("mka") | Some("vortexpart") => MediaFormat::Mkv,
        Some("webm") => MediaFormat::Webm,
        Some("avi") | Some("divx") => MediaFormat::Avi,
        Some("mov") => MediaFormat::Mov,
        Some("flv") => MediaFormat::Flv,
        Some("ts") | Some("m2ts") | Some("mts") => MediaFormat::Ts,
        Some("mpg") | Some("mpeg") | Some("vob") => MediaFormat::Mpeg,
        Some("wmv") | Some("asf") | Some("rm") | Some("rmvb") => MediaFormat::Wmv,
        Some("3gp") | Some("3g2") => MediaFormat::ThreeGp,
        Some("ogv") => MediaFormat::Ogv,
        Some("wav") => MediaFormat::Wav,
        Some("mp3") => MediaFormat::Mp3,
        Some("flac") => MediaFormat::Flac,
        Some("aac") => MediaFormat::Aac,
        Some("m4a") => MediaFormat::M4a,
        Some("alac") => MediaFormat::Alac,
        Some("aiff") | Some("aif") => MediaFormat::Aiff,
        Some("ogg") | Some("oga") => MediaFormat::Ogg,
        Some("opus") => MediaFormat::Opus,
        Some("wma") => MediaFormat::Wma,
        Some("ac3") | Some("dts") => MediaFormat::Ac3,
        _ => MediaFormat::Unknown,
    }
}
