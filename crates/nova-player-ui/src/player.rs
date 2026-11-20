//! High-level Nova Player UI coordinator and master renderer.

use nova_player_core::engine::MediaPlayerEngine;
use crate::components::bottom_bar::{render_bottom_bar, BottomBarActionAreas};
use crate::components::info_overlay::render_info_overlay;
use crate::components::playlist_view::{render_playlist_drawer, PlaylistClickAreas};
use crate::components::top_bar::{render_top_bar, TopBarActionAreas};
use crate::components::visualizer::render_audio_visualizer;
use crate::draw::{fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};
use crate::scaler::{blit_video_frame, compute_viewport_rect};

pub struct NovaPlayerView {
    pub engine: MediaPlayerEngine,
    pub is_fullscreen: bool,
    pub show_controls: bool,
    pub show_playlist: bool,
    pub show_info: bool,
    pub show_visualizer: bool,
    pub is_scrubbing: bool,
    // Hit-tested cached click areas
    pub top_areas: Option<TopBarActionAreas>,
    pub bottom_areas: Option<BottomBarActionAreas>,
    pub playlist_areas: Option<PlaylistClickAreas>,
    pub info_close_btn: Option<Rect>,
}

impl Default for NovaPlayerView {
    fn default() -> Self {
        Self {
            engine: MediaPlayerEngine::new(),
            is_fullscreen: false,
            show_controls: true,
            show_playlist: false,
            show_info: false,
            show_visualizer: false,
            is_scrubbing: false,
            top_areas: None,
            bottom_areas: None,
            playlist_areas: None,
            info_close_btn: None,
        }
    }
}

impl NovaPlayerView {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ticks engine and updates playback simulation.
    pub fn tick(&mut self, delta_ms: u64) {
        self.engine.tick(delta_ms);
    }

    /// Renders the complete player frame into target 32-bit XRGB surface.
    pub fn render(&mut self, surface: &mut [u32], width: usize, height: usize) {
        let w = width as i32;
        let h = height as i32;

        let is_audio_only = self.engine.metadata.format.is_audio_only();

        // 1. Render Video Canvas or Audio Turntable
        if is_audio_only {
            render_audio_visualizer(
                surface,
                w,
                h,
                &self.engine.audio.spectrum_levels,
                &self.engine.audio.spectrum_peaks,
                &self.engine.metadata,
                self.engine.position_ms,
                true,
            );
        } else {
            // Compute viewport based on aspect ratio
            let vp = compute_viewport_rect(
                w,
                h,
                self.engine.aspect_ratio,
                self.engine.metadata.width,
                self.engine.metadata.height,
            );

            // Render synthesized/decoded video frame
            if let Some(frame) = self.engine.current_video_frame(vp.w as u32, vp.h as u32) {
                blit_video_frame(surface, w, h, vp, &frame);
            }

            // If audio visualizer toggle is active, overlay spectrum bars
            if self.show_visualizer {
                render_audio_visualizer(
                    surface,
                    w,
                    h,
                    &self.engine.audio.spectrum_levels,
                    &self.engine.audio.spectrum_peaks,
                    &self.engine.metadata,
                    self.engine.position_ms,
                    false,
                );
            }

            // 2. Render Subtitles Overlay (bottom center above control bar)
            if let Some(sub_text) = self.engine.active_subtitle() {
                let sub_w = measure_text(sub_text, 1) + 24;
                let sub_h = 24;
                let sub_x = (w - sub_w) / 2;
                let sub_y = if self.show_controls { h - 128 } else { h - 48 };

                let sub_rect = Rect::new(sub_x, sub_y, sub_w, sub_h);
                fill_rounded_rect(surface, w, h, sub_rect, 4, Color::rgba(0, 0, 0, 190));
                draw_text(surface, w, h, sub_x + 12, sub_y + 8, sub_text, Color::WHITE, 1);
            }
        }

        // 3. Render Top Bar (if controls visible)
        if self.show_controls {
            let top = render_top_bar(surface, w, h, &self.engine.metadata, self.engine.fps, self.is_fullscreen);
            self.top_areas = Some(top);
        } else {
            self.top_areas = None;
        }

        // 4. Render Bottom Bar (if controls visible)
        if self.show_controls {
            let btm = render_bottom_bar(
                surface,
                w,
                h,
                self.engine.state,
                self.engine.position_ms,
                self.engine.metadata.duration_ms,
                self.engine.audio.volume,
                self.engine.audio.is_muted,
                self.engine.speed,
                self.engine.aspect_ratio,
                self.show_visualizer,
                self.engine.subtitles.is_enabled,
            );
            self.bottom_areas = Some(btm);
        } else {
            self.bottom_areas = None;
        }

        // 5. Render Sliding Playlist Drawer (if active)
        if self.show_playlist {
            let pl_areas = render_playlist_drawer(surface, w, h, &self.engine.playlist);
            self.playlist_areas = Some(pl_areas);
        } else {
            self.playlist_areas = None;
        }

        // 6. Render Media Info & Codec Inspector (if active)
        if self.show_info {
            let close_rect = render_info_overlay(
                surface,
                w,
                h,
                &self.engine.metadata,
                self.engine.fps,
                self.engine.dropped_frames,
                self.engine.aspect_ratio,
                self.engine.audio.volume,
            );
            self.info_close_btn = Some(close_rect);
        } else {
            self.info_close_btn = None;
        }
    }

    /// Handles mouse click event.
    pub fn handle_click(&mut self, px: i32, py: i32) -> bool {
        // 1. Info panel close button
        if self.show_info {
            if let Some(close_btn) = self.info_close_btn {
                if close_btn.contains(px, py) {
                    self.show_info = false;
                    return true;
                }
            }
        }

        // 2. Playlist drawer clicks
        if self.show_playlist {
            if let Some(ref pl) = self.playlist_areas {
                if pl.close_btn.contains(px, py) {
                    self.show_playlist = false;
                    return true;
                }
                for &(idx, rect) in &pl.item_rects {
                    if rect.contains(px, py) {
                        self.engine.select_track(idx);
                        return true;
                    }
                }
            }
        }

        // 3. Top bar action buttons
        if let Some(top) = self.top_areas {
            if top.fullscreen_btn.contains(px, py) {
                self.is_fullscreen = !self.is_fullscreen;
                return true;
            }
            if top.playlist_btn.contains(px, py) {
                self.show_playlist = !self.show_playlist;
                return true;
            }
            if top.info_btn.contains(px, py) {
                self.show_info = !self.show_info;
                return true;
            }
        }

        // 4. Bottom bar controls
        if let Some(btm) = self.bottom_areas {
            if btm.play_pause_btn.contains(px, py) {
                self.engine.toggle_play();
                return true;
            }
            if btm.prev_btn.contains(px, py) {
                self.engine.previous_track();
                return true;
            }
            if btm.next_btn.contains(px, py) {
                self.engine.next_track();
                return true;
            }
            if btm.step_back_btn.contains(px, py) {
                self.engine.seek_relative(-10_000);
                return true;
            }
            if btm.step_fwd_btn.contains(px, py) {
                self.engine.seek_relative(10_000);
                return true;
            }
            if btm.stop_btn.contains(px, py) {
                self.engine.stop();
                return true;
            }
            if btm.volume_mute_btn.contains(px, py) {
                self.engine.audio.toggle_mute();
                return true;
            }
            if btm.volume_slider.contains(px, py) {
                let rel = (px - btm.volume_slider.x) as f32 / btm.volume_slider.w as f32;
                self.engine.audio.set_volume(rel * 1.25);
                return true;
            }
            if btm.speed_btn.contains(px, py) {
                self.engine.cycle_speed();
                return true;
            }
            if btm.aspect_btn.contains(px, py) {
                self.engine.cycle_aspect_ratio();
                return true;
            }
            if btm.viz_btn.contains(px, py) {
                self.show_visualizer = !self.show_visualizer;
                return true;
            }
            if btm.cc_btn.contains(px, py) {
                self.engine.subtitles.is_enabled = !self.engine.subtitles.is_enabled;
                return true;
            }
            if btm.scrub_bar.contains(px, py) {
                let rel = (px - btm.scrub_bar.x) as f32 / btm.scrub_bar.w as f32;
                let target = (rel.clamp(0.0, 1.0) * self.engine.metadata.duration_ms as f32) as u64;
                self.engine.seek(target);
                return true;
            }
        }

        // Toggle controls visibility on empty canvas click
        self.show_controls = !self.show_controls;
        true
    }

    /// Handles mouse motion and dragging.
    pub fn handle_mouse_move(&mut self, px: i32, py: i32, is_down: bool) {
        if is_down {
            if let Some(btm) = self.bottom_areas {
                if self.is_scrubbing || btm.scrub_bar.contains(px, py) {
                    self.is_scrubbing = true;
                    let rel = (px - btm.scrub_bar.x) as f32 / btm.scrub_bar.w as f32;
                    let target = (rel.clamp(0.0, 1.0) * self.engine.metadata.duration_ms as f32) as u64;
                    self.engine.seek(target);
                } else if btm.volume_slider.contains(px, py) {
                    let rel = (px - btm.volume_slider.x) as f32 / btm.volume_slider.w as f32;
                    self.engine.audio.set_volume(rel * 1.25);
                }
            }
        }
    }

    pub fn handle_mouse_up(&mut self) {
        self.is_scrubbing = false;
    }

    /// Handles keyboard shortcut commands.
    pub fn handle_key(&mut self, key_sym: u32) {
        match key_sym {
            // Space: Play/Pause
            32 => self.engine.toggle_play(),
            // Left arrow: Seek -5s
            0xFF51 => self.engine.seek_relative(-5_000),
            // Right arrow: Seek +5s
            0xFF53 => self.engine.seek_relative(5_000),
            // Up arrow: Volume +5%
            0xFF52 => self.engine.audio.set_volume(self.engine.audio.volume + 0.05),
            // Down arrow: Volume -5%
            0xFF54 => self.engine.audio.set_volume(self.engine.audio.volume - 0.05),
            // 'f' or 'F': Fullscreen
            102 | 70 => self.is_fullscreen = !self.is_fullscreen,
            // 'm' or 'M': Mute
            109 | 77 => self.engine.audio.toggle_mute(),
            // 'v' or 'V': Toggle Visualizer
            118 | 86 => self.show_visualizer = !self.show_visualizer,
            // 'i' or 'I': Toggle Info
            105 | 73 => self.show_info = !self.show_info,
            // 'l' or 'L': Toggle Playlist
            108 | 76 => self.show_playlist = !self.show_playlist,
            // 'a' or 'A': Aspect ratio
            97 | 65 => self.engine.cycle_aspect_ratio(),
            // 's' or 'S': Subtitles
            115 | 83 => self.engine.subtitles.is_enabled = !self.engine.subtitles.is_enabled,
            // '[': Decrease speed
            91 => self.engine.set_speed(self.engine.speed - 0.25),
            // ']': Increase speed
            93 => self.engine.set_speed(self.engine.speed + 0.25),
            _ => {}
        }
    }
}
