//! Central High-Performance Media Player Engine.
//! Coordinates real A/V decoding, A/V clock sync, 10-band EQ & FFT spectrum,
//! multi-track subtitles, A-B looping, snapshots, and playlist playback.

use crate::codec::audio_dsp::AudioEngine;
use crate::codec::video_synth::{VideoSynthesizer, VideoTheme};
use crate::demux::demux_file;
use crate::demux::detect::{is_playlist_path, is_subtitle_path, is_supported_media_path};
use crate::playlist::{LoopMode, Playlist, PlaylistItem};
use crate::subtitles::SubtitleTrack;
use crate::types::{
    AspectRatio, DecoderBackend, MediaFormat, MediaMetadata, PlaybackState, VideoFrame,
};

#[cfg(feature = "std")]
use crate::codec::media_backend::{
    discover_sidecar_subtitles, extract_embedded_subtitle, find_ffmpeg_bin, mp4_has_avc1,
    probe_media, save_frame_png, spawn_ffmpeg_downloader, ActiveMediaSession, CodecStatus,
};
use std::sync::Arc;
#[cfg(feature = "std")]
use std::path::Path;
#[cfg(feature = "std")]
use std::sync::Mutex;

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
    pub subtitle_tracks: Vec<SubtitleTrack>,
    pub active_subtitle_idx: Option<usize>,
    pub active_audio_track: Option<usize>,
    pub active_video_track: Option<usize>,
    pub ab_loop: Option<(u64, Option<u64>)>,
    pub recent_paths: Vec<String>,
    pub fps: f32,
    pub measured_fps: f32,
    pub dropped_frames: u64,
    pub total_rendered_frames: u64,
    pub max_decode_width: u32,
    pub last_status_msg: Option<String>,
    #[cfg(feature = "std")]
    pub session: Option<ActiveMediaSession>,
    #[cfg(feature = "std")]
    pub codec_status: Arc<Mutex<CodecStatus>>,
    frame_clock_accum_ms: u64,
    frame_clock_count: u32,
}

impl Default for MediaPlayerEngine {
    fn default() -> Self {
        #[cfg(feature = "std")]
        let init_codec = if find_ffmpeg_bin().is_some() {
            CodecStatus::Ready
        } else {
            CodecStatus::Missing
        };

        Self {
            metadata: MediaMetadata::default(),
            state: PlaybackState::Stopped,
            position_ms: 0,
            speed: 1.0,
            aspect_ratio: AspectRatio::Auto,
            audio: AudioEngine::new(),
            video_synth: VideoSynthesizer::new(VideoTheme::CyberpunkMetropolis),
            playlist: Playlist::empty(),
            subtitles: SubtitleTrack::empty(),
            subtitle_tracks: Vec::new(),
            active_subtitle_idx: None,
            active_audio_track: None,
            active_video_track: None,
            ab_loop: None,
            recent_paths: load_recent_history(),
            fps: 0.0,
            measured_fps: 0.0,
            dropped_frames: 0,
            total_rendered_frames: 0,
            max_decode_width: 1280,
            last_status_msg: None,
            #[cfg(feature = "std")]
            session: None,
            #[cfg(feature = "std")]
            codec_status: Arc::new(Mutex::new(init_codec)),
            frame_clock_accum_ms: 0,
            frame_clock_count: 0,
        }
    }
}

impl MediaPlayerEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances playback, syncs live A/V clock & decoded video frames, and handles loop/A-B/next.
    pub fn tick(&mut self, delta_ms: u64) {
        if delta_ms > 0 {
            self.frame_clock_accum_ms += delta_ms;
            self.frame_clock_count += 1;
            if self.frame_clock_accum_ms >= 500 {
                self.measured_fps =
                    (self.frame_clock_count as f32 * 1000.0) / self.frame_clock_accum_ms as f32;
                self.frame_clock_accum_ms = 0;
                self.frame_clock_count = 0;
            }
        }

        #[cfg(feature = "std")]
        let mut live_ended = false;
        #[cfg(feature = "std")]
        let is_live_session = self.session.is_some();
        #[cfg(not(feature = "std"))]
        let is_live_session = false;

        if self.state == PlaybackState::Playing {
            #[cfg(feature = "std")]
            if let Some(ref mut sess) = self.session {
                if sess.has_audio_stream && !sess.audio_shared.eof.load(std::sync::atomic::Ordering::Acquire) {
                    let audio_ms = sess.audio_shared.current_audio_ms();
                    if audio_ms > self.position_ms {
                        self.position_ms = audio_ms;
                    } else if audio_ms > 0 {
                        // Gently interpolate between audio callbacks without ever drifting > 20ms ahead
                        let cap = audio_ms + 20;
                        let step = (delta_ms as f32 * self.speed) as u64;
                        self.position_ms = (self.position_ms + step).min(cap);
                    }
                } else {
                    self.position_ms += (delta_ms as f32 * self.speed) as u64;
                }

                // Pace video at actual clock without artificial ahead jump
                let (_, dropped) = sess.advance_video_to(self.position_ms);
                self.dropped_frames += dropped;

                let audio_done = !sess.has_audio_stream
                    || (sess.audio_shared.eof.load(std::sync::atomic::Ordering::Acquire)
                        && sess.audio_shared.ring.lock().map(|q| q.is_empty()).unwrap_or(true));
                let video_done = !sess.has_video_stream
                    || (sess.video_eof.load(std::sync::atomic::Ordering::Acquire)
                        && sess.video_queue.lock().map(|q| q.is_empty()).unwrap_or(true));

                // If video is waiting on FFmpeg download or pure-Rust H.264 failed on High/AV1/VP9,
                // trigger Universal Codec Pack and hot-swap video pipeline the instant FFmpeg is ready.
                let video_needs_upgrade = sess.has_video_stream && !sess.video_uses_ffmpeg;
                let has_video_err = sess.video_eof.load(std::sync::atomic::Ordering::Acquire)
                    && sess.latest_frame.is_none()
                    && sess.last_error.lock().ok().and_then(|e| e.as_ref().cloned()).is_some();

                if audio_done && video_done && self.position_ms > 500 && !video_needs_upgrade {
                    live_ended = true;
                }

                if video_needs_upgrade && find_ffmpeg_bin().is_some() {
                    sess.restart_at(
                        self.position_ms,
                        self.speed,
                        self.active_audio_track,
                        self.max_decode_width,
                    );
                    self.metadata.backend = sess.metadata.backend;
                    self.last_status_msg = Some("Universal Codecs Ready — Video Active".to_string());
                } else if has_video_err && find_ffmpeg_bin().is_none() {
                    spawn_ffmpeg_downloader(Arc::clone(&self.codec_status));
                }
            } else {
                let advanced = (delta_ms as f32 * self.speed) as u64;
                self.position_ms += advanced;
            }

            #[cfg(not(feature = "std"))]
            {
                let advanced = (delta_ms as f32 * self.speed) as u64;
                self.position_ms += advanced;
            }

            self.total_rendered_frames += 1;

            // A-B Loop check
            if let Some((a_ms, Some(b_ms))) = self.ab_loop {
                if b_ms > a_ms && self.position_ms >= b_ms {
                    self.seek(a_ms);
                }
            }

            let dur_reached = self.metadata.duration_ms > 0
                && self.position_ms >= self.metadata.duration_ms
                && (!is_live_session || self.metadata.format.is_audio_only() || {
                    #[cfg(feature = "std")]
                    { live_ended }
                    #[cfg(not(feature = "std"))]
                    { true }
                });

            #[cfg(feature = "std")]
            let should_advance = dur_reached || live_ended;
            #[cfg(not(feature = "std"))]
            let should_advance = dur_reached;

            if should_advance {
                self.handle_track_ended();
            }
        }

        if !self.audio.has_live_pcm || self.state != PlaybackState::Playing {
            self.audio
                .update_spectrum(self.position_ms, self.state == PlaybackState::Playing);
        }
    }

    fn handle_track_ended(&mut self) {
        match self.playlist.loop_mode {
            LoopMode::RepeatOne => {
                self.seek(0);
                self.play();
            }
            LoopMode::RepeatAll => {
                self.next_track();
            }
            LoopMode::None => {
                if self.playlist.current_index + 1 < self.playlist.items.len() {
                    self.next_track();
                } else {
                    self.position_ms = self.metadata.duration_ms;
                    self.state = PlaybackState::Ended;
                    #[cfg(feature = "std")]
                    if let Some(ref sess) = self.session {
                        sess.audio_shared
                            .is_playing
                            .store(false, std::sync::atomic::Ordering::Release);
                    }
                }
            }
        }
    }

    pub fn play(&mut self) {
        let has_media = self.metadata.is_live_file
            || !self.playlist.items.is_empty()
            || self.metadata.format != MediaFormat::Unknown;
        if !has_media {
            self.last_status_msg = Some("No media loaded — press Ctrl+O to open".to_string());
            return;
        }
        if self.state == PlaybackState::Ended {
            self.seek(0);
        }
        self.state = PlaybackState::Playing;
        #[cfg(feature = "std")]
        if let Some(ref sess) = self.session {
            sess.audio_shared
                .is_playing
                .store(true, std::sync::atomic::Ordering::Release);
        }
    }

    pub fn pause(&mut self) {
        self.state = PlaybackState::Paused;
        #[cfg(feature = "std")]
        if let Some(ref sess) = self.session {
            sess.audio_shared
                .is_playing
                .store(false, std::sync::atomic::Ordering::Release);
        }
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
        self.ab_loop = None;
        #[cfg(feature = "std")]
        if let Some(ref mut sess) = self.session {
            sess.audio_shared
                .is_playing
                .store(false, std::sync::atomic::Ordering::Release);
            sess.audio_shared.clear_for_seek(0);
            if let Ok(mut q) = sess.video_queue.lock() {
                q.clear();
            }
            sess.latest_frame = None;
        }
        self.audio.update_spectrum(0, false);
    }

    pub fn seek(&mut self, target_ms: u64) {
        let clamped = if self.metadata.duration_ms > 0 {
            target_ms.min(self.metadata.duration_ms)
        } else {
            target_ms
        };
        self.position_ms = clamped;
        #[cfg(feature = "std")]
        if let Some(ref mut sess) = self.session {
            sess.restart_at(
                clamped,
                self.speed,
                self.active_audio_track,
                self.max_decode_width,
            );
            sess.audio_shared
                .is_playing
                .store(self.state == PlaybackState::Playing, std::sync::atomic::Ordering::Release);
        }
        self.audio
            .update_spectrum(self.position_ms, self.state == PlaybackState::Playing);
    }

    pub fn seek_relative(&mut self, delta_ms: i64) {
        if delta_ms < 0 {
            let sub = (-delta_ms) as u64;
            self.seek(self.position_ms.saturating_sub(sub));
        } else {
            self.seek(self.position_ms + delta_ms as u64);
        }
    }

    pub fn step_frame(&mut self) {
        self.pause();
        #[cfg(feature = "std")]
        if let Some(ref mut sess) = self.session {
            if let Some(ts) = sess.step_one_frame() {
                self.position_ms = ts;
                return;
            }
        }
        let step_ms = (1000.0 / self.metadata.fps.max(24.0)) as u64;
        self.position_ms = (self.position_ms + step_ms).min(self.metadata.duration_ms);
    }

    pub fn set_speed(&mut self, speed: f32) {
        let new_speed = speed.clamp(0.25, 4.0);
        if (self.speed - new_speed).abs() < 0.01 {
            return;
        }
        self.speed = new_speed;
        #[cfg(feature = "std")]
        if let Some(ref mut sess) = self.session {
            sess.restart_at(
                self.position_ms,
                self.speed,
                self.active_audio_track,
                self.max_decode_width,
            );
        }
    }

    pub fn cycle_speed(&mut self) {
        let next = match self.speed {
            s if (s - 0.5).abs() < 0.1 => 1.0,
            s if (s - 1.0).abs() < 0.1 => 1.25,
            s if (s - 1.25).abs() < 0.1 => 1.5,
            s if (s - 1.5).abs() < 0.1 => 2.0,
            s if (s - 2.0).abs() < 0.1 => 0.5,
            _ => 1.0,
        };
        self.set_speed(next);
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

    pub fn cycle_ab_loop(&mut self) -> &'static str {
        match self.ab_loop {
            None => {
                self.ab_loop = Some((self.position_ms, None));
                "A-B Loop: Point A set"
            }
            Some((a, None)) => {
                if self.position_ms > a + 200 {
                    self.ab_loop = Some((a, Some(self.position_ms)));
                    self.seek(a);
                    "A-B Loop: Active"
                } else {
                    self.ab_loop = None;
                    "A-B Loop: Cancelled"
                }
            }
            Some((_, Some(_))) => {
                self.ab_loop = None;
                "A-B Loop: Cleared"
            }
        }
    }

    pub fn next_track(&mut self) {
        if let Some(item) = self.playlist.next().cloned() {
            self.apply_playlist_item(&item);
        }
    }

    pub fn previous_track(&mut self) {
        if self.position_ms > 3000 {
            self.seek(0);
        } else if let Some(item) = self.playlist.previous().cloned() {
            self.apply_playlist_item(&item);
        }
    }

    pub fn select_track(&mut self, index: usize) {
        if let Some(item) = self.playlist.select(index).cloned() {
            self.apply_playlist_item(&item);
        }
    }

    fn apply_playlist_item(&mut self, item: &PlaylistItem) {
        if item.is_live_file && !item.path.starts_with("demo://") {
            #[cfg(feature = "std")]
            {
                self.open_media_internal(&item.path, false);
                return;
            }
        }

        #[cfg(feature = "std")]
        {
            self.session = None;
        }
        self.audio.has_live_pcm = false;
        self.metadata = MediaMetadata {
            path: item.path.clone(),
            title: item.title.clone(),
            artist: item.artist.clone(),
            format: item.format,
            duration_ms: item.duration_ms,
            width: item.width,
            height: item.height,
            is_live_file: item.is_live_file,
            backend: DecoderBackend::PureRustSymphonia,
            ..MediaMetadata::default()
        };
        self.position_ms = 0;
        self.dropped_frames = 0;
        self.ab_loop = None;
        self.state = if item.is_live_file {
            PlaybackState::Playing
        } else {
            PlaybackState::Stopped
        };
    }

    /// Loads external media from memory buffer (used in tests & NovaOS VFS).
    pub fn load_file(&mut self, path: &str, data: &[u8]) {
        if is_subtitle_path(path) {
            let text = String::from_utf8_lossy(data);
            let track = SubtitleTrack::parse_auto(&text, path);
            self.add_subtitle_track(track);
            return;
        }
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

    #[cfg(feature = "std")]
    /// Opens a real file, directory, playlist (.m3u/.m3u8), subtitle, or network stream URL.
    pub fn open_path(&mut self, path_or_url: &str) {
        let trimmed = path_or_url.trim();
        if trimmed.is_empty() {
            return;
        }
        if is_subtitle_path(trimmed) {
            let _ = self.load_subtitle_file(trimmed);
            return;
        }
        if is_playlist_path(trimmed) {
            if let Ok(content) = std::fs::read_to_string(trimmed) {
                let items = Playlist::parse_m3u(&content);
                if !items.is_empty() {
                    self.playlist.clear_demos_if_only_demos();
                    let start_idx = self.playlist.items.len();
                    for it in items {
                        self.playlist.add(it);
                    }
                    self.select_track(start_idx);
                    return;
                }
            }
        }
        let p = Path::new(trimmed);
        if p.is_dir() {
            self.open_directory(trimmed);
            return;
        }
        self.open_media_internal(trimmed, true);
    }

    #[cfg(feature = "std")]
    pub fn open_multiple_paths(&mut self, paths: &[String]) {
        let media: Vec<String> = paths
            .iter()
            .filter(|p| is_supported_media_path(p) || Path::new(p).is_file())
            .cloned()
            .collect();
        if media.is_empty() {
            return;
        }
        if media.len() == 1 && is_subtitle_path(&media[0]) {
            let _ = self.load_subtitle_file(&media[0]);
            return;
        }
        self.playlist.clear_demos_if_only_demos();
        let first_new_idx = self.playlist.items.len();
        for (i, path) in media.iter().enumerate() {
            if is_subtitle_path(path) {
                let _ = self.load_subtitle_file(path);
                continue;
            }
            if i == 0 {
                let meta = probe_media(path);
                self.playlist.add(PlaylistItem::from_metadata(&meta));
            } else {
                let title = path.rsplit(['/', '\\']).next().unwrap_or(path);
                let fmt = crate::demux::detect::detect_format_from_path(path);
                let mut item = PlaylistItem::new(path, title, "Queued Media", fmt, 0, 0, 0);
                item.is_live_file = true;
                self.playlist.add(item);
            }
        }
        self.select_track(first_new_idx);
    }

    #[cfg(feature = "std")]
    pub fn open_directory(&mut self, dir: &str) {
        let mut files = Vec::new();
        collect_media_files_recursive(Path::new(dir), &mut files, 0);
        files.sort();
        if !files.is_empty() {
            self.open_multiple_paths(&files);
        } else {
            self.last_status_msg = Some("No supported media files found in folder".to_string());
        }
    }

    #[cfg(feature = "std")]
    fn open_media_internal(&mut self, path_or_url: &str, add_to_playlist: bool) {
        self.session = None;
        self.audio.has_live_pcm = false;
        self.dropped_frames = 0;
        self.ab_loop = None;
        self.position_ms = 0;

        let meta = probe_media(path_or_url);
        let item = PlaylistItem::from_metadata(&meta);

        if add_to_playlist {
            self.playlist.clear_demos_if_only_demos();
            if let Some(existing) = self.playlist.items.iter().position(|i| i.path == path_or_url) {
                self.playlist.items[existing] = item;
                self.playlist.current_index = existing;
            } else {
                self.playlist.add(item);
                self.playlist.current_index = self.playlist.items.len() - 1;
            }
        } else if let Some(cur) = self.playlist.items.get_mut(self.playlist.current_index) {
            *cur = item;
        }

        self.push_recent(path_or_url);

        // Setup subtitles: discover sidecars + embedded tracks
        self.subtitle_tracks.clear();
        let sidecars = discover_sidecar_subtitles(path_or_url);
        for sc in &sidecars {
            if let Ok(text) = std::fs::read_to_string(sc) {
                self.subtitle_tracks.push(SubtitleTrack::parse_auto(&text, sc));
            }
        }
        for st in &meta.subtitle_tracks {
            let label = if !st.title.is_empty() {
                format!("{} [{}]", st.title, st.language)
            } else {
                format!("Track {} [{} {}]", st.index + 1, st.language, st.codec)
            };
            if let Some(track) = extract_embedded_subtitle(path_or_url, st.index, &label, &st.language) {
                self.subtitle_tracks.push(track);
            }
        }
        if let Some(first_sub) = self.subtitle_tracks.first().cloned() {
            self.subtitles = first_sub;
            self.active_subtitle_idx = Some(0);
        } else {
            self.subtitles = SubtitleTrack::empty();
            self.active_subtitle_idx = None;
        }

        self.active_audio_track = if meta.audio_codec != "None" { Some(0) } else { None };
        self.active_video_track = if !meta.format.is_audio_only() && meta.width > 0 {
            Some(0)
        } else {
            None
        };

        // Download Universal Codec Pack automatically whenever a non-AVC1 or unsupported video plays
        if !meta.format.is_audio_only() && meta.width > 0 && find_ffmpeg_bin().is_none() {
            let pure_rust_ok = matches!(meta.format, MediaFormat::Mp4 | MediaFormat::Mov)
                && mp4_has_avc1(path_or_url);
            if !pure_rust_ok {
                self.ensure_universal_codecs();
            }
        }

        let sess = ActiveMediaSession::start(
            path_or_url.to_string(),
            meta.clone(),
            0,
            self.speed,
            self.active_audio_track,
            self.max_decode_width,
        );
        self.metadata = sess.metadata.clone();
        self.fps = if self.metadata.fps > 0.1 {
            self.metadata.fps
        } else {
            60.0
        };
        self.session = Some(sess);
        self.state = PlaybackState::Playing;
        self.last_status_msg = Some(format!("Playing: {}", self.metadata.title));
    }

    #[cfg(feature = "std")]
    pub fn ensure_universal_codecs(&mut self) {
        spawn_ffmpeg_downloader(Arc::clone(&self.codec_status));
    }

    #[cfg(feature = "std")]
    pub fn codec_download_progress(&self) -> Option<(u8, String)> {
        if let Ok(s) = self.codec_status.lock() {
            if let CodecStatus::Downloading {
                progress_pct,
                stage,
            } = &*s
            {
                return Some((*progress_pct, stage.clone()));
            }
        }
        None
    }

    #[cfg(feature = "std")]
    pub fn load_subtitle_file(&mut self, path: &str) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Cannot read subtitle: {e}"))?;
        let text = String::from_utf8_lossy(&bytes);
        let track = SubtitleTrack::parse_auto(&text, path);
        if track.cues.is_empty() {
            return Err("No valid subtitle cues found".to_string());
        }
        self.add_subtitle_track(track);
        self.last_status_msg = Some(format!(
            "Subtitle loaded: {}",
            path.rsplit(['/', '\\']).next().unwrap_or(path)
        ));
        Ok(())
    }

    pub fn add_subtitle_track(&mut self, mut track: SubtitleTrack) {
        track.is_enabled = true;
        self.subtitle_tracks.push(track.clone());
        self.active_subtitle_idx = Some(self.subtitle_tracks.len() - 1);
        self.subtitles = track;
    }

    pub fn select_subtitle_track(&mut self, idx: Option<usize>) {
        self.active_subtitle_idx = idx;
        if let Some(i) = idx {
            if let Some(tr) = self.subtitle_tracks.get(i).cloned() {
                self.subtitles = tr;
                self.subtitles.is_enabled = true;
                return;
            }
        }
        self.subtitles.is_enabled = false;
    }

    pub fn select_audio_track(&mut self, idx: Option<usize>) {
        self.active_audio_track = idx;
        self.audio.is_muted = idx.is_none();
        #[cfg(feature = "std")]
        if let Some(ref mut sess) = self.session {
            sess.restart_at(
                self.position_ms,
                self.speed,
                self.active_audio_track,
                self.max_decode_width,
            );
        }
    }

    /// Pulls live stereo PCM samples for the OS audio device, applying 10-band EQ, 200% boost & FFT.
    #[cfg(feature = "std")]
    pub fn fill_audio_output(&mut self, out_stereo: &mut [f32]) {
        out_stereo.fill(0.0);
        if self.state != PlaybackState::Playing {
            return;
        }
        let Some(ref sess) = self.session else {
            return;
        };
        if !sess.has_audio_stream {
            return;
        }

        let needed = out_stereo.len();
        let taken = if let Ok(mut q) = sess.audio_shared.ring.lock() {
            let avail = q.len().min(needed);
            for (i, sample) in q.drain(..avail).enumerate() {
                out_stereo[i] = sample;
            }
            avail
        } else {
            0
        };

        if taken > 0 {
            let frames_taken = (taken / 2) as u64;
            sess.audio_shared
                .played_frames
                .fetch_add(frames_taken, std::sync::atomic::Ordering::Release);
            self.audio
                .process_pcm_stereo(&mut out_stereo[..taken], crate::codec::media_backend::TARGET_AUDIO_RATE);
            self.audio.feed_pcm_for_spectrum(&out_stereo[..taken]);
        }
    }

    /// Saves a PNG snapshot of the current video frame.
    #[cfg(feature = "std")]
    pub fn take_snapshot(&self) -> Result<String, String> {
        let frame = self
            .current_video_frame(1280, 720)
            .ok_or_else(|| "No active video frame to snapshot".to_string())?;
        let base_dir = std::env::var_os("HOME")
            .map(|h| Path::new(&h).join("Pictures/Spectra"))
            .unwrap_or_else(|| Path::new(".").to_path_buf());
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let out_path = base_dir.join(format!("spectra_snapshot_{ts}.png"));
        save_frame_png(&frame, &out_path)?;
        Ok(out_path.to_string_lossy().to_string())
    }

    /// Returns current decoded video frame zero-copy (Arc<VideoFrame>), or None in standby/audio-only.
    pub fn current_video_frame(&self, target_w: u32, target_h: u32) -> Option<Arc<VideoFrame>> {
        if !self.metadata.is_live_file || self.metadata.format.is_audio_only() {
            return None;
        }
        if self.active_video_track.is_none() {
            return None;
        }

        #[cfg(feature = "std")]
        if let Some(ref sess) = self.session {
            if let Some(ref decoded) = sess.latest_frame {
                if self.video_synth.adjustments.has_color_effects()
                    || self.video_synth.adjustments.has_geometry_effects()
                {
                    let mut frame = (**decoded).clone();
                    self.video_synth.adjustments.apply_to_frame(&mut frame);
                    return Some(Arc::new(frame));
                }
                return Some(Arc::clone(decoded));
            }
            if sess.has_video_stream {
                return Some(Arc::new(VideoFrame::new(target_w.max(2), target_h.max(2), self.position_ms)));
            }
        }

        let _ = (target_w, target_h);
        None
    }

    /// Current active subtitle string, if any.
    pub fn active_subtitle(&self) -> Option<&str> {
        self.subtitles.active_cue_text(self.position_ms)
    }

    fn push_recent(&mut self, path: &str) {
        self.recent_paths.retain(|p| p != path);
        self.recent_paths.insert(0, path.to_string());
        if self.recent_paths.len() > 10 {
            self.recent_paths.truncate(10);
        }
        save_recent_history(&self.recent_paths);
    }
}

#[cfg(feature = "std")]
fn collect_media_files_recursive(dir: &Path, out: &mut Vec<String>, depth: usize) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_media_files_recursive(&p, out, depth + 1);
        } else if p.is_file() {
            let s = p.to_string_lossy().to_string();
            if is_supported_media_path(&s) {
                out.push(s);
            }
        }
    }
}

fn load_recent_history() -> Vec<String> {
    #[cfg(feature = "std")]
    {
        let p = crate::codec::media_backend::spectra_data_dir().join("recent.txt");
        if let Ok(txt) = std::fs::read_to_string(p) {
            return txt
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .take(10)
                .collect();
        }
    }
    Vec::new()
}

fn save_recent_history(items: &[String]) {
    #[cfg(feature = "std")]
    {
        let dir = crate::codec::media_backend::spectra_data_dir();
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("recent.txt");
        let _ = std::fs::write(p, items.join("\n"));
    }
    #[cfg(not(feature = "std"))]
    let _ = items;
}
