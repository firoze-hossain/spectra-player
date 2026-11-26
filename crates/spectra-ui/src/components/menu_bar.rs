//! Top Menu Bar and Dynamic Cascading Dropdown Menus for Spectra Media Studio.

use spectra_core::engine::MediaPlayerEngine;
use spectra_core::types::{AspectRatio, AudioVisualizerMode, CropMode, EqualizerPreset};
use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    // Media Menu
    MediaOpenFile,
    MediaOpenMultiple,
    MediaOpenDirectory,
    MediaOpenNetworkStream,
    MediaOpenClipboard,
    MediaRecent(usize),
    MediaSavePlaylist,
    MediaInstallCodecs,
    MediaQuit,

    // Playback Menu
    PlaybackPlayPause,
    PlaybackStop,
    PlaybackPrevious,
    PlaybackNext,
    PlaybackStepFrame,
    PlaybackSpeed(u32), // 25, 50, 75, 100, 125, 150, 200
    PlaybackJumpForward(i32),
    PlaybackJumpBackward(i32),
    PlaybackJumpToTime,
    PlaybackToggleLoop,
    PlaybackToggleShuffle,
    PlaybackABLoop,

    // Audio Menu
    AudioSelectTrack(usize), // usize::MAX = disable
    AudioVisualizer(usize),  // 0: Off, 1: 32-Band FFT, 2: Waveform, 3: Vinyl
    AudioEqualizerPreset(usize),
    AudioVolUp,
    AudioVolDown,
    AudioMute,

    // Video Menu
    VideoSelectTrack(usize), // usize::MAX = disable
    VideoFullscreen,
    VideoAlwaysFit,
    VideoZoom(u32), // 50, 100, 150, 200
    VideoAspectRatio(u8),
    VideoCrop(u8),
    VideoRotate90,
    VideoFlipH,
    VideoDeinterlace,
    VideoSnapshot,
    VideoTheme(usize),

    // Subtitle Menu
    SubtitleAddFile,
    SubtitleSelectTrack(usize), // usize::MAX = off
    SubtitleToggle,
    SubtitleDelayDelta(i64),

    // Tools Menu
    ToolsEffectsEqualizer,
    ToolsMediaInformation,
    ToolsTrackSync,
    ToolsPreferences,

    // View Menu
    ViewPlaylist,
    ViewVisualizer,
    ViewMinimalView,

    // Help Menu
    HelpAbout,
    HelpShortcuts,
}

#[derive(Debug, Clone)]
pub struct MenuItemDef {
    pub label: String,
    pub shortcut: &'static str,
    pub action: Option<MenuAction>,
    pub is_separator: bool,
    pub is_checked: bool,
    pub submenu_id: Option<u8>,
}

impl MenuItemDef {
    pub fn item(label: impl Into<String>, shortcut: &'static str, action: MenuAction) -> Self {
        Self {
            label: label.into(),
            shortcut,
            action: Some(action),
            is_separator: false,
            is_checked: false,
            submenu_id: None,
        }
    }

    pub fn checked(
        label: impl Into<String>,
        shortcut: &'static str,
        action: MenuAction,
        checked: bool,
    ) -> Self {
        Self {
            label: label.into(),
            shortcut,
            action: Some(action),
            is_separator: false,
            is_checked: checked,
            submenu_id: None,
        }
    }

    pub fn submenu(label: impl Into<String>, submenu_id: u8) -> Self {
        Self {
            label: label.into(),
            shortcut: "",
            action: None,
            is_separator: false,
            is_checked: false,
            submenu_id: Some(submenu_id),
        }
    }

    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: "",
            action: None,
            is_separator: true,
            is_checked: false,
            submenu_id: None,
        }
    }
}

#[derive(Default)]
pub struct MenuBarState {
    pub active_menu: Option<usize>,
    pub hovered_item: Option<usize>,
    pub active_submenu: Option<u8>,
    pub hovered_sub_item: Option<usize>,
}

pub const MENU_TITLES: [&str; 8] = [
    "Media",
    "Playback",
    "Audio",
    "Video",
    "Subtitle",
    "Tools",
    "View",
    "Help",
];

pub fn get_menu_items(menu_idx: usize, engine: &MediaPlayerEngine, show_viz: bool) -> Vec<MenuItemDef> {
    let is_playing = engine.state == spectra_core::types::PlaybackState::Playing;
    match menu_idx {
        0 => vec![
            MenuItemDef::item("Open File...", "Ctrl+O", MenuAction::MediaOpenFile),
            MenuItemDef::item("Open Multiple Files...", "Ctrl+Shift+O", MenuAction::MediaOpenMultiple),
            MenuItemDef::item("Open Directory...", "Ctrl+F", MenuAction::MediaOpenDirectory),
            MenuItemDef::item("Open Network Stream...", "Ctrl+N", MenuAction::MediaOpenNetworkStream),
            MenuItemDef::item("Open Location from Clipboard", "Ctrl+V", MenuAction::MediaOpenClipboard),
            MenuItemDef::submenu("Open Recent Media", 1),
            MenuItemDef::separator(),
            MenuItemDef::item("Save Playlist to M3U8...", "Ctrl+Y", MenuAction::MediaSavePlaylist),
            MenuItemDef::item("Install / Verify Codecs", "", MenuAction::MediaInstallCodecs),
            MenuItemDef::separator(),
            MenuItemDef::item("Quit", "Ctrl+Q", MenuAction::MediaQuit),
        ],
        1 => vec![
            MenuItemDef::submenu("Speed", 2),
            MenuItemDef::item("Step Forward 1 Frame", "E", MenuAction::PlaybackStepFrame),
            MenuItemDef::item("Jump Forward (+10s)", "Right", MenuAction::PlaybackJumpForward(10)),
            MenuItemDef::item("Jump Backward (-10s)", "Left", MenuAction::PlaybackJumpBackward(10)),
            MenuItemDef::item("Jump to Specific Time...", "Ctrl+T", MenuAction::PlaybackJumpToTime),
            MenuItemDef::separator(),
            MenuItemDef::item(if is_playing { "Pause" } else { "Play" }, "Space", MenuAction::PlaybackPlayPause),
            MenuItemDef::item("Stop", "S", MenuAction::PlaybackStop),
            MenuItemDef::item("Previous Track", "P", MenuAction::PlaybackPrevious),
            MenuItemDef::item("Next Track", "N", MenuAction::PlaybackNext),
            MenuItemDef::checked("Cycle Loop Mode", "R", MenuAction::PlaybackToggleLoop, engine.playlist.loop_mode != spectra_core::playlist::LoopMode::None),
            MenuItemDef::checked("Shuffle Playlist", "Z", MenuAction::PlaybackToggleShuffle, engine.playlist.is_shuffled),
            MenuItemDef::checked("A-B Loop Point", "B", MenuAction::PlaybackABLoop, engine.ab_loop.is_some()),
        ],
        2 => vec![
            MenuItemDef::submenu("Audio Track", 3),
            MenuItemDef::submenu("Audio Visualizer", 4),
            MenuItemDef::submenu("Equalizer Preset", 5),
            MenuItemDef::separator(),
            MenuItemDef::item("Increase Volume (+5%)", "Up", MenuAction::AudioVolUp),
            MenuItemDef::item("Decrease Volume (-5%)", "Down", MenuAction::AudioVolDown),
            MenuItemDef::checked("Mute Audio", "M", MenuAction::AudioMute, engine.audio.is_muted),
        ],
        3 => vec![
            MenuItemDef::submenu("Video Track", 6),
            MenuItemDef::item("Fullscreen", "F", MenuAction::VideoFullscreen),
            MenuItemDef::item("Always Fit Window", "", MenuAction::VideoAlwaysFit),
            MenuItemDef::separator(),
            MenuItemDef::submenu("Zoom", 7),
            MenuItemDef::submenu("Aspect Ratio", 8),
            MenuItemDef::submenu("Crop", 11),
            MenuItemDef::item("Rotate 90 Degrees", "Ctrl+R", MenuAction::VideoRotate90),
            MenuItemDef::checked("Flip Horizontal", "", MenuAction::VideoFlipH, engine.video_synth.adjustments.flip_h),
            MenuItemDef::checked("Deinterlace (Bob)", "D", MenuAction::VideoDeinterlace, engine.video_synth.adjustments.deinterlace),
            MenuItemDef::separator(),
            MenuItemDef::item("Take Snapshot (PNG)", "Shift+S", MenuAction::VideoSnapshot),
        ],
        4 => vec![
            MenuItemDef::item("Add Subtitle File...", "Ctrl+Shift+S", MenuAction::SubtitleAddFile),
            MenuItemDef::submenu("Subtitle Track", 10),
            MenuItemDef::checked("Enable Subtitles", "C", MenuAction::SubtitleToggle, engine.subtitles.is_enabled),
            MenuItemDef::separator(),
            MenuItemDef::item("Subtitle Delay +50ms", "H", MenuAction::SubtitleDelayDelta(50)),
            MenuItemDef::item("Subtitle Delay -50ms", "G", MenuAction::SubtitleDelayDelta(-50)),
        ],
        5 => vec![
            MenuItemDef::item("Effects, Filters & 10-Band EQ", "Ctrl+E", MenuAction::ToolsEffectsEqualizer),
            MenuItemDef::item("Media Information & Telemetry", "Ctrl+I", MenuAction::ToolsMediaInformation),
            MenuItemDef::item("Track Synchronization...", "", MenuAction::ToolsTrackSync),
            MenuItemDef::item("Preferences", "Ctrl+P", MenuAction::ToolsPreferences),
        ],
        6 => vec![
            MenuItemDef::item("Playlist Drawer", "L", MenuAction::ViewPlaylist),
            MenuItemDef::checked("Visualizer Overlay", "V", MenuAction::ViewVisualizer, show_viz),
            MenuItemDef::item("Minimal Interface Mode", "Ctrl+H", MenuAction::ViewMinimalView),
        ],
        7 => vec![
            MenuItemDef::item("About Spectra...", "F1", MenuAction::HelpAbout),
            MenuItemDef::item("Keyboard Shortcuts", "?", MenuAction::HelpShortcuts),
        ],
        _ => Vec::new(),
    }
}

pub fn get_submenu_items(submenu_id: u8, engine: &MediaPlayerEngine) -> Vec<MenuItemDef> {
    let current_aspect = engine.aspect_ratio;
    let current_speed = engine.speed;
    match submenu_id {
        1 => {
            if engine.recent_paths.is_empty() {
                vec![MenuItemDef::item("(No Recent Files)", "", MenuAction::MediaOpenFile)]
            } else {
                engine
                    .recent_paths
                    .iter()
                    .enumerate()
                    .take(8)
                    .map(|(i, p)| {
                        let name = p.rsplit(['/', '\\']).next().unwrap_or(p);
                        let short = if name.chars().count() > 26 {
                            let cut: String = name.chars().take(23).collect();
                            format!("{}. {}...", i + 1, cut)
                        } else {
                            format!("{}. {}", i + 1, name)
                        };
                        MenuItemDef::item(short, "", MenuAction::MediaRecent(i))
                    })
                    .collect()
            }
        }
        2 => vec![
            MenuItemDef::checked("0.25x Very Slow", "", MenuAction::PlaybackSpeed(25), (current_speed - 0.25).abs() < 0.05),
            MenuItemDef::checked("0.50x Slower", "", MenuAction::PlaybackSpeed(50), (current_speed - 0.5).abs() < 0.05),
            MenuItemDef::checked("0.75x Slower", "", MenuAction::PlaybackSpeed(75), (current_speed - 0.75).abs() < 0.05),
            MenuItemDef::checked("1.00x Normal Speed", "", MenuAction::PlaybackSpeed(100), (current_speed - 1.0).abs() < 0.05),
            MenuItemDef::checked("1.25x Faster", "", MenuAction::PlaybackSpeed(125), (current_speed - 1.25).abs() < 0.05),
            MenuItemDef::checked("1.50x Faster", "", MenuAction::PlaybackSpeed(150), (current_speed - 1.5).abs() < 0.05),
            MenuItemDef::checked("2.00x Double Speed", "", MenuAction::PlaybackSpeed(200), (current_speed - 2.0).abs() < 0.05),
        ],
        3 => {
            let mut items = Vec::new();
            if engine.metadata.audio_tracks.is_empty() {
                items.push(MenuItemDef::checked(
                    "Track 1 - Primary Audio",
                    "",
                    MenuAction::AudioSelectTrack(0),
                    engine.active_audio_track == Some(0),
                ));
            } else {
                for tr in &engine.metadata.audio_tracks {
                    let lbl = if tr.title.is_empty() {
                        format!("Track {} [{} {}]", tr.index + 1, tr.language, tr.codec)
                    } else {
                        format!("Track {} [{}] {}", tr.index + 1, tr.language, tr.title)
                    };
                    items.push(MenuItemDef::checked(
                        lbl,
                        "",
                        MenuAction::AudioSelectTrack(tr.index),
                        engine.active_audio_track == Some(tr.index),
                    ));
                }
            }
            items.push(MenuItemDef::checked(
                "Disable Audio",
                "",
                MenuAction::AudioSelectTrack(usize::MAX),
                engine.active_audio_track.is_none() || engine.audio.is_muted,
            ));
            items
        }
        4 => {
            let m = engine.audio.visualizer_mode;
            vec![
                MenuItemDef::checked("Off", "", MenuAction::AudioVisualizer(0), m == AudioVisualizerMode::Off),
                MenuItemDef::checked("32-Band FFT Spectrum", "", MenuAction::AudioVisualizer(1), m == AudioVisualizerMode::Spectrum32),
                MenuItemDef::checked("Oscilloscope Waveform", "", MenuAction::AudioVisualizer(2), m == AudioVisualizerMode::Waveform),
                MenuItemDef::checked("Vinyl Turntable", "", MenuAction::AudioVisualizer(3), m == AudioVisualizerMode::VinylTurntable),
            ]
        }
        5 => {
            let eq = engine.audio.equalizer;
            vec![
                MenuItemDef::checked("Flat", "", MenuAction::AudioEqualizerPreset(0), eq == EqualizerPreset::Flat),
                MenuItemDef::checked("Rock", "", MenuAction::AudioEqualizerPreset(1), eq == EqualizerPreset::Rock),
                MenuItemDef::checked("Pop", "", MenuAction::AudioEqualizerPreset(6), eq == EqualizerPreset::Pop),
                MenuItemDef::checked("Classical", "", MenuAction::AudioEqualizerPreset(7), eq == EqualizerPreset::Classical),
                MenuItemDef::checked("Techno / EDM", "", MenuAction::AudioEqualizerPreset(2), eq == EqualizerPreset::Techno),
                MenuItemDef::checked("Bass Boost (+6dB)", "", MenuAction::AudioEqualizerPreset(3), eq == EqualizerPreset::BassBoost),
                MenuItemDef::checked("Vocal Clarity", "", MenuAction::AudioEqualizerPreset(4), eq == EqualizerPreset::Vocal),
                MenuItemDef::checked("Nova Acoustic Studio", "", MenuAction::AudioEqualizerPreset(5), eq == EqualizerPreset::NovaAcoustic),
            ]
        }
        6 => vec![
            MenuItemDef::checked("Track 1 - Video Stream", "", MenuAction::VideoSelectTrack(0), engine.active_video_track == Some(0)),
            MenuItemDef::checked("Disable Video", "", MenuAction::VideoSelectTrack(usize::MAX), engine.active_video_track.is_none()),
        ],
        7 => {
            let z = engine.video_synth.adjustments.zoom;
            vec![
                MenuItemDef::checked("1:2 Half (50%)", "", MenuAction::VideoZoom(50), (z - 0.5).abs() < 0.05),
                MenuItemDef::checked("1:1 Fit (100%)", "", MenuAction::VideoZoom(100), (z - 1.0).abs() < 0.05),
                MenuItemDef::checked("3:2 Zoom (150%)", "", MenuAction::VideoZoom(150), (z - 1.5).abs() < 0.05),
                MenuItemDef::checked("2:1 Double (200%)", "", MenuAction::VideoZoom(200), (z - 2.0).abs() < 0.05),
            ]
        }
        8 => vec![
            MenuItemDef::checked("Auto (Source Ratio)", "", MenuAction::VideoAspectRatio(0), current_aspect == AspectRatio::Auto),
            MenuItemDef::checked("16:9 Widescreen", "", MenuAction::VideoAspectRatio(1), current_aspect == AspectRatio::Ratio16_9),
            MenuItemDef::checked("4:3 Standard", "", MenuAction::VideoAspectRatio(2), current_aspect == AspectRatio::Ratio4_3),
            MenuItemDef::checked("21:9 Ultrawide", "", MenuAction::VideoAspectRatio(3), current_aspect == AspectRatio::Ratio21_9),
            MenuItemDef::checked("1:1 Square", "", MenuAction::VideoAspectRatio(4), current_aspect == AspectRatio::Ratio1_1),
            MenuItemDef::checked("Fill / Stretch", "", MenuAction::VideoAspectRatio(5), current_aspect == AspectRatio::Fill),
        ],
        10 => {
            let mut items = Vec::new();
            for (i, st) in engine.subtitle_tracks.iter().enumerate() {
                let title = if st.title.chars().count() > 26 {
                    let cut: String = st.title.chars().take(23).collect();
                    format!("{cut}...")
                } else {
                    st.title.clone()
                };
                items.push(MenuItemDef::checked(
                    title,
                    "",
                    MenuAction::SubtitleSelectTrack(i),
                    engine.subtitles.is_enabled && engine.active_subtitle_idx == Some(i),
                ));
            }
            items.push(MenuItemDef::checked(
                "Disable Subtitles",
                "",
                MenuAction::SubtitleSelectTrack(usize::MAX),
                !engine.subtitles.is_enabled || engine.active_subtitle_idx.is_none(),
            ));
            items
        }
        11 => {
            let c = engine.video_synth.adjustments.crop;
            vec![
                MenuItemDef::checked("No Crop", "", MenuAction::VideoCrop(0), c == CropMode::None),
                MenuItemDef::checked("16:9 Crop", "", MenuAction::VideoCrop(1), c == CropMode::Ratio16_9),
                MenuItemDef::checked("4:3 Crop", "", MenuAction::VideoCrop(2), c == CropMode::Ratio4_3),
                MenuItemDef::checked("21:9 Crop", "", MenuAction::VideoCrop(3), c == CropMode::Ratio21_9),
                MenuItemDef::checked("1:1 Crop", "", MenuAction::VideoCrop(4), c == CropMode::Ratio1_1),
            ]
        }
        _ => Vec::new(),
    }
}

pub struct MenuBarLayout {
    pub menu_rects: [Rect; 8],
    pub bar_rect: Rect,
}

impl MenuBarLayout {
    pub fn new(surface_w: i32) -> Self {
        let bar_h = 24;
        let mut cur_x = 6;
        let mut rects = [Rect::new(0, 0, 0, 0); 8];

        for (i, &title) in MENU_TITLES.iter().enumerate() {
            let text_w = measure_text(title, 1);
            let btn_w = text_w + 14;
            rects[i] = Rect::new(cur_x, 1, btn_w, bar_h - 2);
            cur_x += btn_w + 2;
        }

        Self {
            menu_rects: rects,
            bar_rect: Rect::new(0, 0, surface_w, bar_h),
        }
    }
}

pub fn render_menu_bar(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    state: &MenuBarState,
    layout: &MenuBarLayout,
    engine: &MediaPlayerEngine,
    viz_on: bool,
) {
    fill_rect(surface, surface_w, surface_h, layout.bar_rect, Color::rgb(22, 27, 34));
    fill_rect(surface, surface_w, surface_h, Rect::new(0, layout.bar_rect.h - 1, surface_w, 1), Color::rgb(48, 54, 61));

    for (i, &title) in MENU_TITLES.iter().enumerate() {
        let r = layout.menu_rects[i];
        let is_active = state.active_menu == Some(i);

        if is_active {
            fill_rounded_rect(surface, surface_w, surface_h, r, 3, Color::rgb(30, 41, 59));
            draw_rect_outline(surface, surface_w, surface_h, r, 1, Color::rgb(56, 189, 248));
        }

        let text_col = if is_active { Color::WHITE } else { Color::rgb(226, 232, 240) };
        draw_text(surface, surface_w, surface_h, r.x + 7, r.y + 4, title, text_col, 1);
    }

    if let Some(menu_idx) = state.active_menu {
        let items = get_menu_items(menu_idx, engine, viz_on);
        let menu_x = layout.menu_rects[menu_idx].x.min((surface_w - 256).max(4));
        let menu_y = layout.bar_rect.h;
        let item_h = 22;
        let menu_w = 250;
        let menu_h = (items.len() as i32) * item_h + 8;

        let drop_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);
        fill_rect(surface, surface_w, surface_h, Rect::new(drop_rect.x + 3, drop_rect.y + 3, drop_rect.w, drop_rect.h), Color::rgba(0, 0, 0, 80));
        fill_rounded_rect(surface, surface_w, surface_h, drop_rect, 6, Color::rgb(15, 23, 42));
        draw_rect_outline(surface, surface_w, surface_h, drop_rect, 1, Color::rgb(51, 65, 85));

        let mut iy = menu_y + 4;
        let mut active_sub_y = menu_y + 12;

        for (item_idx, item) in items.iter().enumerate() {
            let item_rect = Rect::new(menu_x + 4, iy, menu_w - 8, item_h);
            let is_hovered = state.hovered_item == Some(item_idx);

            if item.is_separator {
                let sep_y = iy + item_h / 2;
                fill_rect(surface, surface_w, surface_h, Rect::new(menu_x + 8, sep_y, menu_w - 16, 1), Color::rgb(48, 54, 61));
            } else {
                if is_hovered {
                    fill_rounded_rect(surface, surface_w, surface_h, item_rect, 4, Color::rgb(30, 41, 59));
                    draw_rect_outline(surface, surface_w, surface_h, item_rect, 1, Color::rgb(56, 189, 248));
                }
                if item.submenu_id.is_some() && state.active_submenu == item.submenu_id {
                    active_sub_y = iy;
                }

                if item.is_checked {
                    draw_text(surface, surface_w, surface_h, item_rect.x + 6, iy + 5, "v", Color::rgb(56, 189, 248), 1);
                }

                let text_col = if is_hovered { Color::WHITE } else { Color::rgb(226, 232, 240) };
                draw_text(surface, surface_w, surface_h, item_rect.x + 18, iy + 5, &item.label, text_col, 1);

                if item.submenu_id.is_some() {
                    draw_text(surface, surface_w, surface_h, item_rect.right() - 14, iy + 5, ">", Color::rgb(148, 163, 184), 1);
                } else if !item.shortcut.is_empty() {
                    let sc_w = measure_text(item.shortcut, 1);
                    draw_text(surface, surface_w, surface_h, item_rect.right() - sc_w - 6, iy + 5, item.shortcut, Color::rgb(148, 163, 184), 1);
                }
            }
            iy += item_h;
        }

        if let Some(sub_id) = state.active_submenu {
            let sub_items = get_submenu_items(sub_id, engine);
            if !sub_items.is_empty() {
                let sub_w = 240;
                let sub_x = if menu_x + menu_w + sub_w <= surface_w {
                    menu_x + menu_w - 4
                } else {
                    (menu_x - sub_w + 4).max(4)
                };
                let sub_h = (sub_items.len() as i32) * item_h + 8;
                let sub_y = active_sub_y.min((surface_h - sub_h - 8).max(menu_y));

                let sub_rect = Rect::new(sub_x, sub_y, sub_w, sub_h);
                fill_rect(surface, surface_w, surface_h, Rect::new(sub_rect.x + 3, sub_rect.y + 3, sub_rect.w, sub_rect.h), Color::rgba(0, 0, 0, 80));
                fill_rounded_rect(surface, surface_w, surface_h, sub_rect, 6, Color::rgb(15, 23, 42));
                draw_rect_outline(surface, surface_w, surface_h, sub_rect, 1, Color::rgb(56, 189, 248));

                let mut sy = sub_y + 4;
                for (s_idx, s_item) in sub_items.iter().enumerate() {
                    let s_rect = Rect::new(sub_x + 4, sy, sub_w - 8, item_h);
                    let is_sub_hov = state.hovered_sub_item == Some(s_idx);

                    if is_sub_hov {
                        fill_rounded_rect(surface, surface_w, surface_h, s_rect, 4, Color::rgb(30, 41, 59));
                        draw_rect_outline(surface, surface_w, surface_h, s_rect, 1, Color::rgb(56, 189, 248));
                    }

                    if s_item.is_checked {
                        draw_text(surface, surface_w, surface_h, s_rect.x + 6, sy + 5, "v", Color::rgb(56, 189, 248), 1);
                    }

                    let scol = if is_sub_hov { Color::WHITE } else { Color::rgb(226, 232, 240) };
                    draw_text(surface, surface_w, surface_h, s_rect.x + 18, sy + 5, &s_item.label, scol, 1);

                    sy += item_h;
                }
            }
        }
    }
}
