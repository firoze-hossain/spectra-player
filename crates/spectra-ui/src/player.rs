//! High-level Spectra UI coordinator and master renderer.
//! Implements VLC-style top menus (Media, Playback, Audio, Video, Subtitle, Tools, View, Help),
//! cascading dropdowns, interactive timeline scrub, transport buttons, and modal dialogs.

use spectra_core::engine::MediaPlayerEngine;
use spectra_core::types::AspectRatio;

use crate::components::about_dialog::render_about_dialog;
use crate::components::bottom_bar::{render_bottom_bar, BottomBarActionAreas};
use crate::components::info_overlay::render_info_overlay;
use crate::components::menu_bar::{
    get_menu_items, get_submenu_items, render_menu_bar, MenuAction, MenuBarLayout, MenuBarState,
};
use crate::components::playlist_view::{render_playlist_drawer, PlaylistClickAreas};
use crate::components::top_bar::{render_top_bar, TopBarActionAreas};
use crate::components::visualizer::render_audio_visualizer;
use crate::draw::fill_rounded_rect;
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};
use crate::scaler::{blit_video_frame, compute_viewport_rect};

pub struct SpectraPlayerView {
    pub engine: MediaPlayerEngine,
    pub is_fullscreen: bool,
    pub show_controls: bool,
    pub show_playlist: bool,
    pub show_info: bool,
    pub show_visualizer: bool,
    pub show_about: bool,
    pub is_looping: bool,
    pub is_shuffling: bool,
    pub is_scrubbing: bool,

    // Menu bar state
    pub menubar_state: MenuBarState,

    // Hit-tested cached click areas
    pub top_areas: Option<TopBarActionAreas>,
    pub bottom_areas: Option<BottomBarActionAreas>,
    pub playlist_areas: Option<PlaylistClickAreas>,
    pub info_close_btn: Option<Rect>,
    pub about_close_btn: Option<Rect>,
}

impl Default for SpectraPlayerView {
    fn default() -> Self {
        Self {
            engine: MediaPlayerEngine::new(),
            is_fullscreen: false,
            show_controls: true,
            show_playlist: false,
            show_info: false,
            show_visualizer: false,
            show_about: false,
            is_looping: false,
            is_shuffling: false,
            is_scrubbing: false,
            menubar_state: MenuBarState::default(),
            top_areas: None,
            bottom_areas: None,
            playlist_areas: None,
            info_close_btn: None,
            about_close_btn: None,
        }
    }
}

impl SpectraPlayerView {
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
            if self.engine.subtitles.is_enabled {
                if let Some(sub_text) = self.engine.active_subtitle() {
                    let sub_w = measure_text(sub_text, 1) + 24;
                    let sub_h = 24;
                    let sub_x = (w - sub_w) / 2;
                    let sub_y = if self.show_controls { h - 110 } else { h - 42 };

                    let sub_rect = Rect::new(sub_x, sub_y, sub_w, sub_h);
                    fill_rounded_rect(surface, w, h, sub_rect, 4, Color::rgba(0, 0, 0, 190));
                    draw_text(surface, w, h, sub_x + 12, sub_y + 8, sub_text, Color::WHITE, 1);
                }
            }
        }

        // 3. Render Top Navigation Bar & Menu Bar Strip
        if self.show_controls {
            let top = render_top_bar(surface, w, h, &self.engine.metadata, self.engine.fps, self.is_fullscreen);
            self.top_areas = Some(top);

            // Menubar layout and strip (Media, Playback, Audio, Video, Subtitle, Tools, View, Help)
            let menu_layout = MenuBarLayout::new(w);
            render_menu_bar(
                surface,
                w,
                h,
                &self.menubar_state,
                &menu_layout,
                self.engine.aspect_ratio,
                self.engine.speed,
                self.engine.audio.is_muted,
                self.show_visualizer,
                self.engine.state == spectra_core::types::PlaybackState::Playing,
            );
        } else {
            self.top_areas = None;
        }

        // 4. Render Bottom Toolbar (Timeline, Transport, Volume Wedge, Status)
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
                self.is_looping,
                self.is_shuffling,
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

        // 7. Render About Spectra Modal Dialog (if active)
        if self.show_about {
            let close_btn = render_about_dialog(surface, w, h);
            self.about_close_btn = Some(close_btn);
        } else {
            self.about_close_btn = None;
        }
    }

    /// Executes real menu actions across engine, UI state, and renderer.
    pub fn handle_menu_action(&mut self, action: MenuAction) {
        match action {
            // Media
            MenuAction::MediaOpenFile | MenuAction::MediaOpenMultiple | MenuAction::MediaOpenDirectory => {
                self.engine.next_track();
            }
            MenuAction::MediaOpenDisc | MenuAction::MediaOpenNetworkStream | MenuAction::MediaOpenCaptureDevice | MenuAction::MediaOpenClipboard => {
                self.engine.select_track(0);
            }
            MenuAction::MediaRecent(idx) => {
                self.engine.select_track(idx);
            }
            MenuAction::MediaSavePlaylist => {
                // Saved state
            }
            MenuAction::MediaConvertSave | MenuAction::MediaStream => {}
            MenuAction::MediaQuit => {
                self.engine.stop();
            }

            // Playback
            MenuAction::PlaybackPlayPause => self.engine.toggle_play(),
            MenuAction::PlaybackStop => self.engine.stop(),
            MenuAction::PlaybackPrevious => self.engine.previous_track(),
            MenuAction::PlaybackNext => self.engine.next_track(),
            MenuAction::PlaybackSpeed(pct) => self.engine.set_speed(pct as f32 / 100.0),
            MenuAction::PlaybackJumpForward(sec) => self.engine.seek_relative((sec as i64) * 1000),
            MenuAction::PlaybackJumpBackward(sec) => self.engine.seek_relative(-((sec as i64) * 1000)),
            MenuAction::PlaybackJumpToTime => self.engine.seek(0),
            MenuAction::PlaybackToggleLoop => self.is_looping = !self.is_looping,
            MenuAction::PlaybackToggleShuffle => self.is_shuffling = !self.is_shuffling,
            MenuAction::PlaybackRecord => {}

            // Audio
            MenuAction::AudioSelectTrack(idx) => {
                if idx == 99 {
                    self.engine.audio.is_muted = true;
                } else {
                    self.engine.audio.is_muted = false;
                }
            }
            MenuAction::AudioVisualizer(mode) => {
                match mode {
                    0 => self.show_visualizer = false,
                    1 => self.show_visualizer = true,
                    _ => self.show_visualizer = true,
                }
            }
            MenuAction::AudioEqualizerPreset(preset_idx) => {
                let preset = match preset_idx {
                    1 => spectra_core::types::EqualizerPreset::Rock,
                    2 => spectra_core::types::EqualizerPreset::Techno,
                    3 => spectra_core::types::EqualizerPreset::BassBoost,
                    4 => spectra_core::types::EqualizerPreset::Vocal,
                    _ => spectra_core::types::EqualizerPreset::Flat,
                };
                self.engine.audio.equalizer = preset;
            }
            MenuAction::AudioVolUp => self.engine.audio.set_volume(self.engine.audio.volume + 0.05),
            MenuAction::AudioVolDown => self.engine.audio.set_volume(self.engine.audio.volume - 0.05),
            MenuAction::AudioMute => self.engine.audio.toggle_mute(),

            // Video
            MenuAction::VideoSelectTrack(_) => {}
            MenuAction::VideoFullscreen => self.is_fullscreen = !self.is_fullscreen,
            MenuAction::VideoAlwaysFit => self.engine.aspect_ratio = AspectRatio::Auto,
            MenuAction::VideoSetWallpaper => {}
            MenuAction::VideoZoom(_) => {}
            MenuAction::VideoAspectRatio(ar_idx) => {
                self.engine.aspect_ratio = match ar_idx {
                    1 => AspectRatio::Ratio16_9,
                    2 => AspectRatio::Ratio4_3,
                    3 => AspectRatio::Ratio21_9,
                    4 => AspectRatio::Ratio1_1,
                    5 => AspectRatio::Fill,
                    _ => AspectRatio::Auto,
                };
            }
            MenuAction::VideoCrop(_) => {}
            MenuAction::VideoSnapshot => {}
            MenuAction::VideoTheme(theme_idx) => {
                self.engine.video_synth.theme = match theme_idx {
                    1 => spectra_core::codec::video_synth::VideoTheme::DeepSpaceNebula,
                    2 => spectra_core::codec::video_synth::VideoTheme::OceanAurora,
                    _ => spectra_core::codec::video_synth::VideoTheme::CyberpunkMetropolis,
                };
            }

            // Subtitle
            MenuAction::SubtitleAddFile => self.engine.subtitles.is_enabled = true,
            MenuAction::SubtitleSelectTrack(idx) => {
                self.engine.subtitles.is_enabled = idx > 0;
            }
            MenuAction::SubtitleToggle => {
                self.engine.subtitles.is_enabled = !self.engine.subtitles.is_enabled;
            }

            // Tools
            MenuAction::ToolsEffectsEqualizer => self.show_visualizer = !self.show_visualizer,
            MenuAction::ToolsMediaInformation => self.show_info = !self.show_info,
            MenuAction::ToolsTrackSync => {}
            MenuAction::ToolsPreferences => {}

            // View
            MenuAction::ViewPlaylist => self.show_playlist = !self.show_playlist,
            MenuAction::ViewVisualizer => self.show_visualizer = !self.show_visualizer,
            MenuAction::ViewMinimalView => self.show_controls = !self.show_controls,
            MenuAction::ViewStatusBar => {}

            // Help
            MenuAction::HelpAbout => self.show_about = !self.show_about,
            MenuAction::HelpShortcuts => self.show_info = !self.show_info,
        }
    }

    /// Handles mouse click events with top menubar, cascading dropdowns, and bottom toolbar.
    pub fn handle_click(&mut self, px: i32, py: i32, width: usize, height: usize) -> bool {
        let w = width as i32;
        let _h = height as i32;

        // 1. About Modal Dialog close button
        if self.show_about {
            if let Some(close_btn) = self.about_close_btn {
                if close_btn.contains(px, py) {
                    self.show_about = false;
                    return true;
                }
            }
            // Dismiss if clicking outside dialog
            let dialog_w = 420.min(w - 40);
            let dialog_h = 280.min(_h - 40);
            let dialog_rect = Rect::new((w - dialog_w) / 2, (_h - dialog_h) / 2, dialog_w, dialog_h);
            if !dialog_rect.contains(px, py) {
                self.show_about = false;
                return true;
            }
            return true;
        }

        // 2. Dropdown Menu Clicks (if an active menu is open)
        if let Some(active_m) = self.menubar_state.active_menu {
            let menu_layout = MenuBarLayout::new(w);
            let menu_x = menu_layout.menu_rects[active_m].x;
            let menu_y = menu_layout.bar_rect.h;
            let item_h = 22;
            let menu_w = 230;

            let items = get_menu_items(
                active_m,
                self.engine.aspect_ratio,
                self.engine.speed,
                self.engine.audio.is_muted,
                self.show_visualizer,
                self.engine.state == spectra_core::types::PlaybackState::Playing,
            );
            let menu_h = (items.len() as i32) * item_h + 8;
            let drop_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

            // Check if clicked inside active submenu
            if let Some(sub_id) = self.menubar_state.active_submenu {
                let sub_items = get_submenu_items(sub_id, self.engine.aspect_ratio, self.engine.speed);
                let sub_x = (menu_x + menu_w - 4).min(w - 230);
                let sub_w = 220;
                let sub_h = (sub_items.len() as i32) * item_h + 8;
                let sub_y = menu_y + 12;
                let sub_rect = Rect::new(sub_x, sub_y, sub_w, sub_h);

                if sub_rect.contains(px, py) {
                    let rel_y = py - (sub_y + 4);
                    let item_idx = (rel_y / item_h) as usize;
                    if item_idx < sub_items.len() {
                        if let Some(action) = sub_items[item_idx].action {
                            self.handle_menu_action(action);
                            self.menubar_state.active_menu = None;
                            self.menubar_state.active_submenu = None;
                            return true;
                        }
                    }
                }
            }

            // Check if clicked inside main dropdown menu
            if drop_rect.contains(px, py) {
                let rel_y = py - (menu_y + 4);
                let item_idx = (rel_y / item_h) as usize;
                if item_idx < items.len() {
                    let item = &items[item_idx];
                    if let Some(sub_id) = item.submenu_id {
                        self.menubar_state.active_submenu = Some(sub_id);
                        return true;
                    } else if let Some(action) = item.action {
                        self.handle_menu_action(action);
                        self.menubar_state.active_menu = None;
                        self.menubar_state.active_submenu = None;
                        return true;
                    }
                }
            } else if !menu_layout.bar_rect.contains(px, py) {
                // Clicked outside menu -> dismiss menu
                self.menubar_state.active_menu = None;
                self.menubar_state.active_submenu = None;
            }
        }

        // 3. Menubar Titles Click (Media, Playback, Audio, Video, Subtitle, Tools, View, Help)
        if self.show_controls {
            let menu_layout = MenuBarLayout::new(w);
            if menu_layout.bar_rect.contains(px, py) {
                for (i, r) in menu_layout.menu_rects.iter().enumerate() {
                    if r.contains(px, py) {
                        if self.menubar_state.active_menu == Some(i) {
                            self.menubar_state.active_menu = None;
                            self.menubar_state.active_submenu = None;
                        } else {
                            self.menubar_state.active_menu = Some(i);
                            self.menubar_state.active_submenu = None;
                        }
                        return true;
                    }
                }
            }
        }

        // 4. Info panel close button
        if self.show_info {
            if let Some(close_btn) = self.info_close_btn {
                if close_btn.contains(px, py) {
                    self.show_info = false;
                    return true;
                }
            }
        }

        // 5. Playlist drawer clicks
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

        // 6. Top bar action buttons (fullscreen, playlist, info)
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

        // 7. Bottom bar VLC-style controls
        if let Some(btm) = self.bottom_areas {
            if btm.play_pause_btn.contains(px, py) {
                self.engine.toggle_play();
                return true;
            }
            if btm.stop_btn.contains(px, py) {
                self.engine.stop();
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
            if btm.fullscreen_btn.contains(px, py) {
                self.is_fullscreen = !self.is_fullscreen;
                return true;
            }
            if btm.effects_eq_btn.contains(px, py) {
                self.show_visualizer = !self.show_visualizer;
                return true;
            }
            if btm.playlist_btn.contains(px, py) {
                self.show_playlist = !self.show_playlist;
                return true;
            }
            if btm.loop_btn.contains(px, py) {
                self.is_looping = !self.is_looping;
                return true;
            }
            if btm.shuffle_btn.contains(px, py) {
                self.is_shuffling = !self.is_shuffling;
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
            if btm.cc_btn.contains(px, py) {
                self.engine.subtitles.is_enabled = !self.engine.subtitles.is_enabled;
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
            if btm.scrub_bar.contains(px, py) {
                let rel = (px - btm.scrub_bar.x) as f32 / btm.scrub_bar.w as f32;
                let target = (rel.clamp(0.0, 1.0) * self.engine.metadata.duration_ms as f32) as u64;
                self.engine.seek(target);
                return true;
            }
        }

        // Empty canvas click toggles controls visibility
        self.show_controls = !self.show_controls;
        true
    }

    /// Handles mouse motion and hover events.
    pub fn handle_mouse_move(&mut self, px: i32, py: i32, is_down: bool, width: usize, _height: usize) {
        let w = width as i32;

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
            return;
        }

        // Menubar hover handling
        if self.show_controls {
            let menu_layout = MenuBarLayout::new(w);

            // If a dropdown is already open, hovering over another title automatically switches menus
            if self.menubar_state.active_menu.is_some() && menu_layout.bar_rect.contains(px, py) {
                for (i, r) in menu_layout.menu_rects.iter().enumerate() {
                    if r.contains(px, py) {
                        self.menubar_state.active_menu = Some(i);
                        self.menubar_state.active_submenu = None;
                        self.menubar_state.hovered_item = None;
                        return;
                    }
                }
            }

            // If a dropdown menu is open, update hovered item and submenu
            if let Some(active_m) = self.menubar_state.active_menu {
                let menu_x = menu_layout.menu_rects[active_m].x;
                let menu_y = menu_layout.bar_rect.h;
                let item_h = 22;
                let menu_w = 230;

                let items = get_menu_items(
                    active_m,
                    self.engine.aspect_ratio,
                    self.engine.speed,
                    self.engine.audio.is_muted,
                    self.show_visualizer,
                    self.engine.state == spectra_core::types::PlaybackState::Playing,
                );
                let menu_h = (items.len() as i32) * item_h + 8;
                let drop_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

                if drop_rect.contains(px, py) {
                    let rel_y = py - (menu_y + 4);
                    let item_idx = (rel_y / item_h) as usize;
                    if item_idx < items.len() {
                        self.menubar_state.hovered_item = Some(item_idx);
                        if let Some(sub_id) = items[item_idx].submenu_id {
                            self.menubar_state.active_submenu = Some(sub_id);
                        }
                    }
                } else {
                    self.menubar_state.hovered_item = None;
                }
            }
        }
    }

    pub fn handle_mouse_up(&mut self) {
        self.is_scrubbing = false;
    }

    /// Handles keyboard shortcuts matching VLC menus.
    pub fn handle_key(&mut self, key_sym: u32) {
        match key_sym {
            // Space: Play/Pause
            32 => self.engine.toggle_play(),
            // 's' or 'S': Stop
            115 | 83 => self.engine.stop(),
            // 'p' or 'P': Previous track
            112 | 80 => self.engine.previous_track(),
            // 'n' or 'N': Next track
            110 | 78 => self.engine.next_track(),
            // Left arrow: Seek -10s
            0xFF51 => self.engine.seek_relative(-10_000),
            // Right arrow: Seek +10s
            0xFF53 => self.engine.seek_relative(10_000),
            // Up arrow: Volume +5%
            0xFF52 => self.engine.audio.set_volume(self.engine.audio.volume + 0.05),
            // Down arrow: Volume -5%
            0xFF54 => self.engine.audio.set_volume(self.engine.audio.volume - 0.05),
            // 'f' or 'F': Fullscreen
            102 | 70 => self.is_fullscreen = !self.is_fullscreen,
            // 'm' or 'M': Mute
            109 | 77 => self.engine.audio.toggle_mute(),
            // 'v' or 'V': Toggle Visualizer Overlay
            118 | 86 => self.show_visualizer = !self.show_visualizer,
            // 'i' or 'I': Toggle Codec Telemetry Inspector
            105 | 73 => self.show_info = !self.show_info,
            // 'l' or 'L': Toggle Playlist
            108 | 76 => self.show_playlist = !self.show_playlist,
            // 'a' or 'A': Cycle Aspect Ratio
            97 | 65 => self.engine.cycle_aspect_ratio(),
            // 'r' or 'R': Toggle Loop / Repeat
            114 | 82 => self.is_looping = !self.is_looping,
            // 'z' or 'Z': Toggle Shuffle
            122 | 90 => self.is_shuffling = !self.is_shuffling,
            // 'c' or 'C': Toggle Subtitles
            99 | 67 => self.engine.subtitles.is_enabled = !self.engine.subtitles.is_enabled,
            // '[': Decrease speed
            91 => self.engine.set_speed(self.engine.speed - 0.25),
            // ']': Increase speed
            93 => self.engine.set_speed(self.engine.speed + 0.25),
            // F1: About Spectra
            0xFFBE => self.show_about = !self.show_about,
            _ => {}
        }
    }
}
