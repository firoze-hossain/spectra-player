//! Central High-Performance Media Player Engine.

use crate::codec::audio_dsp::AudioEngine;
use crate::codec::video_synth::{VideoSynthesizer, VideoTheme};
use crate::demux::demux_file;
use crate::playlist::Playlist;
use crate::subtitles::SubtitleTrack;
use crate::types::{AspectRatio, MediaFormat, MediaMetadata, PlaybackState, VideoFrame};

pub struct MediaPlayerEngine {
    pub metadata: MediaMetadata,
    pub state: PlaybackState,
    pub position_ms: u64,
    pub speed: f32,
    pub aspect_ratio: AspectRatio,
    pub audio: AudioEngine,
    pub video_synth: VideoSynthesizer,
    pub playlist: Playlist,
    pub subtitles: SubtitleTrack,
    pub fps: f32,
    pub dropped_frames: u64,
    pub total_rendered_frames: u64,
}

impl Default for MediaPlayerEngine {
    fn default() -> Self {
        let pl = Playlist::default();
        let first = pl.current_item().cloned();

        let mut meta = MediaMetadata::default();
        if let Some(item) = first {
            meta.title = item.title;
            meta.format = item.format;
            meta.duration_ms = item.duration_ms;
            meta.width = item.width;
            meta.height = item.height;
        }

        Self {
            metadata: meta,
            state: PlaybackState::Playing,
            position_ms: 0,
            speed: 1.0,
            aspect_ratio: AspectRatio::Auto,
            audio: AudioEngine::new(),
            video_synth: VideoSynthesizer::new(VideoTheme::CyberpunkMetropolis),
            playlist: pl,
            subtitles: SubtitleTrack::default(),
            fps: 60.0,
            dropped_frames: 0,
            total_rendered_frames: 0,
        }
    }
}

impl MediaPlayerEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances playback simulation by elapsed time delta in milliseconds.
    pub fn tick(&mut self, delta_ms: u64) {
        if self.state == PlaybackState::Playing {
            let advanced = (delta_ms as f32 * self.speed) as u64;
            self.position_ms += advanced;

            if self.position_ms >= self.metadata.duration_ms {
                self.position_ms = self.metadata.duration_ms;
                // Move to next playlist track if repeat active
                self.next_track();
            }

            self.total_rendered_frames += 1;
        }

        self.audio.update_spectrum(self.position_ms, self.state == PlaybackState::Playing);
    }

    pub fn play(&mut self) {
        self.state = PlaybackState::Playing;
    }

    pub fn pause(&mut self) {
        self.state = PlaybackState::Paused;
    }

    pub fn toggle_play(&mut self) {
        if self.state == PlaybackState::Playing {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.position_ms = 0;
        self.audio.update_spectrum(0, false);
    }

    pub fn seek(&mut self, target_ms: u64) {
        self.position_ms = target_ms.min(self.metadata.duration_ms);
        self.audio.update_spectrum(self.position_ms, self.state == PlaybackState::Playing);
    }

    pub fn seek_relative(&mut self, delta_ms: i64) {
        if delta_ms < 0 {
            let sub = (-delta_ms) as u64;
            self.position_ms = self.position_ms.saturating_sub(sub);
        } else {
            self.seek(self.position_ms + delta_ms as u64);
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.25, 4.0);
    }

    pub fn cycle_speed(&mut self) {
        self.speed = match self.speed {
            s if (s - 0.5).abs() < 0.1 => 1.0,
            s if (s - 1.0).abs() < 0.1 => 1.25,
            s if (s - 1.25).abs() < 0.1 => 1.5,
            s if (s - 1.5).abs() < 0.1 => 2.0,
            s if (s - 2.0).abs() < 0.1 => 0.5,
            _ => 1.0,
        };
    }

    pub fn cycle_aspect_ratio(&mut self) {
        self.aspect_ratio = match self.aspect_ratio {
            AspectRatio::Auto => AspectRatio::Ratio16_9,
            AspectRatio::Ratio16_9 => AspectRatio::Ratio4_3,
            AspectRatio::Ratio4_3 => AspectRatio::Ratio21_9,
            AspectRatio::Ratio21_9 => AspectRatio::Ratio1_1,
            AspectRatio::Ratio1_1 => AspectRatio::Fill,
            AspectRatio::Fill => AspectRatio::Auto,
        };
    }

    pub fn next_track(&mut self) {
        if let Some(item) = self.playlist.next().cloned() {
            self.apply_playlist_item(&item);
        }
    }

    pub fn previous_track(&mut self) {
        if self.position_ms > 3000 {
            // If more than 3s into track, restart current track
            self.position_ms = 0;
        } else if let Some(item) = self.playlist.previous().cloned() {
            self.apply_playlist_item(&item);
        }
    }

    pub fn select_track(&mut self, index: usize) {
        if let Some(item) = self.playlist.select(index).cloned() {
            self.apply_playlist_item(&item);
        }
    }

    fn apply_playlist_item(&mut self, item: &crate::playlist::PlaylistItem) {
        self.metadata.title = item.title.clone();
        self.metadata.artist = item.artist.clone();
        self.metadata.format = item.format;
        self.metadata.duration_ms = item.duration_ms;
        self.metadata.width = item.width;
        self.metadata.height = item.height;
        self.position_ms = 0;
        self.state = PlaybackState::Playing;

        // Switch visualizer theme based on format and resolution
        if item.width >= 7000 {
            self.video_synth.theme = VideoTheme::DeepSpaceNebula;
            self.metadata.video_codec = "AV1 8K FUHD Ultra Cinema".to_string();
        } else if item.format == MediaFormat::Webm {
            self.video_synth.theme = VideoTheme::OceanAurora;
            self.metadata.video_codec = "VP9 1080p 60FPS High-Efficiency".to_string();
        } else {
            self.video_synth.theme = VideoTheme::CyberpunkMetropolis;
            self.metadata.video_codec = "H.264 / AVC 4K UHD Master".to_string();
        }
    }

    /// Loads external file from disk or memory and sets up media pipeline.
    pub fn load_file(&mut self, path: &str, data: &[u8]) {
        if let Ok(meta) = demux_file(path, data) {
            self.metadata = meta;
            self.position_ms = 0;
            self.state = PlaybackState::Playing;

            if self.metadata.width >= 7000 {
                self.video_synth.theme = VideoTheme::DeepSpaceNebula;
            } else if self.metadata.format == MediaFormat::Webm {
                self.video_synth.theme = VideoTheme::OceanAurora;
            } else {
                self.video_synth.theme = VideoTheme::CyberpunkMetropolis;
            }
        }
    }

    /// Renders current video frame scaled to display area.
    pub fn current_video_frame(&self, target_w: u32, target_h: u32) -> Option<VideoFrame> {
        if self.metadata.format.is_audio_only() {
            return None;
        }

        let (_src_w, _src_h) = if self.metadata.width > 0 && self.metadata.height > 0 {
            (self.metadata.width, self.metadata.height)
        } else {
            (1920, 1080)
        };

        // Render at target resolution directly for maximum performance and sharpness
        Some(self.video_synth.render_frame(target_w, target_h, self.position_ms))
    }

    /// Current active subtitle string, if any.
    pub fn active_subtitle(&self) -> Option<&str> {
        self.subtitles.active_cue_text(self.position_ms)
    }
}
