//! # Nova Player Core Engine
//!
//! Pure-Rust media processing pipeline, container demuxers (MP4, MKV, WebM, AVI, WAV, MP3, FLAC),
//! procedural ultra-HD video synthesizers, audio DSP with 32-band spectrum analyzer, and state machine.

pub mod codec;
pub mod demux;
pub mod engine;
pub mod playlist;
pub mod subtitles;
pub mod types;

pub use codec::audio_dsp::{AudioEngine, SPECTRUM_BANDS};
pub use codec::video_synth::{VideoAdjustments, VideoSynthesizer, VideoTheme};
pub use demux::demux_file;
pub use engine::MediaPlayerEngine;
pub use playlist::{LoopMode, Playlist, PlaylistItem};
pub use subtitles::{SubtitleCue, SubtitleTrack};
pub use types::{AspectRatio, EqualizerPreset, MediaFormat, MediaMetadata, PlaybackState, Resolution, VideoFrame};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_initialization_and_tick() {
        let mut engine = MediaPlayerEngine::new();
        assert_eq!(engine.state, PlaybackState::Playing);
        assert_eq!(engine.position_ms, 0);

        engine.tick(100);
        assert_eq!(engine.position_ms, 100);
        assert!(engine.total_rendered_frames > 0);
    }

    #[test]
    fn test_transport_controls_and_seeking() {
        let mut engine = MediaPlayerEngine::new();
        engine.pause();
        assert_eq!(engine.state, PlaybackState::Paused);

        engine.tick(100);
        assert_eq!(engine.position_ms, 0); // Paused, does not advance

        engine.play();
        engine.seek(15_000);
        assert_eq!(engine.position_ms, 15_000);

        engine.seek_relative(-5_000);
        assert_eq!(engine.position_ms, 10_000);

        engine.seek_relative(8_000);
        assert_eq!(engine.position_ms, 18_000);
    }

    #[test]
    fn test_volume_boost_and_audio_spectrum() {
        let mut engine = MediaPlayerEngine::new();
        engine.audio.set_volume(1.25);
        assert_eq!(engine.audio.volume, 1.25); // 125% boost

        engine.tick(200);
        let has_spectrum_activity = engine.audio.spectrum_levels.iter().any(|&lvl| lvl > 0.0);
        assert!(has_spectrum_activity, "Spectrum analyzer should report active frequency bands");

        engine.audio.toggle_mute();
        assert!(engine.audio.is_muted);
        assert_eq!(engine.audio.effective_gain(), 0.0);
    }

    #[test]
    fn test_mp4_demuxing_simulation() {
        // Construct minimal valid MP4 header
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x14]); // 20 bytes
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"isom");
        data.extend_from_slice(&[0x00, 0x00, 0x02, 0x00]);
        data.extend_from_slice(b"mp41");

        let meta = demux_file("movies/clip.mp4", &data).expect("Should demux MP4");
        assert_eq!(meta.format, MediaFormat::Mp4);
    }

    #[test]
    fn test_mkv_demuxing_simulation() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x1A, 0x45, 0xDF, 0xA3]); // EBML magic
        data.extend_from_slice(b"matroska");

        let meta = demux_file("streams/ultra4k.mkv", &data).expect("Should demux MKV");
        assert_eq!(meta.format, MediaFormat::Mkv);
    }

    #[test]
    fn test_subtitles_parsing() {
        let srt = "1\n00:00:01,000 --> 00:00:04,000\nHello NovaOS!\n\n2\n00:00:05,000 --> 00:00:08,000\nFast Media Engine\n";
        let track = SubtitleTrack::parse_srt(srt, "English");
        assert_eq!(track.cues.len(), 2);
        assert_eq!(track.active_cue_text(2000), Some("Hello NovaOS!"));
        assert_eq!(track.active_cue_text(6000), Some("Fast Media Engine"));
        assert_eq!(track.active_cue_text(10000), None);
    }
}
