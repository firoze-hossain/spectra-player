//! Interactive Modal Dialogs:
//! - Open Network Stream / URL (Ctrl+N)
//! - Jump to Specific Time (Ctrl+T)
//! - Keyboard Shortcuts Reference (?)
//! - Preferences (Ctrl+P)

use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModal {
    None,
    OpenUrl,
    JumpToTime,
    Shortcuts,
    Preferences,
}

pub struct ModalActionAreas {
    pub dialog_rect: Rect,
    pub close_btn: Rect,
    pub submit_btn: Rect,
    pub pref_codec_btn: Rect,
    pub pref_autohide_btn: Rect,
    pub pref_res_cap_btn: Rect,
}

pub fn render_input_modal(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    modal: ActiveModal,
    input_buf: &str,
    auto_hide_fs: bool,
    max_decode_w: u32,
    ffmpeg_ready: bool,
) -> ModalActionAreas {
    let dw = 480.min(surface_w - 40);
    let dh = match modal {
        ActiveModal::Shortcuts => 340.min(surface_h - 60),
        ActiveModal::Preferences => 260.min(surface_h - 60),
        _ => 170.min(surface_h - 60),
    };
    let dx = (surface_w - dw) / 2;
    let dy = (surface_h - dh) / 2;
    let dialog_rect = Rect::new(dx, dy, dw, dh);

    fill_rect(surface, surface_w, surface_h, Rect::new(0, 0, surface_w, surface_h), Color::rgba(0, 0, 0, 150));
    fill_rounded_rect(surface, surface_w, surface_h, dialog_rect, 10, Color::rgb(15, 23, 42));
    draw_rect_outline(surface, surface_w, surface_h, dialog_rect, 1, Color::rgb(56, 189, 248));

    let title = match modal {
        ActiveModal::OpenUrl => "OPEN NETWORK STREAM / FILE PATH",
        ActiveModal::JumpToTime => "JUMP TO TIME (MM:SS or HH:MM:SS)",
        ActiveModal::Shortcuts => "SPECTRA KEYBOARD SHORTCUTS",
        ActiveModal::Preferences => "SPECTRA PREFERENCES",
        ActiveModal::None => "",
    };
    fill_rounded_rect(surface, surface_w, surface_h, Rect::new(dx, dy, dw, 34), 10, Color::rgba(30, 41, 59, 235));
    draw_text(surface, surface_w, surface_h, dx + 14, dy + 11, title, Color::rgb(56, 189, 248), 1);

    let close_btn = Rect::new(dx + dw - 28, dy + 7, 20, 20);
    fill_rounded_rect(surface, surface_w, surface_h, close_btn, 4, Color::rgba(255, 255, 255, 20));
    draw_text(surface, surface_w, surface_h, close_btn.x + 6, close_btn.y + 5, "X", Color::WHITE, 1);

    let mut submit_btn = Rect::default();
    let mut pref_codec_btn = Rect::default();
    let mut pref_autohide_btn = Rect::default();
    let mut pref_res_cap_btn = Rect::default();

    match modal {
        ActiveModal::OpenUrl | ActiveModal::JumpToTime => {
            let hint = if modal == ActiveModal::OpenUrl {
                "Enter HTTP/HTTPS/RTSP/HLS URL or local file path (Ctrl+V to paste):"
            } else {
                "Enter target timestamp (e.g. 01:30 or 1:05:00 or seconds):"
            };
            draw_text(surface, surface_w, surface_h, dx + 16, dy + 48, hint, Color::rgb(148, 163, 184), 1);

            let input_rect = Rect::new(dx + 16, dy + 70, dw - 32, 32);
            fill_rounded_rect(surface, surface_w, surface_h, input_rect, 5, Color::rgb(30, 41, 59));
            draw_rect_outline(surface, surface_w, surface_h, input_rect, 1, Color::rgb(56, 189, 248));

            let max_chars = ((dw - 52) / 7).max(16) as usize;
            let disp = if input_buf.chars().count() > max_chars {
                let skip = input_buf.chars().count() - max_chars;
                format!("{}|", input_buf.chars().skip(skip).collect::<String>())
            } else {
                format!("{input_buf}|")
            };
            draw_text(surface, surface_w, surface_h, input_rect.x + 10, input_rect.y + 10, &disp, Color::WHITE, 1);

            submit_btn = Rect::new(dx + dw - 106, dy + dh - 38, 90, 26);
            fill_rounded_rect(surface, surface_w, surface_h, submit_btn, 5, Color::rgb(56, 189, 248));
            let btn_lbl = if modal == ActiveModal::OpenUrl { "Play Stream" } else { "Jump" };
            draw_text(surface, surface_w, surface_h, submit_btn.x + 12, submit_btn.y + 8, btn_lbl, Color::rgb(15, 23, 42), 1);
        }
        ActiveModal::Shortcuts => {
            let rows = [
                ("Space", "Play / Pause"),
                ("Left / Right", "Seek -10s / +10s"),
                ("Up / Down / Wheel", "Volume +5% / -5% (up to 200%)"),
                ("Ctrl+O / Ctrl+F", "Open File / Open Directory"),
                ("Ctrl+N / Ctrl+V", "Open URL / Open Clipboard"),
                ("Ctrl+Shift+S / C", "Add Subtitle / Toggle Subtitles"),
                ("G / H", "Subtitle Delay -50ms / +50ms"),
                ("[ / ] / E", "Speed -0.25x / +0.25x / Step Frame"),
                ("B / R / Z", "A-B Loop / Repeat Mode / Shuffle"),
                ("F / Double-Click", "Fullscreen Toggle"),
                ("V / L / I / Ctrl+E", "Visualizer / Playlist / Info / EQ"),
                ("Shift+S / M / A", "PNG Snapshot / Mute / Aspect Ratio"),
            ];
            let mut ry = dy + 46;
            for (k, desc) in rows {
                if ry + 16 > dy + dh - 14 {
                    break;
                }
                draw_text(surface, surface_w, surface_h, dx + 18, ry, k, Color::rgb(56, 189, 248), 1);
                draw_text(surface, surface_w, surface_h, dx + 180, ry, desc, Color::rgb(226, 232, 240), 1);
                ry += 22;
            }
        }
        ActiveModal::Preferences => {
            let r1 = dy + 54;
            draw_text(surface, surface_w, surface_h, dx + 18, r1 + 6, "Universal Codecs (FFmpeg):", Color::rgb(226, 232, 240), 1);
            pref_codec_btn = Rect::new(dx + 230, r1, 210, 26);
            fill_rounded_rect(surface, surface_w, surface_h, pref_codec_btn, 5, if ffmpeg_ready { Color::rgb(16, 185, 129) } else { Color::rgb(14, 165, 233) });
            draw_text(surface, surface_w, surface_h, pref_codec_btn.x + 14, pref_codec_btn.y + 8, if ffmpeg_ready { "Installed & Ready" } else { "Install Codec Pack" }, Color::WHITE, 1);

            let r2 = dy + 98;
            draw_text(surface, surface_w, surface_h, dx + 18, r2 + 6, "Auto-Hide Controls (FS):", Color::rgb(226, 232, 240), 1);
            pref_autohide_btn = Rect::new(dx + 230, r2, 120, 26);
            fill_rounded_rect(surface, surface_w, surface_h, pref_autohide_btn, 5, if auto_hide_fs { Color::rgb(14, 165, 233) } else { Color::rgba(255, 255, 255, 20) });
            draw_text(surface, surface_w, surface_h, pref_autohide_btn.x + 16, pref_autohide_btn.y + 8, if auto_hide_fs { "Enabled" } else { "Disabled" }, Color::WHITE, 1);

            let r3 = dy + 142;
            draw_text(surface, surface_w, surface_h, dx + 18, r3 + 6, "Max Decode Cap (4K/8K):", Color::rgb(226, 232, 240), 1);
            pref_res_cap_btn = Rect::new(dx + 230, r3, 150, 26);
            fill_rounded_rect(surface, surface_w, surface_h, pref_res_cap_btn, 5, Color::rgba(255, 255, 255, 22));
            draw_text(surface, surface_w, surface_h, pref_res_cap_btn.x + 14, pref_res_cap_btn.y + 8, &format!("{} px width", max_decode_w), Color::WHITE, 1);
        }
        ActiveModal::None => {}
    }

    ModalActionAreas {
        dialog_rect,
        close_btn,
        submit_btn,
        pref_codec_btn,
        pref_autohide_btn,
        pref_res_cap_btn,
    }
}
