//! About Spectra Modal Dialog.

use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

pub fn render_about_dialog(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
) -> Rect {
    let dialog_w = 420.min(surface_w - 40);
    let dialog_h = 280.min(surface_h - 40);
    let dialog_x = (surface_w - dialog_w) / 2;
    let dialog_y = (surface_h - dialog_h) / 2;
    let rect = Rect::new(dialog_x, dialog_y, dialog_w, dialog_h);

    // Dim background scrim
    fill_rect(surface, surface_w, surface_h, Rect::new(0, 0, surface_w, surface_h), Color::rgba(0, 0, 0, 140));

    // Modal dialog body
    fill_rounded_rect(surface, surface_w, surface_h, rect, 10, Color::rgb(15, 23, 42));
    draw_rect_outline(surface, surface_w, surface_h, rect, 1, Color::rgb(56, 189, 248));

    // Header with Logo
    let logo_pill = Rect::new(dialog_x + 16, dialog_y + 16, 32, 28);
    fill_rounded_rect(surface, surface_w, surface_h, logo_pill, 6, Color::rgb(56, 189, 248));
    draw_text(surface, surface_w, surface_h, dialog_x + 27, dialog_y + 21, ">", Color::rgb(15, 23, 42), 2);

    draw_text(surface, surface_w, surface_h, dialog_x + 58, dialog_y + 16, "Spectra Media Studio", Color::WHITE, 2);
    draw_text(surface, surface_w, surface_h, dialog_x + 58, dialog_y + 36, "v1.0.0 (Pure Rust Engine)", Color::rgb(56, 189, 248), 1);

    // Separator line
    fill_rect(surface, surface_w, surface_h, Rect::new(dialog_x + 16, dialog_y + 54, dialog_w - 32, 1), Color::rgb(51, 65, 85));

    // Information rows
    let rows: [(&str, &str); 6] = [
        ("Architecture", "100% Pure Rust 1.99.1 (Zero C/C++, Zero FFmpeg)"),
        ("Video Pipeline", "8K FUHD, 4K UHD, 1080p, Multi-Aspect Ratio Letterbox"),
        ("Audio Engine", "32-Band FFT Spectrum Equalizer, 125% Volume Boost"),
        ("Containers", "MKV, MP4, WebM, AVI, MOV, FLAC, MP3, WAV, OGG"),
        ("Subtitles", "SubRip (.srt) and WebVTT (.vtt) millisecond sync"),
        ("Operating System", "NovaOS Default Media Player & Standalone Linux/X11"),
    ];

    let mut ry = dialog_y + 66;
    for (k, v) in rows {
        draw_text(surface, surface_w, surface_h, dialog_x + 18, ry, k, Color::rgb(148, 163, 184), 1);
        draw_text(surface, surface_w, surface_h, dialog_x + 18, ry + 12, v, Color::rgb(226, 232, 240), 1);
        ry += 28;
    }

    // Close Button at bottom right
    let btn_w = 70;
    let btn_h = 24;
    let btn_rect = Rect::new(dialog_x + dialog_w - btn_w - 18, dialog_y + dialog_h - btn_h - 14, btn_w, btn_h);
    fill_rounded_rect(surface, surface_w, surface_h, btn_rect, 4, Color::rgb(56, 189, 248));
    draw_text(surface, surface_w, surface_h, btn_rect.x + 18, btn_rect.y + 6, "Close", Color::rgb(15, 23, 42), 1);

    btn_rect
}
