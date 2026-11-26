//! # Spectra Core Engine
//!
//! Pure-Rust & Universal media processing pipeline:
//! container demuxers, real audio/video decoding, 10-band biquad EQ & FFT spectrum,
//! multi-format subtitles (SRT/VTT/ASS/SUB), playlist (M3U8/shuffle/repeat), and state machine.

pub mod codec;
pub mod demux;
pub mod engine;
pub mod playlist;
pub mod subtitles;
pub mod types;

pub use codec::audio_dsp::{AudioEngine, EQ_BANDS, EQ_FREQUENCIES, SPECTRUM_BANDS, WAVEFORM_SAMPLES};
pub use codec::video_synth::{VideoAdjustments, VideoSynthesizer, VideoTheme};
pub use demux::demux_file;
pub use engine::MediaPlayerEngine;
pub use playlist::{LoopMode, Playlist, PlaylistItem};
pub use subtitles::{SubtitleCue, SubtitleTrack};
pub use types::{
    AspectRatio, AudioVisualizerMode, CoverArt, CropMode, DecoderBackend, EqualizerPreset,
    MediaFormat, MediaMetadata, PlaybackState, Resolution, StreamTrackInfo, VideoFrame,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_initialization_and_tick() {
        let mut engine = MediaPlayerEngine::new();
        assert_eq!(engine.state, PlaybackState::Stopped);
        assert!(engine.playlist.items.is_empty());
        assert!(engine.current_video_frame(640, 360).is_none());

        let mp4_bytes = [
            0x00, 0x00, 0x00, 0x14, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm',
            0x00, 0x00, 0x02, 0x00, b'm', b'p', b'4', b'1',
        ];
        engine.load_file("movie.mp4", &mp4_bytes);
        assert_eq!(engine.state, PlaybackState::Playing);
        engine.tick(100);
        assert_eq!(engine.position_ms, 100);
        assert!(engine.total_rendered_frames > 0);
    }

    #[test]
    fn test_transport_controls_and_seeking() {
        let mut engine = MediaPlayerEngine::new();
        let mp4_bytes = [
            0x00, 0x00, 0x00, 0x14, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm',
            0x00, 0x00, 0x02, 0x00, b'm', b'p', b'4', b'1',
        ];
        engine.load_file("sample.mp4", &mp4_bytes);
        engine.pause();
        assert_eq!(engine.state, PlaybackState::Paused);

        engine.tick(100);
        assert_eq!(engine.position_ms, 0);

        engine.play();
        engine.seek(15_000);
        assert_eq!(engine.position_ms, 15_000);

        engine.seek_relative(-5_000);
        assert_eq!(engine.position_ms, 10_000);

        engine.seek_relative(8_000);
        assert_eq!(engine.position_ms, 18_000);
    }

    #[test]
    fn test_volume_boost_eq_and_real_fft_spectrum() {
        let mut engine = MediaPlayerEngine::new();
        engine.audio.set_volume(1.50);
        assert_eq!(engine.audio.volume, 1.50);

        // Test real PCM processing with 10-band EQ & FFT
        let mut pcm = vec![0.0f32; 1024];
        for i in 0..512 {
            let s = (2.0 * core::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin() * 0.5;
            pcm[i * 2] = s;
            pcm[i * 2 + 1] = s;
        }
        engine.audio.set_equalizer_preset(EqualizerPreset::BassBoost);
        engine.audio.process_pcm_stereo(&mut pcm, 48000);
        engine.audio.feed_pcm_for_spectrum(&pcm);

        let has_spectrum_activity = engine.audio.spectrum_levels.iter().any(|&lvl| lvl > 0.05);
        assert!(has_spectrum_activity, "Real FFT analyzer should report active frequency bands");

        engine.audio.toggle_mute();
        assert!(engine.audio.is_muted);
        assert_eq!(engine.audio.effective_gain(), 0.0);
    }

    #[test]
    fn test_mp4_demuxing_simulation() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x14]);
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
        data.extend_from_slice(&[0x1A, 0x45, 0xDF, 0xA3]);
        data.extend_from_slice(b"matroska");

        let meta = demux_file("streams/ultra4k.mkv", &data).expect("Should demux MKV");
        assert_eq!(meta.format, MediaFormat::Mkv);
    }

    #[test]
    fn test_subtitles_srt_vtt_ass_parsing() {
        let srt = "1\n00:00:01,000 --> 00:00:04,000\n<i>Hello NovaOS!</i>\n\n2\n00:00:05,000 --> 00:00:08,000\nFast Media Engine\n";
        let track = SubtitleTrack::parse_srt(srt, "English");
        assert_eq!(track.cues.len(), 2);
        assert_eq!(track.active_cue_text(2000), Some("Hello NovaOS!"));
        assert_eq!(track.active_cue_text(6000), Some("Fast Media Engine"));
        assert_eq!(track.active_cue_text(10000), None);

        let vtt = "WEBVTT\n\n00:01.500 --> 00:03.500 align:middle\nWebVTT Cue\n";
        let vtrack = SubtitleTrack::parse_auto(vtt, "subs.vtt");
        assert_eq!(vtrack.active_cue_text(2000), Some("WebVTT Cue"));

        let ass = "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:02.00,0:00:05.00,Default,,0,0,0,,{\\i1}ASS Line{\\i0}\n";
        let atrack = SubtitleTrack::parse_auto(ass, "subs.ass");
        assert_eq!(atrack.active_cue_text(3000), Some("ASS Line"));
    }

    #[test]
    fn test_playlist_shuffle_and_m3u8() {
        let mut pl = Playlist::empty();
        pl.add(PlaylistItem::new("a.mp4", "Alpha", "Artist A", MediaFormat::Mp4, 60_000, 1920, 1080));
        pl.add(PlaylistItem::new("b.mkv", "Beta", "Artist B", MediaFormat::Mkv, 120_000, 3840, 2160));
        pl.add(PlaylistItem::new("c.flac", "Gamma", "Artist C", MediaFormat::Flac, 180_000, 0, 0));

        pl.set_shuffle(true, 42);
        assert!(pl.is_shuffled);
        let first = pl.current_index;
        let next = pl.next().unwrap().title.clone();
        assert!(!next.is_empty());
        let _prev = pl.previous().unwrap();
        assert_eq!(pl.current_index, first);

        let m3u = pl.to_m3u8();
        assert!(m3u.starts_with("#EXTM3U"));
        let parsed = Playlist::parse_m3u(&m3u);
        assert_eq!(parsed.len(), pl.items.len());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_real_media_probe_if_present() {
        let sample = "/home/firoze/Documents/Vortex/Shararat _ Dhurandhar _ Ranveer, Aditya Dhar, Shashwat Sachdev, Jasmine, Madhubanti, Ayesha, Krystle [1440p].mp4";
        if std::path::Path::new(sample).exists() {
            let meta = crate::codec::media_backend::probe_media(sample);
            assert_eq!(meta.format, MediaFormat::Mp4);
            assert!(meta.width >= 1280, "Expected real 1440p width, got {}", meta.width);
            assert!(meta.duration_ms > 10_000, "Expected real duration > 10s");
        }
    }
}
