//! Top Menu Bar and Cascading Dropdown Menus for Spectra Media Studio.
//! Implements comprehensive VLC-style menus: Media, Playback, Audio, Video, Subtitle, Tools, View, Help.

use spectra_core::types::AspectRatio;
use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    // Media Menu
    MediaOpenFile,
    MediaOpenMultiple,
    MediaOpenDirectory,
    MediaOpenDisc,
    MediaOpenNetworkStream,
    MediaOpenCaptureDevice,
    MediaOpenClipboard,
    MediaRecent(usize),
    MediaSavePlaylist,
    MediaConvertSave,
    MediaStream,
    MediaQuit,

    // Playback Menu
    PlaybackPlayPause,
    PlaybackStop,
    PlaybackPrevious,
    PlaybackNext,
    PlaybackSpeed(u32), // 50 (0.5x), 75 (0.75x), 100 (1.0x), 125 (1.25x), 150 (1.5x), 200 (2.0x)
    PlaybackJumpForward(i32), // seconds: 10, 60, 300
    PlaybackJumpBackward(i32),
    PlaybackJumpToTime,
    PlaybackToggleLoop,
    PlaybackToggleShuffle,
    PlaybackRecord,

    // Audio Menu
    AudioSelectTrack(usize),
    AudioVisualizer(usize), // 0: Off, 1: 32-Band FFT, 2: Vinyl Turntable
    AudioEqualizerPreset(usize), // 0: Flat, 1: Rock, 2: Electronic, 3: Bass, 4: Vocal
    AudioVolUp,
    AudioVolDown,
    AudioMute,

    // Video Menu
    VideoSelectTrack(usize),
    VideoFullscreen,
    VideoAlwaysFit,
    VideoSetWallpaper,
    VideoZoom(u32), // 50%, 100%, 150%, 200%
    VideoAspectRatio(u8), // 0: Auto, 1: 16:9, 2: 4:3, 3: 21:9, 4: 1:1, 5: Fill
    VideoCrop(u8),
    VideoSnapshot,
    VideoTheme(usize), // 0: Synthetic Skyline 4K, 1: Deep Space Accretion 8K, 2: Ocean Aurora 1080p

    // Subtitle Menu
    SubtitleAddFile,
    SubtitleSelectTrack(usize),
    SubtitleToggle,

    // Tools Menu
    ToolsEffectsEqualizer,
    ToolsMediaInformation,
    ToolsTrackSync,
    ToolsPreferences,

    // View Menu
    ViewPlaylist,
    ViewVisualizer,
    ViewMinimalView,
    ViewStatusBar,

    // Help Menu
    HelpAbout,
    HelpShortcuts,
}

#[derive(Debug, Clone)]
pub struct MenuItemDef {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub action: Option<MenuAction>,
    pub is_separator: bool,
    pub is_checked: bool,
    pub submenu_id: Option<u8>,
}

impl MenuItemDef {
    pub const fn item(label: &'static str, shortcut: &'static str, action: MenuAction) -> Self {
        Self {
            label,
            shortcut,
            action: Some(action),
            is_separator: false,
            is_checked: false,
            submenu_id: None,
        }
    }

    pub const fn checked(label: &'static str, shortcut: &'static str, action: MenuAction, checked: bool) -> Self {
        Self {
            label,
            shortcut,
            action: Some(action),
            is_separator: false,
            is_checked: checked,
            submenu_id: None,
        }
    }

    pub const fn submenu(label: &'static str, submenu_id: u8) -> Self {
        Self {
            label,
            shortcut: "",
            action: None,
            is_separator: false,
            is_checked: false,
            submenu_id: Some(submenu_id),
        }
    }

    pub const fn separator() -> Self {
        Self {
            label: "",
            shortcut: "",
            action: None,
            is_separator: true,
            is_checked: false,
            submenu_id: None,
        }
    }
}

pub struct MenuBarState {
    pub active_menu: Option<usize>,
    pub hovered_item: Option<usize>,
    pub active_submenu: Option<u8>,
    pub hovered_sub_item: Option<usize>,
}

impl Default for MenuBarState {
    fn default() -> Self {
        Self {
            active_menu: None,
            hovered_item: None,
            active_submenu: None,
            hovered_sub_item: None,
        }
    }
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

// Top-level menu definitions
pub fn get_menu_items(menu_idx: usize, _current_aspect: AspectRatio, _current_speed: f32, is_muted: bool, viz_on: bool, is_playing: bool) -> Vec<MenuItemDef> {
    match menu_idx {
        0 => vec![ // Media
            MenuItemDef::item("Open File...", "Ctrl+O", MenuAction::MediaOpenFile),
            MenuItemDef::item("Open Multiple Files...", "Ctrl+Shift+O", MenuAction::MediaOpenMultiple),
            MenuItemDef::item("Open Directory...", "Ctrl+F", MenuAction::MediaOpenDirectory),
            MenuItemDef::item("Open Disc...", "Ctrl+D", MenuAction::MediaOpenDisc),
            MenuItemDef::item("Open Network Stream...", "Ctrl+N", MenuAction::MediaOpenNetworkStream),
            MenuItemDef::item("Open Capture Device...", "Ctrl+C", MenuAction::MediaOpenCaptureDevice),
            MenuItemDef::item("Open Location from clipboard", "Ctrl+V", MenuAction::MediaOpenClipboard),
            MenuItemDef::submenu("Open Recent Media", 1),
            MenuItemDef::separator(),
            MenuItemDef::item("Save Playlist to File...", "Ctrl+Y", MenuAction::MediaSavePlaylist),
            MenuItemDef::item("Convert / Save...", "Ctrl+R", MenuAction::MediaConvertSave),
            MenuItemDef::item("Stream...", "Ctrl+S", MenuAction::MediaStream),
            MenuItemDef::separator(),
            MenuItemDef::item("Quit", "Ctrl+Q", MenuAction::MediaQuit),
        ],
        1 => vec![ // Playback
            MenuItemDef::submenu("Speed", 2),
            MenuItemDef::item("Jump Forward (+10s)", "Right", MenuAction::PlaybackJumpForward(10)),
            MenuItemDef::item("Jump Backward (-10s)", "Left", MenuAction::PlaybackJumpBackward(10)),
            MenuItemDef::item("Jump to Specific Time", "Ctrl+T", MenuAction::PlaybackJumpToTime),
            MenuItemDef::separator(),
            MenuItemDef::item(if is_playing { "Pause" } else { "Play" }, "Space", MenuAction::PlaybackPlayPause),
            MenuItemDef::item("Stop", "S", MenuAction::PlaybackStop),
            MenuItemDef::item("Previous Track", "P", MenuAction::PlaybackPrevious),
            MenuItemDef::item("Next Track", "N", MenuAction::PlaybackNext),
            MenuItemDef::item("Toggle Loop / Repeat", "R", MenuAction::PlaybackToggleLoop),
            MenuItemDef::item("Toggle Shuffle", "Z", MenuAction::PlaybackToggleShuffle),
            MenuItemDef::item("Record Stream", "", MenuAction::PlaybackRecord),
        ],
        2 => vec![ // Audio
            MenuItemDef::submenu("Audio Track", 3),
            MenuItemDef::submenu("Audio Visualizer", 4),
            MenuItemDef::submenu("Equalizer Preset", 5),
            MenuItemDef::separator(),
            MenuItemDef::item("Increase Volume (+5%)", "Up", MenuAction::AudioVolUp),
            MenuItemDef::item("Decrease Volume (-5%)", "Down", MenuAction::AudioVolDown),
            MenuItemDef::checked("Mute Audio", "M", MenuAction::AudioMute, is_muted),
        ],
        3 => vec![ // Video
            MenuItemDef::submenu("Video Track", 6),
            MenuItemDef::item("Fullscreen", "F", MenuAction::VideoFullscreen),
            MenuItemDef::item("Always Fit Window", "", MenuAction::VideoAlwaysFit),
            MenuItemDef::item("Set as Desktop Wallpaper", "", MenuAction::VideoSetWallpaper),
            MenuItemDef::separator(),
            MenuItemDef::submenu("Zoom", 7),
            MenuItemDef::submenu("Aspect Ratio", 8),
            MenuItemDef::submenu("Synthetic Video Theme", 9),
            MenuItemDef::separator(),
            MenuItemDef::item("Take Snapshot", "S", MenuAction::VideoSnapshot),
        ],
        4 => vec![ // Subtitle
            MenuItemDef::item("Add Subtitle File...", "Ctrl+Shift+S", MenuAction::SubtitleAddFile),
            MenuItemDef::submenu("Subtitle Track", 10),
            MenuItemDef::item("Toggle Subtitles", "C", MenuAction::SubtitleToggle),
        ],
        5 => vec![ // Tools
            MenuItemDef::item("Effects and Filters / Equalizer", "Ctrl+E", MenuAction::ToolsEffectsEqualizer),
            MenuItemDef::item("Media Information & Telemetry", "Ctrl+I", MenuAction::ToolsMediaInformation),
            MenuItemDef::item("Track Synchronization...", "", MenuAction::ToolsTrackSync),
            MenuItemDef::item("Preferences", "Ctrl+P", MenuAction::ToolsPreferences),
        ],
        6 => vec![ // View
            MenuItemDef::item("Playlist Drawer", "Ctrl+L", MenuAction::ViewPlaylist),
            MenuItemDef::checked("32-Band Visualizer Overlay", "V", MenuAction::ViewVisualizer, viz_on),
            MenuItemDef::item("Minimal Interface Mode", "H", MenuAction::ViewMinimalView),
            MenuItemDef::item("Status Bar", "", MenuAction::ViewStatusBar),
        ],
        7 => vec![ // Help
            MenuItemDef::item("About Spectra...", "F1", MenuAction::HelpAbout),
            MenuItemDef::item("Keyboard Shortcuts", "?", MenuAction::HelpShortcuts),
        ],
        _ => Vec::new(),
    }
}

// Submenu definitions (cascaded menus)
pub fn get_submenu_items(submenu_id: u8, current_aspect: AspectRatio, current_speed: f32) -> Vec<MenuItemDef> {
    match submenu_id {
        1 => vec![ // Recent Media
            MenuItemDef::item("1. Synthetic Skyline 4K UHD", "", MenuAction::MediaRecent(0)),
            MenuItemDef::item("2. Deep Space Accretion Disk 8K", "", MenuAction::MediaRecent(1)),
            MenuItemDef::item("3. Ocean Aurora Borealis 1080p", "", MenuAction::MediaRecent(2)),
            MenuItemDef::item("4. Synthwave Sunset Overdrive (FLAC)", "", MenuAction::MediaRecent(3)),
        ],
        2 => vec![ // Speed
            MenuItemDef::checked("0.5x Slower", "", MenuAction::PlaybackSpeed(50), (current_speed - 0.5).abs() < 0.05),
            MenuItemDef::checked("0.75x Slower", "", MenuAction::PlaybackSpeed(75), (current_speed - 0.75).abs() < 0.05),
            MenuItemDef::checked("1.0x Normal Speed", "", MenuAction::PlaybackSpeed(100), (current_speed - 1.0).abs() < 0.05),
            MenuItemDef::checked("1.25x Faster", "", MenuAction::PlaybackSpeed(125), (current_speed - 1.25).abs() < 0.05),
            MenuItemDef::checked("1.5x Faster", "", MenuAction::PlaybackSpeed(150), (current_speed - 1.5).abs() < 0.05),
            MenuItemDef::checked("2.0x Double Speed", "", MenuAction::PlaybackSpeed(200), (current_speed - 2.0).abs() < 0.05),
        ],
        3 => vec![ // Audio Track
            MenuItemDef::checked("Track 1 - Stereo PCM 48kHz (Default)", "", MenuAction::AudioSelectTrack(0), true),
            MenuItemDef::checked("Track 2 - 5.1 Surround FLAC 24/96", "", MenuAction::AudioSelectTrack(1), false),
            MenuItemDef::checked("Disable Audio", "", MenuAction::AudioSelectTrack(99), false),
        ],
        4 => vec![ // Visualizer
            MenuItemDef::checked("Off", "", MenuAction::AudioVisualizer(0), false),
            MenuItemDef::checked("32-Band Dynamic FFT Spectrum", "", MenuAction::AudioVisualizer(1), true),
            MenuItemDef::checked("Vinyl Turntable Analog Disc", "", MenuAction::AudioVisualizer(2), false),
        ],
        5 => vec![ // Equalizer Preset
            MenuItemDef::item("Flat", "", MenuAction::AudioEqualizerPreset(0)),
            MenuItemDef::item("Rock", "", MenuAction::AudioEqualizerPreset(1)),
            MenuItemDef::item("Electronic / EDM", "", MenuAction::AudioEqualizerPreset(2)),
            MenuItemDef::item("Bass Boost (+6dB)", "", MenuAction::AudioEqualizerPreset(3)),
            MenuItemDef::item("Vocal Clarity", "", MenuAction::AudioEqualizerPreset(4)),
        ],
        6 => vec![ // Video Track
            MenuItemDef::checked("Track 1 - Ultra HD Master (Active)", "", MenuAction::VideoSelectTrack(0), true),
            MenuItemDef::checked("Disable Video", "", MenuAction::VideoSelectTrack(99), false),
        ],
        7 => vec![ // Zoom
            MenuItemDef::item("1:4 Quarter (25%)", "", MenuAction::VideoZoom(25)),
            MenuItemDef::item("1:2 Half (50%)", "", MenuAction::VideoZoom(50)),
            MenuItemDef::item("1:1 Original (100%)", "", MenuAction::VideoZoom(100)),
            MenuItemDef::item("2:1 Double (200%)", "", MenuAction::VideoZoom(200)),
        ],
        8 => vec![ // Aspect Ratio
            MenuItemDef::checked("Auto (Source Ratio)", "", MenuAction::VideoAspectRatio(0), current_aspect == AspectRatio::Auto),
            MenuItemDef::checked("16:9 Widescreen", "", MenuAction::VideoAspectRatio(1), current_aspect == AspectRatio::Ratio16_9),
            MenuItemDef::checked("4:3 Standard Television", "", MenuAction::VideoAspectRatio(2), current_aspect == AspectRatio::Ratio4_3),
            MenuItemDef::checked("21:9 Cinematic Ultrawide", "", MenuAction::VideoAspectRatio(3), current_aspect == AspectRatio::Ratio21_9),
            MenuItemDef::checked("1:1 Square Frame", "", MenuAction::VideoAspectRatio(4), current_aspect == AspectRatio::Ratio1_1),
            MenuItemDef::checked("Fill / Stretch Viewport", "", MenuAction::VideoAspectRatio(5), current_aspect == AspectRatio::Fill),
        ],
        9 => vec![ // Synthetic Theme
            MenuItemDef::item("Synthetic Skyline 4K UHD", "", MenuAction::VideoTheme(0)),
            MenuItemDef::item("Deep Space Accretion Disk 8K", "", MenuAction::VideoTheme(1)),
            MenuItemDef::item("Ocean Aurora Borealis 1080p", "", MenuAction::VideoTheme(2)),
        ],
        10 => vec![ // Subtitle Track
            MenuItemDef::checked("Track 1 [English SRT]", "", MenuAction::SubtitleSelectTrack(1), true),
            MenuItemDef::checked("Track 2 [Spanish VTT]", "", MenuAction::SubtitleSelectTrack(2), false),
            MenuItemDef::checked("Track 3 [French SRT]", "", MenuAction::SubtitleSelectTrack(3), false),
            MenuItemDef::checked("Disable Subtitles", "", MenuAction::SubtitleSelectTrack(0), false),
        ],
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

/// Renders the top menubar and any active dropdown or submenu panels.
pub fn render_menu_bar(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    state: &MenuBarState,
    layout: &MenuBarLayout,
    current_aspect: AspectRatio,
    current_speed: f32,
    is_muted: bool,
    viz_on: bool,
    is_playing: bool,
) {
    // 1. Menu bar strip background
    fill_rect(surface, surface_w, surface_h, layout.bar_rect, Color::rgb(22, 27, 34));
    fill_rect(surface, surface_w, surface_h, Rect::new(0, layout.bar_rect.h - 1, surface_w, 1), Color::rgb(48, 54, 61));

    // 2. Render Menubar Items (Media, Playback, Audio, Video, Subtitle, Tools, View, Help)
    for (i, &title) in MENU_TITLES.iter().enumerate() {
        let r = layout.menu_rects[i];
        let is_active = state.active_menu == Some(i);

        if is_active {
            fill_rounded_rect(surface, surface_w, surface_h, r, 3, Color::rgb(30, 41, 59));
            draw_rect_outline(surface, surface_w, surface_h, r, 1, Color::rgb(56, 189, 248));
        }

        let text_col = if is_active {
            Color::WHITE
        } else {
            Color::rgb(226, 232, 240)
        };

        draw_text(surface, surface_w, surface_h, r.x + 7, r.y + 4, title, text_col, 1);
    }

    // 3. Render Active Dropdown Menu
    if let Some(menu_idx) = state.active_menu {
        let items = get_menu_items(menu_idx, current_aspect, current_speed, is_muted, viz_on, is_playing);
        let menu_x = layout.menu_rects[menu_idx].x;
        let menu_y = layout.bar_rect.h;
        let item_h = 22;
        let menu_w = 230;
        let menu_h = (items.len() as i32) * item_h + 8;

        let drop_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

        // Drop shadow
        fill_rect(surface, surface_w, surface_h, Rect::new(drop_rect.x + 3, drop_rect.y + 3, drop_rect.w, drop_rect.h), Color::rgba(0, 0, 0, 80));

        // Acrylic body and glass border
        fill_rounded_rect(surface, surface_w, surface_h, drop_rect, 6, Color::rgb(15, 23, 42));
        draw_rect_outline(surface, surface_w, surface_h, drop_rect, 1, Color::rgb(51, 65, 85));

        // Render each item
        let mut iy = menu_y + 4;
        let mut hovered_submenu_pos: Option<(u8, i32, i32)> = None;

        for (item_idx, item) in items.iter().enumerate() {
            let item_rect = Rect::new(menu_x + 4, iy, menu_w - 8, item_h);
            let is_hovered = state.hovered_item == Some(item_idx);

            if item.is_separator {
                // Divider line
                let sep_y = iy + item_h / 2;
                fill_rect(surface, surface_w, surface_h, Rect::new(menu_x + 8, sep_y, menu_w - 16, 1), Color::rgb(48, 54, 61));
            } else {
                if is_hovered {
                    fill_rounded_rect(surface, surface_w, surface_h, item_rect, 4, Color::rgb(30, 41, 59));
                    draw_rect_outline(surface, surface_w, surface_h, item_rect, 1, Color::rgb(56, 189, 248));
                    if let Some(sub_id) = item.submenu_id {
                        hovered_submenu_pos = Some((sub_id, menu_x + menu_w - 2, iy));
                    }
                }

                // Checkmark
                if item.is_checked {
                    draw_text(surface, surface_w, surface_h, item_rect.x + 6, iy + 5, "v", Color::rgb(56, 189, 248), 1);
                }

                // Label
                let text_col = if is_hovered { Color::WHITE } else { Color::rgb(226, 232, 240) };
                draw_text(surface, surface_w, surface_h, item_rect.x + 18, iy + 5, item.label, text_col, 1);

                // Right-aligned shortcut or submenu indicator
                if let Some(_) = item.submenu_id {
                    draw_text(surface, surface_w, surface_h, item_rect.right() - 14, iy + 5, ">", Color::rgb(148, 163, 184), 1);
                } else if !item.shortcut.is_empty() {
                    let sc_w = measure_text(item.shortcut, 1);
                    draw_text(surface, surface_w, surface_h, item_rect.right() - sc_w - 6, iy + 5, item.shortcut, Color::rgb(148, 163, 184), 1);
                }
            }

            iy += item_h;
        }

        // 4. Render Active Cascaded Submenu
        let sub_to_render = state.active_submenu.or(hovered_submenu_pos.map(|(id, _, _)| id));
        if let Some(sub_id) = sub_to_render {
            let sub_items = get_submenu_items(sub_id, current_aspect, current_speed);
            if !sub_items.is_empty() {
                let sub_x = (menu_x + menu_w - 4).min(surface_w - 230);
                let sub_y = hovered_submenu_pos.map(|(_, _, y)| y).unwrap_or(menu_y + 12);
                let sub_w = 220;
                let sub_h = (sub_items.len() as i32) * item_h + 8;

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
                    draw_text(surface, surface_w, surface_h, s_rect.x + 18, sy + 5, s_item.label, scol, 1);

                    sy += item_h;
                }
            }
        }
    }
}
