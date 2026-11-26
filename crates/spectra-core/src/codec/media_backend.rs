//! Real cross-platform media probing & decoding backend:
//! - Pure-Rust audio decoding via Symphonia (MP3, FLAC, WAV, OGG, Opus, AAC, M4A, ALAC, AIFF)
//! - Pure-Rust H.264 MP4 decoding via `mp4` + `rusty_h264-decoder`
//! - Universal FFmpeg/FFprobe streaming backend (4K/8K, HEVC, AV1, VP9, MKV, AVI, WebM, MOV, TS, WMV, HTTP/RTSP/HLS)
//! - Embedded & sidecar subtitle extraction + cover art decoding

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::demux::demux_file;
use crate::demux::detect::{detect_format_from_path, is_network_url, SUBTITLE_EXTENSIONS};
use crate::subtitles::SubtitleTrack;
use crate::types::{
    CoverArt, DecoderBackend, MediaFormat, MediaMetadata, StreamTrackInfo, VideoFrame,
};

pub const TARGET_AUDIO_RATE: u32 = 48000;
const AUDIO_RING_MAX_FRAMES: usize = 48000 / 2; // 500ms buffer
const VIDEO_QUEUE_MAX_FRAMES: usize = 8;

#[derive(Debug, Clone)]
pub enum CodecStatus {
    Ready,
    Downloading { progress_pct: u8, stage: String },
    Missing,
    Failed(String),
}

pub fn spectra_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local).join("Spectra");
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("Library/Application Support/Spectra");
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("spectra");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/spectra");
    }
    PathBuf::from(".spectra")
}

pub fn spectra_bin_dir() -> PathBuf {
    spectra_data_dir().join("bin")
}

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

pub fn find_ffmpeg_bin() -> Option<PathBuf> {
    let custom = spectra_bin_dir().join(exe_name("ffmpeg"));
    if custom.is_file() {
        return Some(custom);
    }
    if let Ok(sidecar) = ffmpeg_sidecar::paths::ffmpeg_path().canonicalize() {
        if sidecar.is_file() {
            return Some(sidecar);
        }
    }
    let sidecar_raw = ffmpeg_sidecar::paths::ffmpeg_path();
    if sidecar_raw.is_file() {
        return Some(sidecar_raw);
    }
    if ffmpeg_sidecar::command::ffmpeg_is_installed() {
        return Some(PathBuf::from("ffmpeg"));
    }
    // Check common macOS Homebrew / Linux / Windows locations
    for candidate in [
        "/opt/homebrew/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/usr/bin/ffmpeg",
        "C:\\ffmpeg\\bin\\ffmpeg.exe",
    ] {
        let p = Path::new(candidate);
        if p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    None
}

pub fn find_ffprobe_bin() -> Option<PathBuf> {
    let custom = spectra_bin_dir().join(exe_name("ffprobe"));
    if custom.is_file() {
        return Some(custom);
    }
    if let Ok(sidecar) = ffmpeg_sidecar::ffprobe::ffprobe_sidecar_path() {
        if sidecar.is_file() {
            return Some(sidecar);
        }
    }
    if ffmpeg_sidecar::ffprobe::ffprobe_is_installed() {
        return Some(PathBuf::from("ffprobe"));
    }
    for candidate in [
        "/opt/homebrew/bin/ffprobe",
        "/usr/local/bin/ffprobe",
        "/usr/bin/ffprobe",
        "C:\\ffmpeg\\bin\\ffprobe.exe",
    ] {
        let p = Path::new(candidate);
        if p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    None
}

/// Downloads universal FFmpeg binaries into `spectra_bin_dir()` in the background.
pub fn spawn_ffmpeg_downloader(status: Arc<Mutex<CodecStatus>>) {
    if find_ffmpeg_bin().is_some() {
        if let Ok(mut s) = status.lock() {
            *s = CodecStatus::Ready;
        }
        return;
    }

    {
        let mut s = status.lock().unwrap();
        if matches!(*s, CodecStatus::Downloading { .. }) {
            return;
        }
        *s = CodecStatus::Downloading {
            progress_pct: 1,
            stage: "Preparing Universal Codec Pack...".to_string(),
        };
    }

    thread::spawn(move || {
        use ffmpeg_sidecar::download::{
            download_ffmpeg_package_with_progress, ffmpeg_download_url, unpack_ffmpeg,
            FfmpegDownloadProgressEvent,
        };

        let bin_dir = spectra_bin_dir();
        if let Err(e) = std::fs::create_dir_all(&bin_dir) {
            if let Ok(mut s) = status.lock() {
                *s = CodecStatus::Failed(format!("Cannot create codec dir: {e}"));
            }
            return;
        }

        let url = match ffmpeg_download_url() {
            Ok(u) => u,
            Err(e) => {
                if let Ok(mut s) = status.lock() {
                    *s = CodecStatus::Failed(e.to_string());
                }
                return;
            }
        };

        let st_cb = Arc::clone(&status);
        let archive = match download_ffmpeg_package_with_progress(url, &bin_dir, move |ev| {
            if let Ok(mut s) = st_cb.lock() {
                match ev {
                    FfmpegDownloadProgressEvent::Starting => {
                        *s = CodecStatus::Downloading {
                            progress_pct: 2,
                            stage: "Connecting to FFmpeg release mirror...".to_string(),
                        };
                    }
                    FfmpegDownloadProgressEvent::Downloading {
                        total_bytes,
                        downloaded_bytes,
                    } => {
                        let pct = if total_bytes > 0 {
                            ((downloaded_bytes * 90) / total_bytes).clamp(3, 92) as u8
                        } else {
                            45
                        };
                        let mb_done = downloaded_bytes as f32 / (1024.0 * 1024.0);
                        let mb_tot = total_bytes as f32 / (1024.0 * 1024.0);
                        *s = CodecStatus::Downloading {
                            progress_pct: pct,
                            stage: if total_bytes > 0 {
                                format!("Downloading Codec Pack ({:.1} / {:.1} MB)...", mb_done, mb_tot)
                            } else {
                                format!("Downloading Codec Pack ({:.1} MB)...", mb_done)
                            },
                        };
                    }
                    FfmpegDownloadProgressEvent::UnpackingArchive => {
                        *s = CodecStatus::Downloading {
                            progress_pct: 95,
                            stage: "Unpacking Universal Codecs...".to_string(),
                        };
                    }
                    FfmpegDownloadProgressEvent::Done => {
                        *s = CodecStatus::Ready;
                    }
                }
            }
        }) {
            Ok(a) => a,
            Err(e) => {
                if let Ok(mut s) = status.lock() {
                    *s = CodecStatus::Failed(format!("Download error: {e}"));
                }
                return;
            }
        };

        if let Err(e) = unpack_ffmpeg(&archive, &bin_dir) {
            if let Ok(mut s) = status.lock() {
                *s = CodecStatus::Failed(format!("Unpack error: {e}"));
            }
            return;
        }
        let _ = std::fs::remove_file(&archive);

        if let Ok(mut s) = status.lock() {
            if find_ffmpeg_bin().is_some() {
                *s = CodecStatus::Ready;
            } else {
                *s = CodecStatus::Failed("FFmpeg binary not found after unpack".to_string());
            }
        }
    });
}

fn hide_window_cmd(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

/// Probes real metadata, streams, cover art, and duration for any file or URL.
pub fn probe_media(path: &str) -> MediaMetadata {
    let mut meta = MediaMetadata::default();
    meta.path = path.to_string();
    meta.is_live_file = true;
    meta.format = detect_format_from_path(path);
    meta.width = 0;
    meta.height = 0;
    meta.duration_ms = 0;
    meta.fps = 0.0;
    meta.bitrate_kbps = 0;
    meta.video_codec = if meta.format.is_audio_only() {
        "None (Audio Only)".to_string()
    } else {
        "Video Stream".to_string()
    };
    meta.audio_codec = "None".to_string();
    meta.title = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_string();
    meta.artist = if is_network_url(path) {
        "Network Media Stream".to_string()
    } else {
        "Local Media File".to_string()
    };

    // Image shortcut (.png, .jpg, .jpeg, .bmp, .webp, .gif)
    if is_image_path(path) {
        if let Ok(img) = image::open(path) {
            meta.width = img.width();
            meta.height = img.height();
            meta.duration_ms = 10_000;
            meta.fps = 1.0;
            meta.video_codec = "Static Image Frame".to_string();
            meta.audio_codec = "None".to_string();
            meta.pixel_format = "rgba8".to_string();
            meta.backend = DecoderBackend::PureRustH264;
            return meta;
        }
    }

    // 1. Try FFprobe for exact multi-stream metadata
    if let Some(ffprobe) = find_ffprobe_bin() {
        if let Some(probed) = probe_with_ffprobe(&ffprobe, path, &meta) {
            meta = probed;
        }
    } else if let Some(ffmpeg) = find_ffmpeg_bin() {
        if let Some(probed) = probe_with_ffmpeg_banner(&ffmpeg, path, &meta) {
            meta = probed;
        }
    }

    // 2. Also probe local files with pure-Rust Symphonia for ID3/Vorbis/MP4 tags & embedded Cover Art
    if !is_network_url(path) {
        enrich_with_symphonia(path, &mut meta);
    }

    // 3. If still no video dimensions on local MP4/MOV/MKV/AVI/WAV/FLAC/MP3, use pure-Rust demuxers
    if !is_network_url(path) && (meta.duration_ms == 0 || (meta.width == 0 && !meta.format.is_audio_only())) {
        if let Ok(mut f) = File::open(path) {
            let mut head = vec![0u8; 65536];
            if let Ok(n) = f.read(&mut head) {
                head.truncate(n);
                if let Ok(d_meta) = demux_file(path, &head) {
                    if meta.format == MediaFormat::Unknown {
                        meta.format = d_meta.format;
                    }
                    if meta.width == 0 && d_meta.width > 0 && !meta.format.is_audio_only() {
                        meta.width = d_meta.width;
                        meta.height = d_meta.height;
                    }
                    if meta.duration_ms == 0 {
                        meta.duration_ms = d_meta.duration_ms;
                    }
                    if meta.video_codec.is_empty() || meta.video_codec.contains("High Profile") {
                        meta.video_codec = d_meta.video_codec;
                    }
                }
            }
        }
        // Try mp4 crate for exact MP4 width/height/duration when ffprobe isn't installed
        if matches!(meta.format, MediaFormat::Mp4 | MediaFormat::Mov | MediaFormat::ThreeGp) {
            if let Ok(f) = File::open(path) {
                if let Ok(size) = f.metadata().map(|m| m.len()) {
                    if let Ok(mp4_reader) = mp4::Mp4Reader::read_header(BufReader::new(f), size) {
                        let dur_ms = mp4_reader.duration().as_millis() as u64;
                        if dur_ms > 0 {
                            meta.duration_ms = dur_ms;
                        }
                        for t in mp4_reader.tracks().values() {
                            if matches!(t.track_type(), Ok(mp4::TrackType::Video)) {
                                meta.width = t.width() as u32;
                                meta.height = t.height() as u32;
                                let fps = t.frame_rate();
                                if fps > 0.5 && fps < 240.0 {
                                    meta.fps = fps as f32;
                                }
                                let stsd = &t.trak.mdia.minf.stbl.stsd;
                                if stsd.avc1.is_some() {
                                    meta.video_codec = "H.264 / AVC".to_string();
                                } else if stsd.hev1.is_some() {
                                    meta.video_codec = "HEVC / H.265".to_string();
                                } else if stsd.vp09.is_some() {
                                    meta.video_codec = "VP9".to_string();
                                } else if let Some(det) = detect_mp4_codec_tag(path) {
                                    meta.video_codec = det.to_string();
                                } else if let Ok(mp) = t.media_type() {
                                    meta.video_codec = format!("{:?}", mp);
                                }
                            } else if matches!(t.track_type(), Ok(mp4::TrackType::Audio)) {
                                if meta.audio_codec == "None" {
                                    meta.audio_codec = format!(
                                        "{:?} {}Hz",
                                        t.media_type().unwrap_or(mp4::MediaType::AAC),
                                        meta.sample_rate
                                    );
                                }
                                if meta.audio_tracks.is_empty() {
                                    meta.audio_tracks.push(StreamTrackInfo {
                                        index: 0,
                                        stream_index: 0,
                                        codec: meta.audio_codec.clone(),
                                        language: "und".to_string(),
                                        title: "Audio Track 1".to_string(),
                                        channels: meta.channels.max(2),
                                        sample_rate: meta.sample_rate.max(48000),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !meta.format.is_audio_only() && meta.fps <= 0.0 {
        meta.fps = 30.0;
    }
    if meta.format.is_audio_only() {
        meta.width = 0;
        meta.height = 0;
        meta.fps = 0.0;
        meta.video_codec = "None (Audio Only)".to_string();
    } else if meta.audio_codec != "None" && meta.audio_tracks.is_empty() {
        meta.audio_tracks.push(StreamTrackInfo {
            index: 0,
            stream_index: 0,
            codec: meta.audio_codec.clone(),
            language: "und".to_string(),
            title: "Audio Track 1".to_string(),
            channels: meta.channels.max(1),
            sample_rate: meta.sample_rate.max(8000),
        });
    }

    meta
}

fn probe_with_ffprobe(ffprobe: &Path, path: &str, base: &MediaMetadata) -> Option<MediaMetadata> {
    let mut cmd = Command::new(ffprobe);
    hide_window_cmd(&mut cmd);
    let out = cmd
        .args([
            "-v",
            "quiet",
            "-show_entries",
            "format=duration,bit_rate,format_name:format_tags=title,artist,album:stream=index,codec_type,codec_name,width,height,r_frame_rate,avg_frame_rate,pix_fmt,sample_rate,channels:stream_tags=language,title",
            "-of",
            "default=noprint_wrappers=1",
            path,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut meta = base.clone();
    meta.width = 0;
    meta.height = 0;
    meta.duration_ms = 0;
    meta.fps = 0.0;
    meta.video_codec = "None".to_string();
    meta.audio_codec = "None".to_string();
    meta.audio_tracks.clear();
    meta.subtitle_tracks.clear();

    #[derive(Default)]
    struct StreamAcc {
        index: usize,
        codec_type: String,
        codec_name: String,
        width: u32,
        height: u32,
        fps: f32,
        pix_fmt: String,
        sample_rate: u32,
        channels: u8,
        lang: String,
        title: String,
    }

    let mut streams: Vec<StreamAcc> = Vec::new();
    let mut cur = StreamAcc::default();
    let mut in_stream = false;

    for raw in text.lines() {
        let line = raw.trim();
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k {
            "index" => {
                if in_stream {
                    streams.push(cur);
                    cur = StreamAcc::default();
                }
                in_stream = true;
                cur.index = v.parse().unwrap_or(0);
            }
            "codec_type" if in_stream => cur.codec_type = v.to_string(),
            "codec_name" if in_stream => cur.codec_name = v.to_uppercase(),
            "width" if in_stream => cur.width = v.parse().unwrap_or(0),
            "height" if in_stream => cur.height = v.parse().unwrap_or(0),
            "pix_fmt" if in_stream => cur.pix_fmt = v.to_string(),
            "sample_rate" if in_stream => cur.sample_rate = v.parse().unwrap_or(48000),
            "channels" if in_stream => cur.channels = v.parse().unwrap_or(2),
            "avg_frame_rate" | "r_frame_rate" if in_stream => {
                if cur.fps == 0.0 {
                    cur.fps = parse_rate_fraction(v);
                }
            }
            "TAG:language" if in_stream => cur.lang = v.to_string(),
            "TAG:title" if in_stream => cur.title = v.to_string(),
            "duration" if !in_stream => {
                if let Ok(sec) = v.parse::<f64>() {
                    meta.duration_ms = (sec * 1000.0).max(0.0) as u64;
                }
            }
            "bit_rate" if !in_stream => {
                if let Ok(bps) = v.parse::<u64>() {
                    meta.bitrate_kbps = (bps / 1000) as u32;
                }
            }
            "TAG:title" if !in_stream && !v.is_empty() => meta.title = v.to_string(),
            "TAG:artist" if !in_stream && !v.is_empty() => meta.artist = v.to_string(),
            "TAG:album" if !in_stream && !v.is_empty() => meta.album = v.to_string(),
            _ => {}
        }
    }
    if in_stream {
        streams.push(cur);
    }

    let mut a_idx = 0usize;
    let mut s_idx = 0usize;
    let mut has_video = false;

    for st in streams {
        match st.codec_type.as_str() {
            "video" if st.width > 0 && st.height > 0 && !has_video => {
                has_video = true;
                meta.width = st.width;
                meta.height = st.height;
                meta.fps = if st.fps > 0.1 { st.fps } else { 30.0 };
                meta.video_codec = st.codec_name;
                meta.pixel_format = if st.pix_fmt.is_empty() {
                    "yuv420p".to_string()
                } else {
                    st.pix_fmt
                };
            }
            "audio" => {
                if a_idx == 0 {
                    meta.audio_codec = format!(
                        "{} {}ch {}Hz",
                        st.codec_name,
                        st.channels.max(1),
                        st.sample_rate
                    );
                    meta.sample_rate = st.sample_rate.max(8000);
                    meta.channels = st.channels.max(1);
                }
                meta.audio_tracks.push(StreamTrackInfo {
                    index: a_idx,
                    stream_index: st.index,
                    codec: st.codec_name,
                    language: if st.lang.is_empty() { "und".to_string() } else { st.lang },
                    title: st.title,
                    channels: st.channels.max(1),
                    sample_rate: st.sample_rate.max(8000),
                });
                a_idx += 1;
            }
            "subtitle" => {
                meta.subtitle_tracks.push(StreamTrackInfo {
                    index: s_idx,
                    stream_index: st.index,
                    codec: st.codec_name,
                    language: if st.lang.is_empty() { "und".to_string() } else { st.lang },
                    title: st.title,
                    channels: 0,
                    sample_rate: 0,
                });
                s_idx += 1;
            }
            _ => {}
        }
    }

    if !has_video {
        meta.video_codec = "None (Audio Only)".to_string();
    }
    meta.backend = DecoderBackend::FfmpegUniversal;
    Some(meta)
}

fn probe_with_ffmpeg_banner(ffmpeg: &Path, path: &str, base: &MediaMetadata) -> Option<MediaMetadata> {
    let mut cmd = Command::new(ffmpeg);
    hide_window_cmd(&mut cmd);
    let out = cmd.args(["-hide_banner", "-i", path]).output().ok()?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut meta = base.clone();
    meta.width = 0;
    meta.height = 0;
    meta.duration_ms = 0;
    meta.fps = 0.0;
    meta.audio_tracks.clear();
    meta.subtitle_tracks.clear();

    let mut a_idx = 0usize;
    let mut s_idx = 0usize;

    for line in stderr.lines() {
        let t = line.trim();
        if let Some(pos) = t.find("Duration:") {
            let after = t[pos + 9..].trim();
            if let Some(dur_tok) = after.split(',').next() {
                if let Some(ms) = parse_ffmpeg_duration(dur_tok.trim()) {
                    meta.duration_ms = ms;
                }
            }
            if let Some(br_pos) = t.find("bitrate:") {
                let br_str = t[br_pos + 8..].trim();
                if let Some(num) = br_str.split_whitespace().next() {
                    if let Ok(kb) = num.parse::<u32>() {
                        meta.bitrate_kbps = kb;
                    }
                }
            }
        } else if t.starts_with("Stream #") {
            if t.contains("Video:") && meta.width == 0 {
                let after = t.split("Video:").nth(1).unwrap_or("");
                let parts: Vec<&str> = after.split(',').map(|s| s.trim()).collect();
                if let Some(c) = parts.first() {
                    meta.video_codec = c.split_whitespace().next().unwrap_or("Video").to_uppercase();
                }
                for p in &parts {
                    if let Some((w, h)) = parse_wxh(p) {
                        meta.width = w;
                        meta.height = h;
                    }
                    if p.ends_with("fps") {
                        if let Some(n) = p.split_whitespace().next() {
                            if let Ok(f) = n.parse::<f32>() {
                                meta.fps = f;
                            }
                        }
                    }
                }
            } else if t.contains("Audio:") {
                let after = t.split("Audio:").nth(1).unwrap_or("");
                let parts: Vec<&str> = after.split(',').map(|s| s.trim()).collect();
                let codec = parts
                    .first()
                    .and_then(|c| c.split_whitespace().next())
                    .unwrap_or("Audio")
                    .to_uppercase();
                let mut sr = 48000u32;
                let mut ch = 2u8;
                for p in &parts {
                    if p.ends_with("Hz") {
                        if let Some(n) = p.split_whitespace().next() {
                            sr = n.parse().unwrap_or(48000);
                        }
                    } else if p.contains("mono") {
                        ch = 1;
                    } else if p.contains("5.1") {
                        ch = 6;
                    }
                }
                if a_idx == 0 {
                    meta.audio_codec = format!("{} {}ch {}Hz", codec, ch, sr);
                    meta.sample_rate = sr;
                    meta.channels = ch;
                }
                let lang = extract_stream_lang(t);
                meta.audio_tracks.push(StreamTrackInfo {
                    index: a_idx,
                    stream_index: a_idx,
                    codec,
                    language: lang,
                    title: String::new(),
                    channels: ch,
                    sample_rate: sr,
                });
                a_idx += 1;
            } else if t.contains("Subtitle:") {
                let after = t.split("Subtitle:").nth(1).unwrap_or("");
                let codec = after.split(',').next().unwrap_or("SUB").trim().to_uppercase();
                let lang = extract_stream_lang(t);
                meta.subtitle_tracks.push(StreamTrackInfo {
                    index: s_idx,
                    stream_index: s_idx,
                    codec,
                    language: lang,
                    title: String::new(),
                    channels: 0,
                    sample_rate: 0,
                });
                s_idx += 1;
            }
        }
    }

    meta.backend = DecoderBackend::FfmpegUniversal;
    Some(meta)
}

fn enrich_with_symphonia(path: &str, meta: &mut MediaMetadata) {
    use symphonia::core::codecs::CodecParameters;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::{MetadataOptions, StandardTag};

    let Ok(file) = File::open(path) else {
        return;
    };
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = symphonia::core::formats::probe::Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let Ok(mut format) = symphonia::default::get_probe().probe(
        &hint,
        mss,
        FormatOptions::default(),
        MetadataOptions::default(),
    ) else {
        return;
    };

    if meta.duration_ms == 0 {
        let info = format.media_info();
        if let (Some(tb), Some(dur)) = (info.time_base, info.duration) {
            if let Some(t) = tb.calc_duration(dur) {
                meta.duration_ms = (t.as_nanos().max(0) / 1_000_000) as u64;
            }
        }
    }

    if let Some(track) = format.default_track(TrackType::Audio) {
        if let Some(CodecParameters::Audio(ap)) = &track.codec_params {
            if meta.sample_rate == 48000 || meta.sample_rate == 0 {
                if let Some(sr) = ap.sample_rate {
                    meta.sample_rate = sr;
                }
            }
            if let Some(ch) = &ap.channels {
                meta.channels = ch.count() as u8;
            }
            if meta.audio_codec == "None" || meta.audio_codec.contains("FLAC 24-bit 96kHz") {
                meta.audio_codec = format!(
                    "{}Hz {}ch",
                    ap.sample_rate.unwrap_or(48000),
                    ap.channels.as_ref().map(|c| c.count()).unwrap_or(2)
                );
            }
        }
    }

    if let Some(rev) = format.metadata().current() {
        for tag in &rev.media.tags {
            if let Some(std_tag) = &tag.std {
                match std_tag {
                    StandardTag::TrackTitle(s) if !s.is_empty() => meta.title = s.to_string(),
                    StandardTag::Artist(s) | StandardTag::AlbumArtist(s) if !s.is_empty() => {
                        meta.artist = s.to_string()
                    }
                    StandardTag::Album(s) if !s.is_empty() => meta.album = s.to_string(),
                    _ => {}
                }
            }
        }
        if meta.cover_art.is_none() {
            if let Some(vis) = rev.media.visuals.first() {
                if let Ok(img) = image::load_from_memory(&vis.data) {
                    let rgba = img.resize(320, 320, image::imageops::FilterType::Triangle).to_rgba8();
                    let (cw, ch) = rgba.dimensions();
                    let pixels = rgba
                        .chunks_exact(4)
                        .map(|p| {
                            0xFF000000
                                | ((p[0] as u32) << 16)
                                | ((p[1] as u32) << 8)
                                | (p[2] as u32)
                        })
                        .collect();
                    meta.cover_art = Some(CoverArt {
                        width: cw,
                        height: ch,
                        pixels,
                    });
                }
            }
        }
    }
}

/// Discovers sidecar subtitle files (`.srt`, `.vtt`, `.ass`, `.ssa`, `.sub`) in the same folder.
pub fn discover_sidecar_subtitles(media_path: &str) -> Vec<String> {
    if is_network_url(media_path) {
        return Vec::new();
    }
    let p = Path::new(media_path);
    let Some(parent) = p.parent() else {
        return Vec::new();
    };
    let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
        return Vec::new();
    };
    let stem_lower = stem.to_ascii_lowercase();
    let mut found = Vec::new();

    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let ep = entry.path();
            if !ep.is_file() {
                continue;
            }
            let ext = ep
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default();
            if !SUBTITLE_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }
            let estem = ep
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if estem == stem_lower || estem.starts_with(&format!("{stem_lower}.")) {
                found.push(ep.to_string_lossy().to_string());
            }
        }
    }
    found.sort();
    found
}

/// Extracts an embedded subtitle stream from MKV/MP4 via FFmpeg into a `SubtitleTrack`.
pub fn extract_embedded_subtitle(path: &str, sub_idx: usize, title: &str, lang: &str) -> Option<SubtitleTrack> {
    let ffmpeg = find_ffmpeg_bin()?;
    let mut cmd = Command::new(ffmpeg);
    hide_window_cmd(&mut cmd);
    let map_arg = format!("0:s:{sub_idx}");
    let out = cmd
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            path,
            "-map",
            &map_arg,
            "-f",
            "srt",
            "pipe:1",
        ])
        .output()
        .ok()?;
    if !out.status.success() || out.stdout.is_empty() {
        return None;
    }
    let content = String::from_utf8_lossy(&out.stdout);
    let mut track = SubtitleTrack::parse_srt(&content, title);
    track.language = lang.to_string();
    if track.cues.is_empty() {
        None
    } else {
        Some(track)
    }
}

/// Shared thread-safe audio ring & master clock state.
pub struct AudioSharedBuffer {
    pub ring: Mutex<VecDeque<f32>>, // interleaved stereo @ TARGET_AUDIO_RATE
    pub played_frames: AtomicU64,
    pub seek_origin_ms: AtomicU64,
    pub is_playing: AtomicBool,
    pub eof: AtomicBool,
    pub speed_bits: AtomicU32,
}

impl Default for AudioSharedBuffer {
    fn default() -> Self {
        Self {
            ring: Mutex::new(VecDeque::with_capacity(AUDIO_RING_MAX_FRAMES * 2)),
            played_frames: AtomicU64::new(0),
            seek_origin_ms: AtomicU64::new(0),
            is_playing: AtomicBool::new(false),
            eof: AtomicBool::new(false),
            speed_bits: AtomicU32::new(1.0_f32.to_bits()),
        }
    }
}

impl AudioSharedBuffer {
    pub fn clear_for_seek(&self, target_ms: u64) {
        if let Ok(mut q) = self.ring.lock() {
            q.clear();
        }
        self.played_frames.store(0, Ordering::Release);
        self.seek_origin_ms.store(target_ms, Ordering::Release);
        self.eof.store(false, Ordering::Release);
    }

    pub fn current_audio_ms(&self) -> u64 {
        let frames = self.played_frames.load(Ordering::Acquire);
        let speed = f32::from_bits(self.speed_bits.load(Ordering::Acquire)).clamp(0.25, 4.0);
        let advanced_ms = ((frames as f64 * 1000.0 / TARGET_AUDIO_RATE as f64) * speed as f64) as u64;
        self.seek_origin_ms.load(Ordering::Acquire) + advanced_ms
    }
}

/// Live background decoding session for an opened media file/stream.
pub struct ActiveMediaSession {
    pub path: String,
    pub metadata: MediaMetadata,
    pub audio_shared: Arc<AudioSharedBuffer>,
    pub video_queue: Arc<Mutex<VecDeque<VideoFrame>>>,
    pub latest_frame: Option<VideoFrame>,
    pub generation: Arc<AtomicU64>,
    pub active_audio_track: Option<usize>,
    pub has_audio_stream: bool,
    pub has_video_stream: bool,
    pub video_uses_ffmpeg: bool,
    pub video_eof: Arc<AtomicBool>,
    pub decode_width: u32,
    pub decode_height: u32,
    pub audio_backend_name: String,
    pub video_backend_name: String,
    pub last_error: Arc<Mutex<Option<String>>>,
}

pub fn mp4_has_avc1(path: &str) -> bool {
    let Ok(f) = File::open(path) else {
        return false;
    };
    let Ok(size) = f.metadata().map(|m| m.len()) else {
        return false;
    };
    let Ok(mp4_reader) = mp4::Mp4Reader::read_header(BufReader::new(f), size) else {
        return false;
    };
    mp4_reader
        .tracks()
        .values()
        .any(|t| matches!(t.track_type(), Ok(mp4::TrackType::Video)) && t.trak.mdia.minf.stbl.stsd.avc1.is_some())
}

fn detect_mp4_codec_tag(path: &str) -> Option<&'static str> {
    use std::io::{Seek, SeekFrom};
    let mut f = File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let chunk_sz = (len.min(512 * 1024)) as usize;
    let mut buf = vec![0u8; chunk_sz];
    let n1 = f.read(&mut buf).unwrap_or(0);
    if len > chunk_sz as u64 {
        let _ = f.seek(SeekFrom::End(-(chunk_sz as i64)));
        let mut tail = vec![0u8; chunk_sz];
        let n2 = f.read(&mut tail).unwrap_or(0);
        buf.truncate(n1);
        buf.extend_from_slice(&tail[..n2]);
    } else {
        buf.truncate(n1);
    }
    if buf.windows(4).any(|w| w == b"av01") {
        Some("AV1")
    } else if buf.windows(4).any(|w| w == b"vp09") {
        Some("VP9")
    } else if buf.windows(4).any(|w| w == b"hvc1" || w == b"hev1") {
        Some("HEVC / H.265")
    } else if buf.windows(4).any(|w| w == b"avc1") {
        Some("H.264 / AVC")
    } else {
        None
    }
}

impl Drop for ActiveMediaSession {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::Release);
    }
}

impl ActiveMediaSession {
    pub fn start(
        path: String,
        mut metadata: MediaMetadata,
        seek_ms: u64,
        speed: f32,
        audio_track: Option<usize>,
        max_decode_w: u32,
    ) -> Self {
        let generation = Arc::new(AtomicU64::new(1));
        let audio_shared = Arc::new(AudioSharedBuffer::default());
        audio_shared.seek_origin_ms.store(seek_ms, Ordering::Release);
        audio_shared.speed_bits.store(speed.to_bits(), Ordering::Release);
        audio_shared.is_playing.store(true, Ordering::Release);

        let video_queue = Arc::new(Mutex::new(VecDeque::with_capacity(VIDEO_QUEUE_MAX_FRAMES)));
        let video_eof = Arc::new(AtomicBool::new(false));
        let last_error = Arc::new(Mutex::new(None));

        let is_img = is_image_path(&path);
        let has_video = !metadata.format.is_audio_only() && metadata.width > 0 && metadata.height > 0;
        let has_audio = !is_img && audio_track.is_some() && !metadata.audio_codec.eq_ignore_ascii_case("None");

        let (dec_w, dec_h) = if has_video {
            compute_decode_dims(metadata.width, metadata.height, max_decode_w)
        } else {
            (0, 0)
        };

        let ffmpeg_bin = find_ffmpeg_bin();
        let mut audio_backend_name = "None".to_string();
        let mut video_backend_name = "None".to_string();

        // Start Audio Decoder
        if has_audio {
            let prefer_symphonia = !is_network_url(&path)
                && audio_track == Some(0)
                && can_try_symphonia(&path, &metadata);

            if prefer_symphonia || ffmpeg_bin.is_none() {
                audio_backend_name = "Pure-Rust Symphonia".to_string();
                spawn_symphonia_audio_thread(
                    path.clone(),
                    seek_ms,
                    speed,
                    Arc::clone(&audio_shared),
                    Arc::clone(&generation),
                    1,
                    ffmpeg_bin.clone(),
                    audio_track.unwrap_or(0),
                    Arc::clone(&last_error),
                );
            } else if let Some(ref ff) = ffmpeg_bin {
                audio_backend_name = "FFmpeg Audio Pipe".to_string();
                spawn_ffmpeg_audio_thread(
                    ff.clone(),
                    path.clone(),
                    seek_ms,
                    speed,
                    audio_track.unwrap_or(0),
                    Arc::clone(&audio_shared),
                    Arc::clone(&generation),
                    1,
                );
            }
        } else {
            audio_shared.eof.store(true, Ordering::Release);
        }

        let mut video_uses_ffmpeg = false;
        // Start Video Decoder
        if is_img {
            video_backend_name = "Pure-Rust Image".to_string();
            if let Ok(img) = image::open(&path) {
                let rgba = img.to_rgba8();
                let (iw, ih) = rgba.dimensions();
                let pixels = rgba
                    .chunks_exact(4)
                    .map(|p| {
                        0xFF000000
                            | ((p[0] as u32) << 16)
                            | ((p[1] as u32) << 8)
                            | (p[2] as u32)
                    })
                    .collect();
                if let Ok(mut q) = video_queue.lock() {
                    q.push_back(VideoFrame {
                        width: iw,
                        height: ih,
                        timestamp_ms: 0,
                        buffer: pixels,
                    });
                }
            }
            video_eof.store(true, Ordering::Release);
        } else if has_video {
            if let Some(ref ff) = ffmpeg_bin {
                video_uses_ffmpeg = true;
                video_backend_name = "FFmpeg Universal Video".to_string();
                metadata.backend = if audio_backend_name.contains("Symphonia") {
                    DecoderBackend::HybridSymphoniaFfmpeg
                } else {
                    DecoderBackend::FfmpegUniversal
                };
                spawn_ffmpeg_video_thread(
                    ff.clone(),
                    path.clone(),
                    seek_ms,
                    speed,
                    dec_w,
                    dec_h,
                    metadata.fps.max(1.0),
                    Arc::clone(&video_queue),
                    Arc::clone(&video_eof),
                    Arc::clone(&generation),
                    1,
                    Arc::clone(&last_error),
                );
            } else if matches!(metadata.format, MediaFormat::Mp4 | MediaFormat::Mov) && mp4_has_avc1(&path) {
                video_backend_name = "Pure-Rust H.264 Decoder".to_string();
                metadata.backend = DecoderBackend::PureRustH264;
                spawn_pure_rust_h264_thread(
                    path.clone(),
                    seek_ms,
                    Arc::clone(&video_queue),
                    Arc::clone(&video_eof),
                    Arc::clone(&generation),
                    1,
                    Arc::clone(&last_error),
                );
            } else {
                metadata.backend = DecoderBackend::FfmpegUniversal;
                video_backend_name = "Installing Universal Codecs...".to_string();
                if let Ok(mut er) = last_error.lock() {
                    *er = Some(format!(
                        "Codec {} requires Universal Codec Pack (FFmpeg)",
                        metadata.video_codec
                    ));
                }
                video_eof.store(true, Ordering::Release);
            }
        } else {
            metadata.backend = DecoderBackend::PureRustSymphonia;
            video_eof.store(true, Ordering::Release);
        }

        Self {
            path,
            metadata,
            audio_shared,
            video_queue,
            latest_frame: None,
            generation,
            active_audio_track: audio_track,
            has_audio_stream: has_audio,
            has_video_stream: has_video,
            video_uses_ffmpeg,
            video_eof,
            decode_width: dec_w,
            decode_height: dec_h,
            audio_backend_name,
            video_backend_name,
            last_error,
        }
    }

    pub fn restart_at(&mut self, seek_ms: u64, speed: f32, audio_track: Option<usize>, max_decode_w: u32) {
        let gen_val = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.audio_shared.clear_for_seek(seek_ms);
        self.audio_shared.speed_bits.store(speed.to_bits(), Ordering::Release);
        if let Ok(mut q) = self.video_queue.lock() {
            q.clear();
        }
        self.latest_frame = None;
        self.video_eof.store(false, Ordering::Release);
        self.active_audio_track = audio_track;
        if let Ok(mut er) = self.last_error.lock() {
            *er = None;
        }

        let is_img = is_image_path(&self.path);
        let ffmpeg_bin = find_ffmpeg_bin();
        let has_audio = !is_img && audio_track.is_some() && !self.metadata.audio_codec.eq_ignore_ascii_case("None");
        self.has_audio_stream = has_audio;

        if has_audio {
            let prefer_symphonia = !is_network_url(&self.path)
                && audio_track == Some(0)
                && can_try_symphonia(&self.path, &self.metadata);
            if prefer_symphonia || ffmpeg_bin.is_none() {
                spawn_symphonia_audio_thread(
                    self.path.clone(),
                    seek_ms,
                    speed,
                    Arc::clone(&self.audio_shared),
                    Arc::clone(&self.generation),
                    gen_val,
                    ffmpeg_bin.clone(),
                    audio_track.unwrap_or(0),
                    Arc::clone(&self.last_error),
                );
            } else if let Some(ref ff) = ffmpeg_bin {
                spawn_ffmpeg_audio_thread(
                    ff.clone(),
                    self.path.clone(),
                    seek_ms,
                    speed,
                    audio_track.unwrap_or(0),
                    Arc::clone(&self.audio_shared),
                    Arc::clone(&self.generation),
                    gen_val,
                );
            }
        } else {
            self.audio_shared.eof.store(true, Ordering::Release);
        }

        self.video_uses_ffmpeg = false;
        if self.has_video_stream && !is_img {
            let (dec_w, dec_h) = compute_decode_dims(self.metadata.width, self.metadata.height, max_decode_w);
            self.decode_width = dec_w;
            self.decode_height = dec_h;
            if let Some(ref ff) = ffmpeg_bin {
                self.video_uses_ffmpeg = true;
                self.video_backend_name = "FFmpeg Universal Video".to_string();
                self.metadata.backend = if self.audio_backend_name.contains("Symphonia") {
                    DecoderBackend::HybridSymphoniaFfmpeg
                } else {
                    DecoderBackend::FfmpegUniversal
                };
                spawn_ffmpeg_video_thread(
                    ff.clone(),
                    self.path.clone(),
                    seek_ms,
                    speed,
                    dec_w,
                    dec_h,
                    self.metadata.fps.max(1.0),
                    Arc::clone(&self.video_queue),
                    Arc::clone(&self.video_eof),
                    Arc::clone(&self.generation),
                    gen_val,
                    Arc::clone(&self.last_error),
                );
            } else if matches!(self.metadata.format, MediaFormat::Mp4 | MediaFormat::Mov) && mp4_has_avc1(&self.path) {
                spawn_pure_rust_h264_thread(
                    self.path.clone(),
                    seek_ms,
                    Arc::clone(&self.video_queue),
                    Arc::clone(&self.video_eof),
                    Arc::clone(&self.generation),
                    gen_val,
                    Arc::clone(&self.last_error),
                );
            } else {
                self.video_eof.store(true, Ordering::Release);
            }
        }
    }

    /// Pops frames up to `target_ms` and updates `latest_frame`. Returns (advanced, dropped_count).
    pub fn advance_video_to(&mut self, target_ms: u64) -> (bool, u64) {
        let mut updated = false;
        let mut dropped = 0u64;
        if let Ok(mut q) = self.video_queue.lock() {
            while let Some(front) = q.front() {
                if front.timestamp_ms <= target_ms + 18 {
                    let f = q.pop_front().unwrap();
                    if self.latest_frame.is_some() && front_behind(&f, target_ms) {
                        dropped += 1;
                    }
                    self.latest_frame = Some(f);
                    updated = true;
                } else {
                    break;
                }
            }
        }
        (updated, dropped)
    }

    pub fn step_one_frame(&mut self) -> Option<u64> {
        if let Ok(mut q) = self.video_queue.lock() {
            if let Some(f) = q.pop_front() {
                let ts = f.timestamp_ms;
                self.latest_frame = Some(f);
                return Some(ts);
            }
        }
        None
    }
}

fn front_behind(f: &VideoFrame, target_ms: u64) -> bool {
    target_ms.saturating_sub(f.timestamp_ms) > 45
}

fn can_try_symphonia(path: &str, meta: &MediaMetadata) -> bool {
    if is_network_url(path) {
        return false;
    }
    let ac = meta.audio_codec.to_ascii_lowercase();
    if ac.contains("ac3") || ac.contains("eac3") || ac.contains("dts") || ac.contains("truehd") || ac.contains("wma") {
        return false;
    }
    matches!(
        meta.format,
        MediaFormat::Mp3
            | MediaFormat::Flac
            | MediaFormat::Wav
            | MediaFormat::Ogg
            | MediaFormat::Opus
            | MediaFormat::Aac
            | MediaFormat::M4a
            | MediaFormat::Alac
            | MediaFormat::Aiff
            | MediaFormat::Mp4
            | MediaFormat::Mov
            | MediaFormat::Mkv
            | MediaFormat::Webm
    )
}

fn compute_decode_dims(src_w: u32, src_h: u32, max_w: u32) -> (u32, u32) {
    if src_w == 0 || src_h == 0 {
        return (1280, 720);
    }
    let cap_w = max_w.max(640);
    let (mut w, mut h) = if src_w <= cap_w {
        (src_w, src_h)
    } else {
        let ratio = cap_w as f64 / src_w as f64;
        (cap_w, ((src_h as f64 * ratio).round() as u32).max(2))
    };
    w &= !1;
    h &= !1;
    (w.max(2), h.max(2))
}

#[allow(clippy::too_many_arguments)]
fn spawn_symphonia_audio_thread(
    path: String,
    seek_ms: u64,
    speed: f32,
    shared: Arc<AudioSharedBuffer>,
    gen: Arc<AtomicU64>,
    my_gen: u64,
    ffmpeg_fallback: Option<PathBuf>,
    audio_track_idx: usize,
    last_error: Arc<Mutex<Option<String>>>,
) {
    thread::spawn(move || {
        let res = run_symphonia_decode(&path, seek_ms, &shared, &gen, my_gen);
        if let Err(err_msg) = res {
            if gen.load(Ordering::Acquire) != my_gen {
                return;
            }
            // Automatically fall back to FFmpeg audio pipe if codec is unsupported in Symphonia
            if let Some(ff) = ffmpeg_fallback {
                spawn_ffmpeg_audio_thread(
                    ff,
                    path,
                    seek_ms,
                    speed,
                    audio_track_idx,
                    shared,
                    gen,
                    my_gen,
                );
            } else {
                if let Ok(mut e) = last_error.lock() {
                    *e = Some(err_msg);
                }
                shared.eof.store(true, Ordering::Release);
            }
        }
    });
}

fn run_symphonia_decode(
    path: &str,
    seek_ms: u64,
    shared: &AudioSharedBuffer,
    gen: &AtomicU64,
    my_gen: u64,
) -> Result<(), String> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::codecs::registry::CodecRegistry;
    use symphonia::core::codecs::CodecParameters;
    use symphonia::core::common::Tier;
    use symphonia::core::errors::Error;
    use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::units::Time;
    use symphonia_opus::OpusDecoder;

    let mut codecs = CodecRegistry::new();
    symphonia::default::register_enabled_codecs(&mut codecs);
    codecs.register_audio_decoder_at_tier::<OpusDecoder>(Tier::Fallback);

    let file = File::open(path).map_err(|e| format!("Open failed: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = symphonia::core::formats::probe::Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .map_err(|e| format!("Probe failed: {e}"))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| "No audio track".to_string())?;
    let track_id = track.id;
    let audio_params = match &track.codec_params {
        Some(CodecParameters::Audio(p)) => p.clone(),
        _ => return Err("Missing audio codec parameters".to_string()),
    };

    if seek_ms > 0 {
        if let Some(time) = Time::try_from_secs_f64(seek_ms as f64 / 1000.0) {
            let _ = format.seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(track_id),
                },
            );
        }
    }

    let mut decoder = codecs
        .make_audio_decoder(&audio_params, &AudioDecoderOptions::default())
        .map_err(|e| format!("Unsupported audio codec: {e}"))?;

    let mut interleaved: Vec<f32> = Vec::new();
    let mut resampled: Vec<f32> = Vec::new();

    loop {
        if gen.load(Ordering::Acquire) != my_gen {
            return Ok(());
        }

        // Throttle if ring already holds > 250ms
        loop {
            if gen.load(Ordering::Acquire) != my_gen {
                return Ok(());
            }
            let len = shared.ring.lock().map(|q| q.len()).unwrap_or(0);
            if len < AUDIO_RING_MAX_FRAMES {
                break;
            }
            thread::sleep(Duration::from_millis(8));
        }

        let packet = match format.next_packet() {
            Ok(Some(pkt)) => pkt,
            Ok(None) => {
                shared.eof.store(true, Ordering::Release);
                return Ok(());
            }
            Err(Error::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(Error::IoError(ref ioe)) if ioe.kind() == std::io::ErrorKind::UnexpectedEof => {
                shared.eof.store(true, Ordering::Release);
                return Ok(());
            }
            Err(_) => {
                shared.eof.store(true, Ordering::Release);
                return Ok(());
            }
        };

        if packet.track_id != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(Error::DecodeError(_)) => continue,
            Err(_) => continue,
        };

        let spec = decoded.spec();
        let src_rate = spec.rate().max(8000);
        let src_ch = spec.channels().count().max(1);

        interleaved.clear();
        decoded.copy_to_vec_interleaved(&mut interleaved);

        let speed = f32::from_bits(shared.speed_bits.load(Ordering::Acquire)).clamp(0.25, 4.0);
        resample_to_stereo_48k(&interleaved, src_ch, src_rate, speed, &mut resampled);

        if let Ok(mut q) = shared.ring.lock() {
            q.extend(resampled.iter().copied());
        }
    }
}

fn resample_to_stereo_48k(
    src: &[f32],
    src_ch: usize,
    src_rate: u32,
    speed: f32,
    dst: &mut Vec<f32>,
) {
    dst.clear();
    if src_ch == 0 || src.is_empty() {
        return;
    }
    let src_frames = src.len() / src_ch;
    if src_frames == 0 {
        return;
    }
    let effective_src_rate = src_rate as f64 * speed as f64;
    let ratio = effective_src_rate / TARGET_AUDIO_RATE as f64;
    let out_frames = ((src_frames as f64 / ratio).ceil() as usize).max(1);
    dst.reserve(out_frames * 2);

    for i in 0..out_frames {
        let pos = i as f64 * ratio;
        let idx0 = (pos as usize).min(src_frames - 1);
        let idx1 = (idx0 + 1).min(src_frames - 1);
        let frac = (pos - idx0 as f64) as f32;

        let (l0, r0) = frame_to_stereo(&src[idx0 * src_ch..(idx0 + 1) * src_ch]);
        let (l1, r1) = frame_to_stereo(&src[idx1 * src_ch..(idx1 + 1) * src_ch]);

        dst.push(l0 + (l1 - l0) * frac);
        dst.push(r0 + (r1 - r0) * frac);
    }
}

#[inline(always)]
fn frame_to_stereo(frame: &[f32]) -> (f32, f32) {
    match frame.len() {
        1 => (frame[0], frame[0]),
        2 => (frame[0], frame[1]),
        n if n >= 6 => {
            // 5.1 downmix: FL, FR, FC, LFE, BL, BR
            let fl = frame[0];
            let fr = frame[1];
            let fc = frame[2] * 0.707;
            let bl = frame[4] * 0.5;
            let br = frame[5] * 0.5;
            ((fl + fc + bl).clamp(-1.0, 1.0), (fr + fc + br).clamp(-1.0, 1.0))
        }
        _ => (frame[0], frame[1]),
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_ffmpeg_audio_thread(
    ffmpeg: PathBuf,
    path: String,
    seek_ms: u64,
    speed: f32,
    audio_track: usize,
    shared: Arc<AudioSharedBuffer>,
    gen: Arc<AtomicU64>,
    my_gen: u64,
) {
    thread::spawn(move || {
        let mut cmd = Command::new(ffmpeg);
        hide_window_cmd(&mut cmd);
        cmd.args(["-hide_banner", "-loglevel", "error"]);
        if seek_ms > 0 {
            let ss = format!("{:.3}", seek_ms as f64 / 1000.0);
            cmd.args(["-ss", &ss]);
        }
        cmd.args(["-i", &path]);
        let map_arg = format!("0:a:{audio_track}?");
        cmd.args(["-map", &map_arg, "-vn", "-sn"]);
        if (speed - 1.0).abs() > 0.05 {
            let atempo = build_atempo_chain(speed);
            cmd.args(["-filter:a", &atempo]);
        }
        cmd.args([
            "-f",
            "f32le",
            "-ac",
            "2",
            "-ar",
            "48000",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

        let mut child: Child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => {
                shared.eof.store(true, Ordering::Release);
                return;
            }
        };
        let Some(mut stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            shared.eof.store(true, Ordering::Release);
            return;
        };

        let mut raw = vec![0u8; 4096 * 8];
        loop {
            if gen.load(Ordering::Acquire) != my_gen {
                break;
            }
            loop {
                if gen.load(Ordering::Acquire) != my_gen {
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
                let len = shared.ring.lock().map(|q| q.len()).unwrap_or(0);
                if len < AUDIO_RING_MAX_FRAMES {
                    break;
                }
                thread::sleep(Duration::from_millis(8));
            }
            match stdout.read(&mut raw) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let usable = n & !7; // whole stereo frames (8 bytes)
                    if usable > 0 {
                        if let Ok(mut q) = shared.ring.lock() {
                            for chunk in raw[..usable].chunks_exact(4) {
                                let s = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                                q.push_back(if s.is_finite() { s } else { 0.0 });
                            }
                        }
                    }
                }
            }
        }

        let _ = child.kill();
        let _ = child.wait();
        if gen.load(Ordering::Acquire) == my_gen {
            shared.eof.store(true, Ordering::Release);
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn spawn_ffmpeg_video_thread(
    ffmpeg: PathBuf,
    path: String,
    seek_ms: u64,
    speed: f32,
    dec_w: u32,
    dec_h: u32,
    fps: f32,
    video_queue: Arc<Mutex<VecDeque<VideoFrame>>>,
    video_eof: Arc<AtomicBool>,
    gen: Arc<AtomicU64>,
    my_gen: u64,
    last_error: Arc<Mutex<Option<String>>>,
) {
    thread::spawn(move || {
        let mut cmd = Command::new(ffmpeg);
        hide_window_cmd(&mut cmd);
        cmd.args(["-hide_banner", "-loglevel", "error"]);
        if seek_ms > 0 {
            let ss = format!("{:.3}", seek_ms as f64 / 1000.0);
            cmd.args(["-ss", &ss]);
        }
        let scale_vf = format!("scale={dec_w}:{dec_h}:flags=bilinear");
        cmd.args([
            "-i",
            &path,
            "-map",
            "0:v:0",
            "-an",
            "-sn",
            "-vf",
            &scale_vf,
            "-pix_fmt",
            "bgr0",
            "-f",
            "rawvideo",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                if let Ok(mut er) = last_error.lock() {
                    *er = Some(format!("FFmpeg launch failed: {e}"));
                }
                video_eof.store(true, Ordering::Release);
                return;
            }
        };

        let Some(mut stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            video_eof.store(true, Ordering::Release);
            return;
        };

        let frame_bytes = (dec_w as usize) * (dec_h as usize) * 4;
        let mut raw = vec![0u8; frame_bytes];
        let mut frame_idx: u64 = 0;
        let frame_ms = 1000.0 / (fps.max(1.0) as f64);

        loop {
            if gen.load(Ordering::Acquire) != my_gen {
                break;
            }

            // Throttle video queue so memory stays lightweight
            loop {
                if gen.load(Ordering::Acquire) != my_gen {
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
                let qlen = video_queue.lock().map(|q| q.len()).unwrap_or(0);
                if qlen < VIDEO_QUEUE_MAX_FRAMES {
                    break;
                }
                thread::sleep(Duration::from_millis(6));
            }

            let mut filled = 0usize;
            let mut eof = false;
            while filled < frame_bytes {
                match stdout.read(&mut raw[filled..]) {
                    Ok(0) => {
                        eof = true;
                        break;
                    }
                    Ok(n) => filled += n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        eof = true;
                        break;
                    }
                }
            }
            if eof || filled < frame_bytes {
                break;
            }

            let mut buf = Vec::with_capacity(dec_w as usize * dec_h as usize);
            for px in raw.chunks_exact(4) {
                // bgr0 in memory: [B, G, R, 0] -> 0xFFRRGGBB
                buf.push(0xFF000000 | ((px[2] as u32) << 16) | ((px[1] as u32) << 8) | (px[0] as u32));
            }

            let ts = seek_ms + ((frame_idx as f64 * frame_ms) / speed.max(0.25) as f64 * speed as f64) as u64;
            frame_idx += 1;

            if let Ok(mut q) = video_queue.lock() {
                q.push_back(VideoFrame {
                    width: dec_w,
                    height: dec_h,
                    timestamp_ms: ts,
                    buffer: buf,
                });
            }
        }

        let _ = child.kill();
        if let Ok(output) = child.wait_with_output() {
            if frame_idx == 0 && !output.stderr.is_empty() {
                let msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if !msg.is_empty() {
                    if let Ok(mut er) = last_error.lock() {
                        *er = Some(msg);
                    }
                }
            }
        }
        if gen.load(Ordering::Acquire) == my_gen {
            video_eof.store(true, Ordering::Release);
        }
    });
}

/// Pure-Rust H.264 MP4 fallback decoder (used when FFmpeg binary is not yet installed).
fn spawn_pure_rust_h264_thread(
    path: String,
    seek_ms: u64,
    video_queue: Arc<Mutex<VecDeque<VideoFrame>>>,
    video_eof: Arc<AtomicBool>,
    gen: Arc<AtomicU64>,
    my_gen: u64,
    last_error: Arc<Mutex<Option<String>>>,
) {
    thread::spawn(move || {
        let res = run_pure_rust_h264(&path, seek_ms, &video_queue, &gen, my_gen);
        if let Err(e) = res {
            if let Ok(mut er) = last_error.lock() {
                *er = Some(e);
            }
        }
        if gen.load(Ordering::Acquire) == my_gen {
            video_eof.store(true, Ordering::Release);
        }
    });
}

fn run_pure_rust_h264(
    path: &str,
    seek_ms: u64,
    video_queue: &Mutex<VecDeque<VideoFrame>>,
    gen: &AtomicU64,
    my_gen: u64,
) -> Result<(), String> {
    let f = File::open(path).map_err(|e| format!("Open MP4: {e}"))?;
    let size = f.metadata().map_err(|e| e.to_string())?.len();
    let mut mp4 = mp4::Mp4Reader::read_header(BufReader::new(f), size)
        .map_err(|e| format!("MP4 parse: {e}"))?;

    let mut video_track_id = None;
    let mut sps_pps_annexb = Vec::new();
    let mut nal_len_size = 4usize;
    let mut timescale = 1000u32;

    for (&tid, trk) in mp4.tracks() {
        if matches!(trk.track_type(), Ok(mp4::TrackType::Video)) {
            if let Some(ref stsd) = Some(&trk.trak.mdia.minf.stbl.stsd) {
                if let Some(ref avc1) = stsd.avc1 {
                    video_track_id = Some(tid);
                    timescale = trk.timescale().max(1);
                    nal_len_size = (avc1.avcc.length_size_minus_one as usize & 3) + 1;
                    for sps in &avc1.avcc.sequence_parameter_sets {
                        sps_pps_annexb.extend_from_slice(&[0, 0, 0, 1]);
                        sps_pps_annexb.extend_from_slice(&sps.bytes);
                    }
                    for pps in &avc1.avcc.picture_parameter_sets {
                        sps_pps_annexb.extend_from_slice(&[0, 0, 0, 1]);
                        sps_pps_annexb.extend_from_slice(&pps.bytes);
                    }
                    break;
                }
            }
        }
    }

    let tid = video_track_id.ok_or_else(|| "No AVC/H.264 track in MP4".to_string())?;
    let count = mp4.sample_count(tid).map_err(|e| e.to_string())?;

    let mut decoder = rusty_h264_decoder::Decoder::new();
    if !sps_pps_annexb.is_empty() {
        let _ = decoder.decode(&sps_pps_annexb);
    }

    // Find start sample near seek_ms (back up to nearest sync sample if possible)
    let mut start_id = 1u32;
    if seek_ms > 0 {
        for sid in 1..=count {
            if let Ok(Some(s)) = mp4.read_sample(tid, sid) {
                let ts_ms = (s.start_time * 1000) / timescale as u64;
                if s.is_sync && ts_ms <= seek_ms {
                    start_id = sid;
                } else if ts_ms > seek_ms {
                    break;
                }
            }
        }
    }

    let mut annex_b = Vec::with_capacity(65536);
    for sid in start_id..=count {
        if gen.load(Ordering::Acquire) != my_gen {
            return Ok(());
        }
        loop {
            if gen.load(Ordering::Acquire) != my_gen {
                return Ok(());
            }
            let qlen = video_queue.lock().map(|q| q.len()).unwrap_or(0);
            if qlen < VIDEO_QUEUE_MAX_FRAMES {
                break;
            }
            thread::sleep(Duration::from_millis(6));
        }

        let Ok(Some(sample)) = mp4.read_sample(tid, sid) else {
            continue;
        };
        let ts_ms = (sample.start_time * 1000) / timescale as u64;

        annex_b.clear();
        avcc_to_annex_b(&sample.bytes, nal_len_size, &mut annex_b);

        if let Ok(Some(yuv)) = decoder.decode(&annex_b) {
            if ts_ms + 30 < seek_ms {
                continue;
            }
            let frame = yuv420_to_xrgb(yuv.width, yuv.height, &yuv.y, &yuv.u, &yuv.v, ts_ms);
            if let Ok(mut q) = video_queue.lock() {
                q.push_back(frame);
            }
        }
    }
    Ok(())
}

fn avcc_to_annex_b(data: &[u8], len_size: usize, out: &mut Vec<u8>) {
    let mut off = 0usize;
    while off + len_size <= data.len() {
        let mut nal_len = 0usize;
        for b in &data[off..off + len_size] {
            nal_len = (nal_len << 8) | (*b as usize);
        }
        off += len_size;
        if nal_len == 0 || off + nal_len > data.len() {
            break;
        }
        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(&data[off..off + nal_len]);
        off += nal_len;
    }
}

fn yuv420_to_xrgb(w: usize, h: usize, y_plane: &[u8], u_plane: &[u8], v_plane: &[u8], ts_ms: u64) -> VideoFrame {
    let cw = (w / 2).max(1);
    let ch = (h / 2).max(1);
    let mut buf = vec![0xFF000000u32; w * h];

    for y in 0..h {
        let cy = (y / 2).min(ch - 1);
        for x in 0..w {
            let cx = (x / 2).min(cw - 1);
            let yy = y_plane.get(y * w + x).copied().unwrap_or(16) as i32 - 16;
            let uu = u_plane.get(cy * cw + cx).copied().unwrap_or(128) as i32 - 128;
            let vv = v_plane.get(cy * cw + cx).copied().unwrap_or(128) as i32 - 128;

            let c = 298 * yy;
            let r = ((c + 409 * vv + 128) >> 8).clamp(0, 255) as u32;
            let g = ((c - 100 * uu - 208 * vv + 128) >> 8).clamp(0, 255) as u32;
            let b = ((c + 516 * uu + 128) >> 8).clamp(0, 255) as u32;
            buf[y * w + x] = 0xFF000000 | (r << 16) | (g << 8) | b;
        }
    }

    VideoFrame {
        width: w as u32,
        height: h as u32,
        timestamp_ms: ts_ms,
        buffer: buf,
    }
}

fn build_atempo_chain(mut speed: f32) -> String {
    let mut filters = Vec::new();
    while speed > 2.0 {
        filters.push("atempo=2.0".to_string());
        speed /= 2.0;
    }
    while speed < 0.5 {
        filters.push("atempo=0.5".to_string());
        speed *= 2.0;
    }
    filters.push(format!("atempo={:.3}", speed));
    filters.join(",")
}

fn parse_rate_fraction(s: &str) -> f32 {
    if let Some((num, den)) = s.split_once('/') {
        let n: f32 = num.parse().unwrap_or(0.0);
        let d: f32 = den.parse().unwrap_or(1.0);
        if d > 0.0 && n > 0.0 {
            return n / d;
        }
    }
    s.parse().unwrap_or(0.0)
}

fn parse_ffmpeg_duration(s: &str) -> Option<u64> {
    let segs: Vec<&str> = s.split(':').collect();
    if segs.len() != 3 {
        return None;
    }
    let h: f64 = segs[0].parse().ok()?;
    let m: f64 = segs[1].parse().ok()?;
    let sec: f64 = segs[2].parse().ok()?;
    Some(((h * 3600.0 + m * 60.0 + sec) * 1000.0) as u64)
}

fn parse_wxh(s: &str) -> Option<(u32, u32)> {
    let tok = s.split_whitespace().next()?;
    let (ws, hs) = tok.split_once('x')?;
    let w: u32 = ws.parse().ok()?;
    let h: u32 = hs.parse().ok()?;
    if w >= 64 && h >= 64 && w <= 16384 && h <= 16384 {
        Some((w, h))
    } else {
        None
    }
}

fn extract_stream_lang(line: &str) -> String {
    if let Some(p1) = line.find('(') {
        if let Some(p2) = line[p1 + 1..].find(')') {
            let inside = &line[p1 + 1..p1 + 1 + p2];
            if inside.len() <= 5 && !inside.contains(':') {
                return inside.to_string();
            }
        }
    }
    "und".to_string()
}

pub fn is_image_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".bmp")
        || lower.ends_with(".webp")
        || lower.ends_with(".gif")
}

/// Saves a VideoFrame (0xFFRRGGBB) to a real PNG file.
pub fn save_frame_png(frame: &VideoFrame, out_path: &Path) -> Result<(), String> {
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut rgba = Vec::with_capacity(frame.buffer.len() * 4);
    for &px in &frame.buffer {
        rgba.push(((px >> 16) & 0xFF) as u8);
        rgba.push(((px >> 8) & 0xFF) as u8);
        rgba.push((px & 0xFF) as u8);
        rgba.push(255);
    }
    let img = image::RgbaImage::from_raw(frame.width, frame.height, rgba)
        .ok_or_else(|| "Invalid frame buffer dimensions".to_string())?;
    img.save(out_path).map_err(|e| format!("PNG save failed: {e}"))
}

pub fn wall_clock_ms_since(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}
