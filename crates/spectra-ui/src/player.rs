//! High-level Spectra UI coordinator and master renderer.
//! Implements dynamic VLC-style top menus, cascading dropdowns, interactive timeline scrub with hover preview,
//! 10-band EQ & Video Effects studio, Track Sync, Network URL / Jump modals, OSD toasts, and multi-line subtitles.

use spectra_core::codec::audio_dsp::EQ_BANDS;
use spectra_core::engine::MediaPlayerEngine;
use spectra_core::playlist::LoopMode;
use spectra_core::types::{AspectRatio, AudioVisualizerMode, CropMode, EqualizerPreset, PlaybackState};

use crate::components::about_dialog::render_about_dialog;
use crate::components::bottom_bar::{format_time, render_bottom_bar, BottomBarActionAreas};
use crate::components::effects_dialog::{render_effects_dialog, EffectsDialogAreas, EffectsTab};
use crate::components::info_overlay::render_info_overlay;
use crate::components::input_modal::{render_input_modal, ActiveModal, ModalActionAreas};
use crate::components::menu_bar::{
    get_menu_items, get_submenu_items, render_menu_bar, MenuAction, MenuBarLayout, MenuBarState,
};
use crate::components::playlist_view::{render_playlist_drawer, PlaylistClickAreas};
use crate::components::top_bar::{render_top_bar, TopBarActionAreas};
use crate::components::visualizer::render_audio_visualizer;
use crate::draw::{draw_circle, draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};
use crate::scaler::{blit_video_frame, compute_viewport_rect_zoom};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    OpenFileDialog,
    OpenMultipleDialog,
    OpenDirectoryDialog,
    OpenSubtitleDialog,
    SavePlaylistDialog,
    PasteFromClipboard,
    TakeSnapshot,
    ToggleOsFullscreen,
    QuitApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    None,
    Scrub,
    Volume,
    EqBand(usize),
    VidBrightness,
    VidContrast,
    VidSaturation,
    VidGamma,
}

#[derive(Debug, Clone)]
pub struct IdleStageAreas {
    pub open_file_btn: Rect,
    pub open_folder_btn: Rect,
    pub open_stream_btn: Rect,
    pub recent_rects: Vec<(usize, Rect)>,
}

pub struct SpectraPlayerView {
    pub engine: MediaPlayerEngine,
    pub is_fullscreen: bool,
    pub show_controls: bool,
    pub show_playlist: bool,
    pub show_info: bool,
    pub show_visualizer: bool,
    pub show_about: bool,
    pub show_effects: bool,
    pub effects_tab: EffectsTab,
    pub active_modal: ActiveModal,
    pub modal_input: String,
    pub is_looping: bool,
    pub is_shuffling: bool,
    pub is_scrubbing: bool,
    pub auto_hide_fullscreen: bool,
    pub playlist_scroll: usize,
    pub pending_commands: Vec<UiCommand>,

    // Menu bar state
    pub menubar_state: MenuBarState,

    // Cached hit-tested areas
    pub top_areas: Option<TopBarActionAreas>,
    pub bottom_areas: Option<BottomBarActionAreas>,
    pub playlist_areas: Option<PlaylistClickAreas>,
    pub effects_areas: Option<EffectsDialogAreas>,
    pub modal_areas: Option<ModalActionAreas>,
    pub idle_areas: Option<IdleStageAreas>,
    pub info_close_btn: Option<Rect>,
    pub about_close_btn: Option<Rect>,

    // OSD Toast & interaction state
    osd_text: Option<String>,
    osd_timer_ms: u64,
    idle_ms: u64,
    hover_pos: Option<(i32, i32)>,
    drag_target: DragTarget,
    last_canvas_click_ms: u64,
    clock_ms: u64,
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
            show_effects: false,
            effects_tab: EffectsTab::AudioEq,
            active_modal: ActiveModal::None,
            modal_input: String::new(),
            is_looping: false,
            is_shuffling: false,
            is_scrubbing: false,
            auto_hide_fullscreen: true,
            playlist_scroll: 0,
            pending_commands: Vec::new(),
            menubar_state: MenuBarState::default(),
            top_areas: None,
            bottom_areas: None,
            playlist_areas: None,
            effects_areas: None,
            modal_areas: None,
            idle_areas: None,
            info_close_btn: None,
            about_close_btn: None,
            osd_text: None,
            osd_timer_ms: 0,
            idle_ms: 0,
            hover_pos: None,
            drag_target: DragTarget::None,
            last_canvas_click_ms: 0,
            clock_ms: 0,
        }
    }
}

impl SpectraPlayerView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show_osd(&mut self, msg: impl Into<String>) {
        self.osd_text = Some(msg.into());
        self.osd_timer_ms = 2200;
    }

    pub fn take_commands(&mut self) -> Vec<UiCommand> {
        core::mem::take(&mut self.pending_commands)
    }

    pub fn tick(&mut self, delta_ms: u64) {
        self.clock_ms = self.clock_ms.wrapping_add(delta_ms);
        self.engine.tick(delta_ms);
        if let Some(msg) = self.engine.last_status_msg.take() {
            self.show_osd(msg);
        }
        if self.osd_timer_ms > 0 {
            self.osd_timer_ms = self.osd_timer_ms.saturating_sub(delta_ms);
            if self.osd_timer_ms == 0 {
                self.osd_text = None;
            }
        }
        if self.is_fullscreen
            && self.auto_hide_fullscreen
            && self.engine.state == PlaybackState::Playing
            && self.menubar_state.active_menu.is_none()
            && !self.show_effects
            && !self.show_playlist
            && self.active_modal == ActiveModal::None
        {
            self.idle_ms = self.idle_ms.saturating_add(delta_ms);
            if self.idle_ms > 2500 {
                self.show_controls = false;
            }
        } else {
            self.idle_ms = 0;
        }
    }

    pub fn render(&mut self, surface: &mut [u32], width: usize, height: usize) {
        let w = width as i32;
        let h = height as i32;

        let is_idle = !self.engine.metadata.is_live_file;
        let is_audio_only = self.engine.metadata.format.is_audio_only();

        // 1. Render Idle Standby Canvas, Video Canvas, or Audio Studio
        self.idle_areas = None;
        if is_idle {
            self.idle_areas = Some(render_standby_stage(
                surface,
                w,
                h,
                &self.engine.recent_paths,
            ));
        } else if is_audio_only || self.engine.active_video_track.is_none() {
            render_audio_visualizer(
                surface,
                w,
                h,
                &self.engine.audio.spectrum_levels,
                &self.engine.audio.spectrum_peaks,
                &self.engine.audio.waveform_samples,
                &self.engine.metadata,
                self.engine.position_ms,
                true,
                self.engine.audio.visualizer_mode,
            );
        } else {
            let vp = compute_viewport_rect_zoom(
                w,
                h,
                self.engine.aspect_ratio,
                self.engine.metadata.width,
                self.engine.metadata.height,
                self.engine.video_synth.adjustments.zoom,
            );

            if let Some(frame) = self.engine.current_video_frame(vp.w.max(2) as u32, vp.h.max(2) as u32) {
                blit_video_frame(surface, w, h, vp, &frame);
            } else {
                fill_rect(surface, w, h, Rect::new(0, 0, w, h), Color::BLACK);
            }

            if self.show_visualizer && self.engine.audio.visualizer_mode != AudioVisualizerMode::Off {
                render_audio_visualizer(
                    surface,
                    w,
                    h,
                    &self.engine.audio.spectrum_levels,
                    &self.engine.audio.spectrum_peaks,
                    &self.engine.audio.waveform_samples,
                    &self.engine.metadata,
                    self.engine.position_ms,
                    false,
                    self.engine.audio.visualizer_mode,
                );
            }

            // 2. Render Multi-line Subtitles Overlay
            if self.engine.subtitles.is_enabled {
                if let Some(sub_text) = self.engine.active_subtitle() {
                    render_subtitle_overlay(
                        surface,
                        w,
                        h,
                        sub_text,
                        self.engine.subtitles.font_scale,
                        self.show_controls,
                    );
                }
            }
        }

        // 2b. Codec Download Progress / Pending Video Notice (when downloading FFmpeg or buffering video)
        #[cfg(feature = "std")]
        if let Some((pct, stage)) = self.engine.codec_download_progress() {
            let bw = 420.min(w - 40);
            let bx = (w - bw) / 2;
            let by = (h / 2) - 24;
            let br = Rect::new(bx, by, bw, 54);
            fill_rounded_rect(surface, w, h, br, 8, Color::rgba(15, 23, 42, 240));
            draw_rect_outline(surface, w, h, br, 1, Color::rgb(56, 189, 248));
            draw_text(surface, w, h, bx + 14, by + 10, &stage, Color::WHITE, 1);
            let prog_bg = Rect::new(bx + 14, by + 32, bw - 28, 10);
            fill_rounded_rect(surface, w, h, prog_bg, 4, Color::rgb(48, 54, 61));
            let fw = ((bw - 28) * pct as i32) / 100;
            if fw > 0 {
                fill_rounded_rect(surface, w, h, Rect::new(bx + 14, by + 32, fw, 10), 4, Color::rgb(56, 189, 248));
            }
        } else if !is_idle && !is_audio_only && self.engine.active_video_track.is_some() {
            if let Some(ref sess) = self.engine.session {
                if sess.latest_frame.is_none() {
                    if let Ok(er) = sess.last_error.lock() {
                        if let Some(ref msg) = *er {
                            let short = if msg.chars().count() > 54 {
                                let cut: String = msg.chars().take(51).collect();
                                format!("{cut}...")
                            } else {
                                msg.clone()
                            };
                            let bw = (measure_text(&short, 1) + 28).max(320).min(w - 40);
                            let bx = (w - bw) / 2;
                            let by = (h / 2) - 20;
                            let br = Rect::new(bx, by, bw, 40);
                            fill_rounded_rect(surface, w, h, br, 8, Color::rgba(15, 23, 42, 235));
                            draw_rect_outline(surface, w, h, br, 1, Color::rgb(245, 158, 11));
                            draw_text(surface, w, h, bx + 14, by + 15, &short, Color::rgb(251, 191, 36), 1);
                        }
                    }
                }
            }
        }

        // 3. Top Header & Menu Bar
        if self.show_controls {
            let top = render_top_bar(
                surface,
                w,
                h,
                &self.engine.metadata,
                if self.engine.metadata.is_live_file {
                    self.engine.measured_fps
                } else {
                    self.engine.fps
                },
                self.is_fullscreen,
            );
            self.top_areas = Some(top);

            let menu_layout = MenuBarLayout::new(w);
            render_menu_bar(
                surface,
                w,
                h,
                &self.menubar_state,
                &menu_layout,
                &self.engine,
                self.show_visualizer,
            );

            // 4. Bottom Toolbar
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
                self.show_visualizer || self.show_effects,
                self.engine.subtitles.is_enabled,
                self.engine.playlist.loop_mode,
                self.engine.playlist.is_shuffled,
                self.engine.ab_loop,
                self.hover_pos,
            );
            self.bottom_areas = Some(btm);
        } else {
            self.top_areas = None;
            self.bottom_areas = None;
        }

        // 5. Playlist Drawer
        if self.show_playlist {
            self.playlist_areas = Some(render_playlist_drawer(
                surface,
                w,
                h,
                &self.engine.playlist,
                self.playlist_scroll,
            ));
        } else {
            self.playlist_areas = None;
        }

        // 6. Media Info & Codec Telemetry
        if self.show_info {
            self.info_close_btn = Some(render_info_overlay(
                surface,
                w,
                h,
                &self.engine.metadata,
                self.engine.fps,
                self.engine.measured_fps,
                self.engine.dropped_frames,
                self.engine.aspect_ratio,
                self.engine.audio.volume,
                self.engine.speed,
                self.engine.active_subtitle(),
            ));
        } else {
            self.info_close_btn = None;
        }

        // 7. Effects & 10-Band EQ Dialog
        if self.show_effects {
            self.effects_areas = Some(render_effects_dialog(
                surface,
                w,
                h,
                self.effects_tab,
                &self.engine,
            ));
        } else {
            self.effects_areas = None;
        }

        // 8. Input / Shortcuts / Preferences Modal
        if self.active_modal != ActiveModal::None {
            #[cfg(feature = "std")]
            let ff_ready = spectra_core::codec::media_backend::find_ffmpeg_bin().is_some();
            #[cfg(not(feature = "std"))]
            let ff_ready = false;

            self.modal_areas = Some(render_input_modal(
                surface,
                w,
                h,
                self.active_modal,
                &self.modal_input,
                self.auto_hide_fullscreen,
                self.engine.max_decode_width,
                ff_ready,
            ));
        } else {
            self.modal_areas = None;
        }

        // 9. About Dialog
        if self.show_about {
            self.about_close_btn = Some(render_about_dialog(surface, w, h));
        } else {
            self.about_close_btn = None;
        }

        // 10. OSD Toast Pill (Top-Right below header, capped width so it stays clean)
        if let Some(ref osd) = self.osd_text {
            let short = if osd.chars().count() > 42 {
                let prefix: String = osd.chars().take(39).collect();
                format!("{prefix}...")
            } else {
                osd.clone()
            };
            let tw = measure_text(&short, 1) + 22;
            let tx = (w - tw - 18).max(12);
            let ty = if self.show_controls { 74 } else { 18 };
            let tr = Rect::new(tx, ty, tw, 24);
            fill_rounded_rect(surface, w, h, tr, 6, Color::rgba(15, 23, 42, 225));
            draw_rect_outline(surface, w, h, tr, 1, Color::rgb(56, 189, 248));
            draw_text(surface, w, h, tx + 11, ty + 7, &short, Color::WHITE, 1);
        }
    }

    pub fn handle_menu_action(&mut self, action: MenuAction) {
        match action {
            // Media
            MenuAction::MediaOpenFile => self.pending_commands.push(UiCommand::OpenFileDialog),
            MenuAction::MediaOpenMultiple => self.pending_commands.push(UiCommand::OpenMultipleDialog),
            MenuAction::MediaOpenDirectory => self.pending_commands.push(UiCommand::OpenDirectoryDialog),
            MenuAction::MediaOpenNetworkStream => {
                self.active_modal = ActiveModal::OpenUrl;
                self.modal_input.clear();
            }
            MenuAction::MediaOpenClipboard => self.pending_commands.push(UiCommand::PasteFromClipboard),
            MenuAction::MediaRecent(idx) => {
                if let Some(p) = self.engine.recent_paths.get(idx).cloned() {
                    #[cfg(feature = "std")]
                    self.engine.open_path(&p);
                    #[cfg(not(feature = "std"))]
                    {
                        let _ = p;
                        self.engine.select_track(idx);
                    }
                }
            }
            MenuAction::MediaSavePlaylist => self.pending_commands.push(UiCommand::SavePlaylistDialog),
            MenuAction::MediaInstallCodecs => {
                #[cfg(feature = "std")]
                self.engine.ensure_universal_codecs();
                self.show_osd("Checking Universal Codec Pack...");
            }
            MenuAction::MediaQuit => {
                self.engine.stop();
                self.pending_commands.push(UiCommand::QuitApp);
            }

            // Playback
            MenuAction::PlaybackPlayPause => {
                self.engine.toggle_play();
                self.show_osd(if self.engine.state == PlaybackState::Playing { "Playing" } else { "Paused" });
            }
            MenuAction::PlaybackStop => {
                self.engine.stop();
                self.show_osd("Stopped");
            }
            MenuAction::PlaybackPrevious => self.engine.previous_track(),
            MenuAction::PlaybackNext => self.engine.next_track(),
            MenuAction::PlaybackStepFrame => {
                self.engine.step_frame();
                self.show_osd("Step +1 Frame");
            }
            MenuAction::PlaybackSpeed(pct) => {
                self.engine.set_speed(pct as f32 / 100.0);
                self.show_osd(format!("Speed: {:.2}x", self.engine.speed));
            }
            MenuAction::PlaybackJumpForward(sec) => {
                self.engine.seek_relative((sec as i64) * 1000);
                self.show_osd(format!("Seek {}", format_time(self.engine.position_ms)));
            }
            MenuAction::PlaybackJumpBackward(sec) => {
                self.engine.seek_relative(-((sec as i64) * 1000));
                self.show_osd(format!("Seek {}", format_time(self.engine.position_ms)));
            }
            MenuAction::PlaybackJumpToTime => {
                self.active_modal = ActiveModal::JumpToTime;
                self.modal_input.clear();
            }
            MenuAction::PlaybackToggleLoop => {
                self.engine.playlist.loop_mode = self.engine.playlist.loop_mode.next_mode();
                self.is_looping = self.engine.playlist.loop_mode != LoopMode::None;
                self.show_osd(format!("Repeat: {:?}", self.engine.playlist.loop_mode));
            }
            MenuAction::PlaybackToggleShuffle => {
                let shuf = self.engine.playlist.toggle_shuffle(self.clock_ms + 13);
                self.is_shuffling = shuf;
                self.show_osd(if shuf { "Shuffle: On" } else { "Shuffle: Off" });
            }
            MenuAction::PlaybackABLoop => {
                let msg = self.engine.cycle_ab_loop();
                self.show_osd(msg);
            }

            // Audio
            MenuAction::AudioSelectTrack(idx) => {
                if idx == usize::MAX {
                    self.engine.select_audio_track(None);
                    self.show_osd("Audio Track: Disabled");
                } else {
                    self.engine.select_audio_track(Some(idx));
                    self.show_osd(format!("Audio Track {}", idx + 1));
                }
            }
            MenuAction::AudioVisualizer(mode) => {
                let m = match mode {
                    0 => AudioVisualizerMode::Off,
                    2 => AudioVisualizerMode::Waveform,
                    3 => AudioVisualizerMode::VinylTurntable,
                    _ => AudioVisualizerMode::Spectrum32,
                };
                self.engine.audio.visualizer_mode = m;
                self.show_visualizer = m != AudioVisualizerMode::Off;
                self.show_osd(format!("Visualizer: {}", m.name()));
            }
            MenuAction::AudioEqualizerPreset(preset_idx) => {
                let preset = match preset_idx {
                    1 => EqualizerPreset::Rock,
                    2 => EqualizerPreset::Techno,
                    3 => EqualizerPreset::BassBoost,
                    4 => EqualizerPreset::Vocal,
                    5 => EqualizerPreset::NovaAcoustic,
                    6 => EqualizerPreset::Pop,
                    7 => EqualizerPreset::Classical,
                    _ => EqualizerPreset::Flat,
                };
                self.engine.audio.set_equalizer_preset(preset);
                self.show_osd(format!("EQ: {}", preset.name()));
            }
            MenuAction::AudioVolUp => {
                self.engine.audio.set_volume(self.engine.audio.volume + 0.05);
                self.show_osd(format!("Volume: {:.0}%", self.engine.audio.volume * 100.0));
            }
            MenuAction::AudioVolDown => {
                self.engine.audio.set_volume(self.engine.audio.volume - 0.05);
                self.show_osd(format!("Volume: {:.0}%", self.engine.audio.volume * 100.0));
            }
            MenuAction::AudioMute => {
                self.engine.audio.toggle_mute();
                self.show_osd(if self.engine.audio.is_muted { "Muted" } else { "Unmuted" });
            }

            // Video
            MenuAction::VideoSelectTrack(idx) => {
                self.engine.active_video_track = if idx == usize::MAX { None } else { Some(idx) };
                self.show_osd(if idx == usize::MAX { "Video Disabled" } else { "Video Track 1" });
            }
            MenuAction::VideoFullscreen => {
                self.is_fullscreen = !self.is_fullscreen;
                self.pending_commands.push(UiCommand::ToggleOsFullscreen);
            }
            MenuAction::VideoAlwaysFit => {
                self.engine.aspect_ratio = AspectRatio::Auto;
                self.engine.video_synth.adjustments.zoom = 1.0;
                self.show_osd("Fit to Window");
            }
            MenuAction::VideoZoom(pct) => {
                self.engine.video_synth.adjustments.zoom = (pct as f32 / 100.0).clamp(0.25, 3.0);
                self.show_osd(format!("Zoom: {}%", pct));
            }
            MenuAction::VideoAspectRatio(ar_idx) => {
                self.engine.aspect_ratio = match ar_idx {
                    1 => AspectRatio::Ratio16_9,
                    2 => AspectRatio::Ratio4_3,
                    3 => AspectRatio::Ratio21_9,
                    4 => AspectRatio::Ratio1_1,
                    5 => AspectRatio::Fill,
                    _ => AspectRatio::Auto,
                };
                self.show_osd(format!("Aspect: {}", self.engine.aspect_ratio.name()));
            }
            MenuAction::VideoCrop(c_idx) => {
                self.engine.video_synth.adjustments.crop = match c_idx {
                    1 => CropMode::Ratio16_9,
                    2 => CropMode::Ratio4_3,
                    3 => CropMode::Ratio21_9,
                    4 => CropMode::Ratio1_1,
                    _ => CropMode::None,
                };
                self.show_osd(self.engine.video_synth.adjustments.crop.name());
            }
            MenuAction::VideoRotate90 => {
                let deg = (self.engine.video_synth.adjustments.rotate_deg + 90) % 360;
                self.engine.video_synth.adjustments.rotate_deg = deg;
                self.show_osd(format!("Rotation: {} deg", deg));
            }
            MenuAction::VideoFlipH => {
                self.engine.video_synth.adjustments.flip_h = !self.engine.video_synth.adjustments.flip_h;
                self.show_osd(if self.engine.video_synth.adjustments.flip_h { "Flipped H" } else { "Normal H" });
            }
            MenuAction::VideoDeinterlace => {
                self.engine.video_synth.adjustments.deinterlace = !self.engine.video_synth.adjustments.deinterlace;
                self.show_osd(if self.engine.video_synth.adjustments.deinterlace { "Deinterlace: On" } else { "Deinterlace: Off" });
            }
            MenuAction::VideoSnapshot => self.pending_commands.push(UiCommand::TakeSnapshot),
            MenuAction::VideoTheme(theme_idx) => {
                self.engine.video_synth.theme = match theme_idx {
                    1 => spectra_core::codec::video_synth::VideoTheme::DeepSpaceNebula,
                    2 => spectra_core::codec::video_synth::VideoTheme::OceanAurora,
                    _ => spectra_core::codec::video_synth::VideoTheme::CyberpunkMetropolis,
                };
            }

            // Subtitle
            MenuAction::SubtitleAddFile => self.pending_commands.push(UiCommand::OpenSubtitleDialog),
            MenuAction::SubtitleSelectTrack(idx) => {
                if idx == usize::MAX {
                    self.engine.select_subtitle_track(None);
                    self.show_osd("Subtitles: Off");
                } else {
                    self.engine.select_subtitle_track(Some(idx));
                    self.show_osd(format!("Subtitle: {}", self.engine.subtitles.title));
                }
            }
            MenuAction::SubtitleToggle => {
                self.engine.subtitles.is_enabled = !self.engine.subtitles.is_enabled;
                self.show_osd(if self.engine.subtitles.is_enabled { "Subtitles: On" } else { "Subtitles: Off" });
            }
            MenuAction::SubtitleDelayDelta(delta) => {
                self.engine.subtitles.delay_ms += delta;
                self.show_osd(format!("Subtitle Delay: {} ms", self.engine.subtitles.delay_ms));
            }

            // Tools
            MenuAction::ToolsEffectsEqualizer => {
                self.show_effects = !self.show_effects;
                self.effects_tab = EffectsTab::AudioEq;
            }
            MenuAction::ToolsMediaInformation => self.show_info = !self.show_info,
            MenuAction::ToolsTrackSync => {
                self.show_effects = true;
                self.effects_tab = EffectsTab::TrackSync;
            }
            MenuAction::ToolsPreferences => {
                self.active_modal = ActiveModal::Preferences;
            }

            // View
            MenuAction::ViewPlaylist => self.show_playlist = !self.show_playlist,
            MenuAction::ViewVisualizer => {
                self.show_visualizer = !self.show_visualizer;
                if self.show_visualizer && self.engine.audio.visualizer_mode == AudioVisualizerMode::Off {
                    self.engine.audio.visualizer_mode = AudioVisualizerMode::Spectrum32;
                }
            }
            MenuAction::ViewMinimalView => self.show_controls = !self.show_controls,

            // Help
            MenuAction::HelpAbout => self.show_about = !self.show_about,
            MenuAction::HelpShortcuts => {
                self.active_modal = ActiveModal::Shortcuts;
            }
        }
    }

    pub fn handle_click(&mut self, px: i32, py: i32, width: usize, height: usize) -> bool {
        let w = width as i32;
        let h = height as i32;
        self.idle_ms = 0;

        // 1. About Modal Dialog
        if self.show_about {
            if let Some(close_btn) = self.about_close_btn {
                if close_btn.contains(px, py) {
                    self.show_about = false;
                    return true;
                }
            }
            let dialog_w = 450.min(w - 40);
            let dialog_h = 300.min(h - 40);
            let dialog_rect = Rect::new((w - dialog_w) / 2, (h - dialog_h) / 2, dialog_w, dialog_h);
            if !dialog_rect.contains(px, py) {
                self.show_about = false;
            }
            return true;
        }

        // 1b. Input / Shortcuts / Preferences Modal
        if self.active_modal != ActiveModal::None {
            if let Some(ref ma) = self.modal_areas {
                if ma.close_btn.contains(px, py) || !ma.dialog_rect.contains(px, py) {
                    self.active_modal = ActiveModal::None;
                    return true;
                }
                if ma.submit_btn.contains(px, py) {
                    self.handle_modal_submit();
                    return true;
                }
                if ma.pref_codec_btn.contains(px, py) {
                    #[cfg(feature = "std")]
                    self.engine.ensure_universal_codecs();
                    return true;
                }
                if ma.pref_autohide_btn.contains(px, py) {
                    self.auto_hide_fullscreen = !self.auto_hide_fullscreen;
                    return true;
                }
                if ma.pref_res_cap_btn.contains(px, py) {
                    self.engine.max_decode_width = match self.engine.max_decode_width {
                        1280 => 1920,
                        1920 => 2560,
                        2560 => 3840,
                        _ => 1280,
                    };
                    return true;
                }
            }
            return true;
        }

        // 2. Dropdown Menu Clicks
        if let Some(active_m) = self.menubar_state.active_menu {
            let menu_layout = MenuBarLayout::new(w);
            let menu_x = menu_layout.menu_rects[active_m].x.min((w - 256).max(4));
            let menu_y = menu_layout.bar_rect.h;
            let item_h = 22;
            let menu_w = 250;

            let items = get_menu_items(active_m, &self.engine, self.show_visualizer);
            let menu_h = (items.len() as i32) * item_h + 8;
            let drop_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

            if let Some(sub_id) = self.menubar_state.active_submenu {
                let sub_items = get_submenu_items(sub_id, &self.engine);
                let sub_w = 240;
                let sub_x = if menu_x + menu_w + sub_w <= w {
                    menu_x + menu_w - 4
                } else {
                    (menu_x - sub_w + 4).max(4)
                };
                let sub_h = (sub_items.len() as i32) * item_h + 8;
                // Find parent y
                let mut sub_y = menu_y + 12;
                for (ii, it) in items.iter().enumerate() {
                    if it.submenu_id == Some(sub_id) {
                        sub_y = menu_y + 4 + (ii as i32) * item_h;
                        break;
                    }
                }
                sub_y = sub_y.min((h - sub_h - 8).max(menu_y));
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
                self.menubar_state.active_menu = None;
                self.menubar_state.active_submenu = None;
            }
        }

        // 3. Menubar Titles Click
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

        // 4. Effects & 10-Band EQ Dialog Clicks
        if self.show_effects {
            if let Some(ref ea) = self.effects_areas {
                if ea.close_btn.contains(px, py) {
                    self.show_effects = false;
                    return true;
                }
                if ea.tab_audio.contains(px, py) {
                    self.effects_tab = EffectsTab::AudioEq;
                    return true;
                }
                if ea.tab_video.contains(px, py) {
                    self.effects_tab = EffectsTab::VideoEffects;
                    return true;
                }
                if ea.tab_sync.contains(px, py) {
                    self.effects_tab = EffectsTab::TrackSync;
                    return true;
                }
                if ea.dialog_rect.contains(px, py) {
                    match self.effects_tab {
                        EffectsTab::AudioEq => {
                            if ea.eq_enable_btn.contains(px, py) {
                                self.engine.audio.eq_enabled = !self.engine.audio.eq_enabled;
                                return true;
                            }
                            if ea.eq_reset_btn.contains(px, py) {
                                self.engine.audio.reset_eq();
                                return true;
                            }
                            for &(preset, r) in &ea.eq_preset_btns {
                                if r.contains(px, py) {
                                    self.engine.audio.set_equalizer_preset(preset);
                                    return true;
                                }
                            }
                            for b in 0..EQ_BANDS {
                                if ea.eq_band_sliders[b].contains(px, py) {
                                    self.drag_target = DragTarget::EqBand(b);
                                    let r = ea.eq_band_sliders[b];
                                    let norm = 1.0 - ((py - r.y) as f32 / r.h as f32).clamp(0.0, 1.0);
                                    self.engine.audio.set_eq_band(b, norm * 24.0 - 12.0);
                                    return true;
                                }
                            }
                        }
                        EffectsTab::VideoEffects => {
                            if ea.vid_brightness_bar.contains(px, py) {
                                self.drag_target = DragTarget::VidBrightness;
                                let r = ea.vid_brightness_bar;
                                let frac = ((px - r.x) as f32 / r.w as f32).clamp(0.0, 1.0);
                                self.engine.video_synth.adjustments.brightness = (frac * 200.0 - 100.0) as i32;
                                return true;
                            }
                            if ea.vid_contrast_bar.contains(px, py) {
                                self.drag_target = DragTarget::VidContrast;
                                let r = ea.vid_contrast_bar;
                                let frac = ((px - r.x) as f32 / r.w as f32).clamp(0.0, 1.0);
                                self.engine.video_synth.adjustments.contrast = 0.5 + frac * 1.5;
                                return true;
                            }
                            if ea.vid_saturation_bar.contains(px, py) {
                                self.drag_target = DragTarget::VidSaturation;
                                let r = ea.vid_saturation_bar;
                                let frac = ((px - r.x) as f32 / r.w as f32).clamp(0.0, 1.0);
                                self.engine.video_synth.adjustments.saturation = frac * 2.0;
                                return true;
                            }
                            if ea.vid_gamma_bar.contains(px, py) {
                                self.drag_target = DragTarget::VidGamma;
                                let r = ea.vid_gamma_bar;
                                let frac = ((px - r.x) as f32 / r.w as f32).clamp(0.0, 1.0);
                                self.engine.video_synth.adjustments.gamma = 0.5 + frac * 2.0;
                                return true;
                            }
                            if ea.vid_grayscale_btn.contains(px, py) {
                                self.engine.video_synth.adjustments.grayscale = !self.engine.video_synth.adjustments.grayscale;
                                return true;
                            }
                            if ea.vid_invert_btn.contains(px, py) {
                                self.engine.video_synth.adjustments.invert = !self.engine.video_synth.adjustments.invert;
                                return true;
                            }
                            if ea.vid_deinterlace_btn.contains(px, py) {
                                self.engine.video_synth.adjustments.deinterlace = !self.engine.video_synth.adjustments.deinterlace;
                                return true;
                            }
                            if ea.vid_flip_btn.contains(px, py) {
                                self.engine.video_synth.adjustments.flip_h = !self.engine.video_synth.adjustments.flip_h;
                                return true;
                            }
                            if ea.vid_rotate_btn.contains(px, py) {
                                self.engine.video_synth.adjustments.rotate_deg =
                                    (self.engine.video_synth.adjustments.rotate_deg + 90) % 360;
                                return true;
                            }
                            if ea.vid_reset_btn.contains(px, py) {
                                self.engine.video_synth.adjustments.reset();
                                return true;
                            }
                        }
                        EffectsTab::TrackSync => {
                            if ea.aud_delay_minus.contains(px, py) {
                                self.engine.audio.audio_delay_ms = (self.engine.audio.audio_delay_ms - 50).max(-5000);
                                return true;
                            }
                            if ea.aud_delay_plus.contains(px, py) {
                                self.engine.audio.audio_delay_ms = (self.engine.audio.audio_delay_ms + 50).min(5000);
                                return true;
                            }
                            if ea.sub_delay_minus.contains(px, py) {
                                self.engine.subtitles.delay_ms = (self.engine.subtitles.delay_ms - 100).max(-10000);
                                return true;
                            }
                            if ea.sub_delay_plus.contains(px, py) {
                                self.engine.subtitles.delay_ms = (self.engine.subtitles.delay_ms + 100).min(10000);
                                return true;
                            }
                            for &(sc, r) in &ea.sub_scale_btns {
                                if r.contains(px, py) {
                                    self.engine.subtitles.font_scale = sc;
                                    return true;
                                }
                            }
                        }
                    }
                    return true;
                }
            }
        }

        // 5. Info Panel close button
        if self.show_info {
            if let Some(close_btn) = self.info_close_btn {
                if close_btn.contains(px, py) {
                    self.show_info = false;
                    return true;
                }
            }
        }

        // 6. Playlist drawer clicks
        if self.show_playlist {
            if let Some(ref pl) = self.playlist_areas {
                if pl.close_btn.contains(px, py) {
                    self.show_playlist = false;
                    return true;
                }
                if pl.add_file_btn.contains(px, py) {
                    self.pending_commands.push(UiCommand::OpenMultipleDialog);
                    return true;
                }
                if pl.add_dir_btn.contains(px, py) {
                    self.pending_commands.push(UiCommand::OpenDirectoryDialog);
                    return true;
                }
                if pl.save_btn.contains(px, py) {
                    self.pending_commands.push(UiCommand::SavePlaylistDialog);
                    return true;
                }
                if pl.clear_btn.contains(px, py) {
                    self.engine.playlist.clear();
                    self.playlist_scroll = 0;
                    return true;
                }
                for &(idx, rect) in &pl.remove_rects {
                    if rect.contains(px, py) {
                        self.engine.playlist.remove(idx);
                        return true;
                    }
                }
                for &(idx, rect) in &pl.item_rects {
                    if rect.contains(px, py) {
                        self.engine.select_track(idx);
                        return true;
                    }
                }
                if pl.drawer_rect.contains(px, py) {
                    return true;
                }
            }
        }

        // 7. Top bar action buttons
        if let Some(top) = self.top_areas {
            if top.open_btn.contains(px, py) {
                self.pending_commands.push(UiCommand::OpenFileDialog);
                return true;
            }
            if top.fullscreen_btn.contains(px, py) {
                self.is_fullscreen = !self.is_fullscreen;
                self.pending_commands.push(UiCommand::ToggleOsFullscreen);
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

        // 8. Bottom bar controls
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
                self.pending_commands.push(UiCommand::ToggleOsFullscreen);
                return true;
            }
            if btm.effects_eq_btn.contains(px, py) {
                self.show_effects = !self.show_effects;
                return true;
            }
            if btm.playlist_btn.contains(px, py) {
                self.show_playlist = !self.show_playlist;
                return true;
            }
            if btm.loop_btn.contains(px, py) {
                self.engine.playlist.loop_mode = self.engine.playlist.loop_mode.next_mode();
                self.is_looping = self.engine.playlist.loop_mode != LoopMode::None;
                return true;
            }
            if btm.shuffle_btn.contains(px, py) {
                self.is_shuffling = self.engine.playlist.toggle_shuffle(self.clock_ms + 13);
                return true;
            }
            if btm.volume_mute_btn.contains(px, py) {
                self.engine.audio.toggle_mute();
                return true;
            }
            if btm.volume_slider.contains(px, py) {
                self.drag_target = DragTarget::Volume;
                let rel = (px - btm.volume_slider.x) as f32 / btm.volume_slider.w as f32;
                self.engine.audio.set_volume(rel * 2.0);
                return true;
            }
            if btm.speed_btn.contains(px, py) {
                self.engine.cycle_speed();
                self.show_osd(format!("Speed: {:.2}x", self.engine.speed));
                return true;
            }
            if btm.aspect_btn.contains(px, py) {
                self.engine.cycle_aspect_ratio();
                self.show_osd(format!("Aspect: {}", self.engine.aspect_ratio.name()));
                return true;
            }
            if btm.cc_btn.contains(px, py) {
                self.engine.subtitles.is_enabled = !self.engine.subtitles.is_enabled;
                self.show_osd(if self.engine.subtitles.is_enabled { "Subtitles: On" } else { "Subtitles: Off" });
                return true;
            }
            if btm.snap_btn.contains(px, py) {
                self.pending_commands.push(UiCommand::TakeSnapshot);
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
                self.drag_target = DragTarget::Scrub;
                self.is_scrubbing = true;
                let rel = (px - btm.scrub_bar.x) as f32 / btm.scrub_bar.w as f32;
                let target = (rel.clamp(0.0, 1.0) * self.engine.metadata.duration_ms as f32) as u64;
                self.engine.seek(target);
                return true;
            }
            if btm.bar_rect.contains(px, py) {
                return true;
            }
        }

        // 9. Standby Welcome Stage Clicks (only when no media loaded)
        if !self.engine.metadata.is_live_file {
            if let Some(ref idle) = self.idle_areas {
                if idle.open_file_btn.contains(px, py) {
                    self.pending_commands.push(UiCommand::OpenFileDialog);
                    return true;
                }
                if idle.open_folder_btn.contains(px, py) {
                    self.pending_commands.push(UiCommand::OpenDirectoryDialog);
                    return true;
                }
                if idle.open_stream_btn.contains(px, py) {
                    self.active_modal = ActiveModal::OpenUrl;
                    self.modal_input.clear();
                    return true;
                }
                for &(idx, rect) in &idle.recent_rects {
                    if rect.contains(px, py) {
                        if let Some(path) = self.engine.recent_paths.get(idx).cloned() {
                            #[cfg(feature = "std")]
                            self.engine.open_path(&path);
                            #[cfg(not(feature = "std"))]
                            let _ = path;
                        }
                        return true;
                    }
                }
            }
        }

        // 10. Video Canvas Click: double-click toggles fullscreen, single-click toggles controls
        if self.clock_ms.saturating_sub(self.last_canvas_click_ms) < 340 && self.last_canvas_click_ms > 0 {
            self.is_fullscreen = !self.is_fullscreen;
            self.pending_commands.push(UiCommand::ToggleOsFullscreen);
            self.show_controls = true;
            self.last_canvas_click_ms = 0;
        } else {
            self.last_canvas_click_ms = self.clock_ms;
            self.show_controls = !self.show_controls;
        }
        true
    }

    pub fn handle_mouse_move(&mut self, px: i32, py: i32, is_down: bool, width: usize, _height: usize) {
        let w = width as i32;
        self.hover_pos = Some((px, py));
        self.idle_ms = 0;
        if self.is_fullscreen && !self.show_controls {
            self.show_controls = true;
        }

        if is_down {
            match self.drag_target {
                DragTarget::Scrub => {
                    if let Some(btm) = self.bottom_areas {
                        let rel = (px - btm.scrub_bar.x) as f32 / btm.scrub_bar.w as f32;
                        let target = (rel.clamp(0.0, 1.0) * self.engine.metadata.duration_ms as f32) as u64;
                        self.engine.seek(target);
                    }
                    return;
                }
                DragTarget::Volume => {
                    if let Some(btm) = self.bottom_areas {
                        let rel = (px - btm.volume_slider.x) as f32 / btm.volume_slider.w as f32;
                        self.engine.audio.set_volume(rel * 2.0);
                    }
                    return;
                }
                DragTarget::EqBand(b) => {
                    if let Some(ref ea) = self.effects_areas {
                        let r = ea.eq_band_sliders[b];
                        let norm = 1.0 - ((py - r.y) as f32 / r.h as f32).clamp(0.0, 1.0);
                        self.engine.audio.set_eq_band(b, norm * 24.0 - 12.0);
                    }
                    return;
                }
                DragTarget::VidBrightness => {
                    if let Some(ref ea) = self.effects_areas {
                        let r = ea.vid_brightness_bar;
                        let frac = ((px - r.x) as f32 / r.w as f32).clamp(0.0, 1.0);
                        self.engine.video_synth.adjustments.brightness = (frac * 200.0 - 100.0) as i32;
                    }
                    return;
                }
                DragTarget::VidContrast => {
                    if let Some(ref ea) = self.effects_areas {
                        let r = ea.vid_contrast_bar;
                        let frac = ((px - r.x) as f32 / r.w as f32).clamp(0.0, 1.0);
                        self.engine.video_synth.adjustments.contrast = 0.5 + frac * 1.5;
                    }
                    return;
                }
                DragTarget::VidSaturation => {
                    if let Some(ref ea) = self.effects_areas {
                        let r = ea.vid_saturation_bar;
                        let frac = ((px - r.x) as f32 / r.w as f32).clamp(0.0, 1.0);
                        self.engine.video_synth.adjustments.saturation = frac * 2.0;
                    }
                    return;
                }
                DragTarget::VidGamma => {
                    if let Some(ref ea) = self.effects_areas {
                        let r = ea.vid_gamma_bar;
                        let frac = ((px - r.x) as f32 / r.w as f32).clamp(0.0, 1.0);
                        self.engine.video_synth.adjustments.gamma = 0.5 + frac * 2.0;
                    }
                    return;
                }
                DragTarget::None => {
                    if let Some(btm) = self.bottom_areas {
                        if btm.scrub_bar.contains(px, py) {
                            self.drag_target = DragTarget::Scrub;
                            self.is_scrubbing = true;
                        } else if btm.volume_slider.contains(px, py) {
                            self.drag_target = DragTarget::Volume;
                        }
                    }
                    return;
                }
            }
        }

        if self.show_controls {
            let menu_layout = MenuBarLayout::new(w);

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

            if let Some(active_m) = self.menubar_state.active_menu {
                let menu_x = menu_layout.menu_rects[active_m].x.min((w - 256).max(4));
                let menu_y = menu_layout.bar_rect.h;
                let item_h = 22;
                let menu_w = 250;

                let items = get_menu_items(active_m, &self.engine, self.show_visualizer);
                let menu_h = (items.len() as i32) * item_h + 8;
                let drop_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

                if drop_rect.contains(px, py) {
                    let rel_y = py - (menu_y + 4);
                    let item_idx = (rel_y / item_h) as usize;
                    if item_idx < items.len() {
                        self.menubar_state.hovered_item = Some(item_idx);
                        if let Some(sub_id) = items[item_idx].submenu_id {
                            self.menubar_state.active_submenu = Some(sub_id);
                        } else {
                            self.menubar_state.active_submenu = None;
                        }
                    }
                } else {
                    self.menubar_state.hovered_item = None;
                }

                if let Some(sub_id) = self.menubar_state.active_submenu {
                    let sub_items = get_submenu_items(sub_id, &self.engine);
                    let sub_w = 240;
                    let sub_x = if menu_x + menu_w + sub_w <= w {
                        menu_x + menu_w - 4
                    } else {
                        (menu_x - sub_w + 4).max(4)
                    };
                    let sub_h = (sub_items.len() as i32) * item_h + 8;
                    let mut sub_y = menu_y + 12;
                    for (ii, it) in items.iter().enumerate() {
                        if it.submenu_id == Some(sub_id) {
                            sub_y = menu_y + 4 + (ii as i32) * item_h;
                            break;
                        }
                    }
                    let sub_rect = Rect::new(sub_x, sub_y, sub_w, sub_h);
                    if sub_rect.contains(px, py) {
                        let rel_y = py - (sub_y + 4);
                        self.menubar_state.hovered_sub_item = Some((rel_y / item_h) as usize);
                    } else {
                        self.menubar_state.hovered_sub_item = None;
                    }
                }
            }
        }
    }

    pub fn handle_mouse_up(&mut self) {
        self.is_scrubbing = false;
        self.drag_target = DragTarget::None;
    }

    pub fn handle_text_input(&mut self, text: &str) {
        if matches!(self.active_modal, ActiveModal::OpenUrl | ActiveModal::JumpToTime) {
            for ch in text.chars() {
                if !ch.is_control() {
                    self.modal_input.push(ch);
                }
            }
        }
    }

    pub fn handle_modal_backspace(&mut self) {
        if matches!(self.active_modal, ActiveModal::OpenUrl | ActiveModal::JumpToTime) {
            self.modal_input.pop();
        }
    }

    pub fn handle_modal_submit(&mut self) {
        match self.active_modal {
            ActiveModal::OpenUrl => {
                let target = self.modal_input.trim().to_string();
                self.active_modal = ActiveModal::None;
                if !target.is_empty() {
                    #[cfg(feature = "std")]
                    self.engine.open_path(&target);
                }
            }
            ActiveModal::JumpToTime => {
                let txt = self.modal_input.trim().to_string();
                self.active_modal = ActiveModal::None;
                if let Some(ms) = parse_jump_time(&txt) {
                    self.engine.seek(ms);
                    self.show_osd(format!("Jumped to {}", format_time(ms)));
                }
            }
            _ => {
                self.active_modal = ActiveModal::None;
            }
        }
    }

    pub fn handle_scroll(&mut self, delta_y: f32) {
        if self.show_playlist {
            if let Some(ref pl) = self.playlist_areas {
                if let Some((hx, hy)) = self.hover_pos {
                    if pl.drawer_rect.contains(hx, hy) {
                        if delta_y < 0.0 && self.playlist_scroll + 1 < self.engine.playlist.items.len() {
                            self.playlist_scroll += 1;
                        } else if delta_y > 0.0 {
                            self.playlist_scroll = self.playlist_scroll.saturating_sub(1);
                        }
                        return;
                    }
                }
            }
        }
        if delta_y > 0.0 {
            self.engine.audio.set_volume(self.engine.audio.volume + 0.05);
            self.show_osd(format!("Volume: {:.0}%", self.engine.audio.volume * 100.0));
        } else if delta_y < 0.0 {
            self.engine.audio.set_volume(self.engine.audio.volume - 0.05);
            self.show_osd(format!("Volume: {:.0}%", self.engine.audio.volume * 100.0));
        }
    }

    pub fn handle_key(&mut self, key_sym: u32) {
        if matches!(self.active_modal, ActiveModal::OpenUrl | ActiveModal::JumpToTime) {
            return;
        }
        match key_sym {
            32 => self.handle_menu_action(MenuAction::PlaybackPlayPause),
            115 | 83 => self.handle_menu_action(MenuAction::PlaybackStop),
            112 | 80 => self.engine.previous_track(),
            110 | 78 => self.engine.next_track(),
            101 | 69 => self.handle_menu_action(MenuAction::PlaybackStepFrame),
            98 | 66 => self.handle_menu_action(MenuAction::PlaybackABLoop),
            100 | 68 => self.handle_menu_action(MenuAction::VideoDeinterlace),
            103 | 71 => self.handle_menu_action(MenuAction::SubtitleDelayDelta(-50)),
            104 | 72 => self.handle_menu_action(MenuAction::SubtitleDelayDelta(50)),
            0xFF51 => self.handle_menu_action(MenuAction::PlaybackJumpBackward(10)),
            0xFF53 => self.handle_menu_action(MenuAction::PlaybackJumpForward(10)),
            0xFF52 => self.handle_menu_action(MenuAction::AudioVolUp),
            0xFF54 => self.handle_menu_action(MenuAction::AudioVolDown),
            102 | 70 => self.handle_menu_action(MenuAction::VideoFullscreen),
            109 | 77 => self.handle_menu_action(MenuAction::AudioMute),
            118 | 86 => self.handle_menu_action(MenuAction::ViewVisualizer),
            105 | 73 => self.handle_menu_action(MenuAction::ToolsMediaInformation),
            108 | 76 => self.handle_menu_action(MenuAction::ViewPlaylist),
            97 | 65 => {
                self.engine.cycle_aspect_ratio();
                self.show_osd(format!("Aspect: {}", self.engine.aspect_ratio.name()));
            }
            114 | 82 => self.handle_menu_action(MenuAction::PlaybackToggleLoop),
            122 | 90 => self.handle_menu_action(MenuAction::PlaybackToggleShuffle),
            99 | 67 => self.handle_menu_action(MenuAction::SubtitleToggle),
            91 => {
                self.engine.set_speed(self.engine.speed - 0.25);
                self.show_osd(format!("Speed: {:.2}x", self.engine.speed));
            }
            93 => {
                self.engine.set_speed(self.engine.speed + 0.25);
                self.show_osd(format!("Speed: {:.2}x", self.engine.speed));
            }
            63 => self.active_modal = ActiveModal::Shortcuts,
            0xFFBE => self.show_about = !self.show_about,
            _ => {}
        }
    }
}

fn render_subtitle_overlay(
    surface: &mut [u32],
    w: i32,
    h: i32,
    sub_text: &str,
    font_scale: f32,
    controls_visible: bool,
) {
    let scale = if font_scale >= 1.5 { 2 } else { 1 };
    let max_chars = ((w - 80).max(200) / (6 * scale)) as usize;
    let mut lines: Vec<String> = Vec::new();

    for raw_line in sub_text.lines() {
        let mut cur = String::new();
        for word in raw_line.split_whitespace() {
            if cur.is_empty() {
                cur.push_str(word);
            } else if cur.chars().count() + 1 + word.chars().count() <= max_chars {
                cur.push(' ');
                cur.push_str(word);
            } else {
                lines.push(cur);
                cur = word.to_string();
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
    }
    if lines.is_empty() {
        return;
    }

    let line_h = 14 * scale;
    let box_h = (lines.len() as i32) * line_h + 12;
    let max_line_w = lines.iter().map(|l| measure_text(l, scale)).max().unwrap_or(60);
    let box_w = max_line_w + 26;
    let box_x = (w - box_w) / 2;
    let base_y = if controls_visible { h - 94 } else { h - 28 };
    let box_y = (base_y - box_h).max(16);

    fill_rounded_rect(surface, w, h, Rect::new(box_x, box_y, box_w, box_h), 5, Color::rgba(0, 0, 0, 195));
    for (i, line) in lines.iter().enumerate() {
        let lw = measure_text(line, scale);
        let lx = box_x + (box_w - lw) / 2;
        let ly = box_y + 7 + (i as i32) * line_h;
        draw_text(surface, w, h, lx + 1, ly + 1, line, Color::BLACK, scale);
        draw_text(surface, w, h, lx, ly, line, Color::WHITE, scale);
    }
}

fn parse_jump_time(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.trim().split(':').collect();
    match parts.len() {
        1 => {
            let secs: f64 = parts[0].parse().ok()?;
            Some((secs.max(0.0) * 1000.0) as u64)
        }
        2 => {
            let m: u64 = parts[0].parse().ok()?;
            let sec: f64 = parts[1].parse().ok()?;
            Some(m * 60_000 + (sec.max(0.0) * 1000.0) as u64)
        }
        3 => {
            let h: u64 = parts[0].parse().ok()?;
            let m: u64 = parts[1].parse().ok()?;
            let sec: f64 = parts[2].parse().ok()?;
            Some(h * 3600_000 + m * 60_000 + (sec.max(0.0) * 1000.0) as u64)
        }
        _ => None,
    }
}

fn render_standby_stage(
    surface: &mut [u32],
    w: i32,
    h: i32,
    recent_paths: &[String],
) -> IdleStageAreas {
    // Sleek dark studio backdrop (no fake video, pure VLC-style ready canvas)
    fill_rect(surface, w, h, Rect::new(0, 0, w, h), Color::rgb(9, 13, 22));

    let cx = w / 2;
    let cy = (h / 2) - 26;

    // Subtle concentric studio rings around emblem
    draw_circle(surface, w, h, cx, cy - 36, 46, Color::rgba(56, 189, 248, 35));
    draw_circle(surface, w, h, cx, cy - 36, 36, Color::rgba(56, 189, 248, 90));
    fill_rounded_rect(
        surface,
        w,
        h,
        Rect::new(cx - 24, cy - 60, 48, 48),
        14,
        Color::rgba(14, 165, 233, 38),
    );
    draw_rect_outline(
        surface,
        w,
        h,
        Rect::new(cx - 24, cy - 60, 48, 48),
        1,
        Color::rgb(56, 189, 248),
    );
    draw_text(
        surface,
        w,
        h,
        cx - 6,
        cy - 44,
        ">",
        Color::rgb(56, 189, 248),
        2,
    );

    let title = "SPECTRA MEDIA PLAYER";
    let tw = measure_text(title, 2);
    draw_text(surface, w, h, cx - tw / 2, cy + 26, title, Color::WHITE, 2);

    let hint = "Drop any video, audio, subtitle, or playlist file here";
    let hw = measure_text(hint, 1);
    draw_text(
        surface,
        w,
        h,
        cx - hw / 2,
        cy + 52,
        hint,
        Color::rgb(148, 163, 184),
        1,
    );

    // Quick-action buttons: [ Open File ] [ Open Folder ] [ Network Stream ]
    let btn_y = cy + 78;
    let btn_w = 134;
    let btn_h = 30;
    let gap = 12;
    let total_w = btn_w * 3 + gap * 2;
    let start_x = cx - total_w / 2;

    let open_file_btn = Rect::new(start_x, btn_y, btn_w, btn_h);
    let open_folder_btn = Rect::new(start_x + btn_w + gap, btn_y, btn_w, btn_h);
    let open_stream_btn = Rect::new(start_x + (btn_w + gap) * 2, btn_y, btn_w, btn_h);

    fill_rounded_rect(
        surface,
        w,
        h,
        open_file_btn,
        6,
        Color::rgba(56, 189, 248, 45),
    );
    draw_rect_outline(surface, w, h, open_file_btn, 1, Color::rgb(56, 189, 248));
    let l1 = "Open File (Ctrl+O)";
    draw_text(
        surface,
        w,
        h,
        open_file_btn.x + (btn_w - measure_text(l1, 1)) / 2,
        open_file_btn.y + 11,
        l1,
        Color::WHITE,
        1,
    );

    fill_rounded_rect(
        surface,
        w,
        h,
        open_folder_btn,
        6,
        Color::rgba(255, 255, 255, 16),
    );
    draw_rect_outline(
        surface,
        w,
        h,
        open_folder_btn,
        1,
        Color::rgba(255, 255, 255, 42),
    );
    let l2 = "Open Folder";
    draw_text(
        surface,
        w,
        h,
        open_folder_btn.x + (btn_w - measure_text(l2, 1)) / 2,
        open_folder_btn.y + 11,
        l2,
        Color::rgb(226, 232, 240),
        1,
    );

    fill_rounded_rect(
        surface,
        w,
        h,
        open_stream_btn,
        6,
        Color::rgba(255, 255, 255, 16),
    );
    draw_rect_outline(
        surface,
        w,
        h,
        open_stream_btn,
        1,
        Color::rgba(255, 255, 255, 42),
    );
    let l3 = "Network Stream";
    draw_text(
        surface,
        w,
        h,
        open_stream_btn.x + (btn_w - measure_text(l3, 1)) / 2,
        open_stream_btn.y + 11,
        l3,
        Color::rgb(226, 232, 240),
        1,
    );

    let mut recent_rects = Vec::new();
    if !recent_paths.is_empty() && h > 420 {
        let rec_y = btn_y + 46;
        let rec_hdr = "Recent Media:";
        draw_text(
            surface,
            w,
            h,
            cx - measure_text(rec_hdr, 1) / 2,
            rec_y,
            rec_hdr,
            Color::rgb(100, 116, 139),
            1,
        );
        let row_w = 340.min(w - 60);
        let row_x = cx - row_w / 2;
        for (idx, p) in recent_paths.iter().take(3).enumerate() {
            let ry = rec_y + 16 + (idx as i32) * 24;
            if ry + 22 > h - 85 {
                break;
            }
            let rr = Rect::new(row_x, ry, row_w, 20);
            fill_rounded_rect(surface, w, h, rr, 4, Color::rgba(255, 255, 255, 10));
            let name = p.rsplit(['/', '\\']).next().unwrap_or(p);
            let max_c = ((row_w - 20).max(80) / 7) as usize;
            let disp = if name.chars().count() > max_c {
                let cut: String = name.chars().take(max_c.saturating_sub(3)).collect();
                format!("{cut}...")
            } else {
                name.to_string()
            };
            draw_text(
                surface,
                w,
                h,
                row_x + 10,
                ry + 6,
                &disp,
                Color::rgb(148, 163, 184),
                1,
            );
            recent_rects.push((idx, rr));
        }
    }

    IdleStageAreas {
        open_file_btn,
        open_folder_btn,
        open_stream_btn,
        recent_rects,
    }
}
