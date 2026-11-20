//! Top navigation bar: title, artist, format/resolution badges, and quick action buttons.

use spectra_core::types::MediaMetadata;
use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy)]
pub struct TopBarActionAreas {
    pub info_btn: Rect,
    pub playlist_btn: Rect,
    pub fullscreen_btn: Rect,
}

pub fn render_top_bar(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    metadata: &MediaMetadata,
    fps: f32,
    is_fullscreen: bool,
) -> TopBarActionAreas {
    let bar_h = 44;
    let bar_rect = Rect::new(0, 0, surface_w, bar_h);

    // Frosted dark acrylic top gradient
    fill_rect(surface, surface_w, surface_h, bar_rect, Color::rgba(11, 15, 25, 235));
    // Subtle bottom glass border
    fill_rect(surface, surface_w, surface_h, Rect::new(0, bar_h - 1, surface_w, 1), Color::rgba(255, 255, 255, 28));

    // 1. Nova Player Logo & Title (Left)
    let logo_pill = Rect::new(12, 10, 26, 24);
    fill_rounded_rect(surface, surface_w, surface_h, logo_pill, 5, Color::rgb(56, 189, 248));
    draw_text(surface, surface_w, surface_h, 20, 14, ">", Color::rgb(15, 23, 42), 2);

    let title_x = 48;
    let title = if metadata.title.len() > 38 {
        format!("{}...", &metadata.title[..35])
    } else {
        metadata.title.clone()
    };
    draw_text(surface, surface_w, surface_h, title_x, 10, &title, Color::WHITE, 1);
    draw_text(surface, surface_w, surface_h, title_x, 24, &metadata.artist, Color::rgb(148, 163, 184), 1);

    // 2. Badges (Resolution, Format, FPS)
    let mut badge_x = title_x + measure_text(&title, 1) + 20;

    // Resolution Badge
    let res_text = if metadata.width >= 7000 {
        "8K HDR"
    } else if metadata.width >= 3500 {
        "4K UHD"
    } else if metadata.width >= 1800 {
        "1080p FHD"
    } else if metadata.format.is_audio_only() {
        "HI-RES AUDIO"
    } else {
        "HD VIDEO"
    };

    let res_col = if metadata.width >= 7000 {
        Color::rgb(244, 63, 94) // Rose
    } else if metadata.width >= 3500 {
        Color::rgb(6, 182, 212) // Cyan
    } else {
        Color::rgb(16, 185, 129) // Emerald
    };

    let res_w = measure_text(res_text, 1) + 12;
    if badge_x + res_w < surface_w - 220 {
        let res_rect = Rect::new(badge_x, 12, res_w, 20);
        fill_rounded_rect(surface, surface_w, surface_h, res_rect, 4, Color::rgba(res_col.r, res_col.g, res_col.b, 40));
        draw_rect_outline(surface, surface_w, surface_h, res_rect, 1, res_col);
        draw_text(surface, surface_w, surface_h, badge_x + 6, 16, res_text, res_col, 1);
        badge_x += res_w + 8;
    }

    // Format Badge
    let fmt_text = metadata.format.badge();
    let fmt_w = measure_text(fmt_text, 1) + 12;
    if badge_x + fmt_w < surface_w - 220 {
        let fmt_rect = Rect::new(badge_x, 12, fmt_w, 20);
        fill_rounded_rect(surface, surface_w, surface_h, fmt_rect, 4, Color::rgba(255, 255, 255, 24));
        draw_rect_outline(surface, surface_w, surface_h, fmt_rect, 1, Color::rgba(255, 255, 255, 50));
        draw_text(surface, surface_w, surface_h, badge_x + 6, 16, fmt_text, Color::WHITE, 1);
        badge_x += fmt_w + 8;
    }

    // FPS Badge
    if !metadata.format.is_audio_only() {
        let fps_text = format!("{:.0} FPS", fps);
        let fps_w = measure_text(&fps_text, 1) + 12;
        if badge_x + fps_w < surface_w - 220 {
            let fps_rect = Rect::new(badge_x, 12, fps_w, 20);
            fill_rounded_rect(surface, surface_w, surface_h, fps_rect, 4, Color::rgba(245, 158, 11, 30));
            draw_rect_outline(surface, surface_w, surface_h, fps_rect, 1, Color::rgb(245, 158, 11));
            draw_text(surface, surface_w, surface_h, badge_x + 6, 16, &fps_text, Color::rgb(251, 191, 36), 1);
        }
    }

    // 3. Right Action Buttons
    let mut btn_x = surface_w - 36;
    let fs_rect = Rect::new(btn_x, 8, 28, 28);
    fill_rounded_rect(surface, surface_w, surface_h, fs_rect, 4, Color::rgba(255, 255, 255, 20));
    draw_rect_outline(surface, surface_w, surface_h, fs_rect, 1, Color::rgba(255, 255, 255, 40));
    draw_text(surface, surface_w, surface_h, btn_x + 8, 15, if is_fullscreen { "X" } else { "[]" }, Color::WHITE, 1);

    btn_x -= 36;
    let pl_rect = Rect::new(btn_x, 8, 28, 28);
    fill_rounded_rect(surface, surface_w, surface_h, pl_rect, 4, Color::rgba(255, 255, 255, 20));
    draw_rect_outline(surface, surface_w, surface_h, pl_rect, 1, Color::rgba(255, 255, 255, 40));
    draw_text(surface, surface_w, surface_h, btn_x + 7, 15, "==", Color::WHITE, 1);

    btn_x -= 36;
    let info_rect = Rect::new(btn_x, 8, 28, 28);
    fill_rounded_rect(surface, surface_w, surface_h, info_rect, 4, Color::rgba(255, 255, 255, 20));
    draw_rect_outline(surface, surface_w, surface_h, info_rect, 1, Color::rgba(255, 255, 255, 40));
    draw_text(surface, surface_w, surface_h, btn_x + 10, 15, "i", Color::rgb(56, 189, 248), 1);

    TopBarActionAreas {
        info_btn: info_rect,
        playlist_btn: pl_rect,
        fullscreen_btn: fs_rect,
    }
}
