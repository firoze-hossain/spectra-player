//! Top media header bar (positioned below the 24px menubar):
//! displays media title, artist, resolution/format/backend badges, and quick action buttons.

use spectra_core::types::{DecoderBackend, MediaMetadata};
use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy)]
pub struct TopBarActionAreas {
    pub open_btn: Rect,
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
    let top_y = 24; // Starts directly below 24px menu bar
    let bar_h = 42;
    let bar_rect = Rect::new(0, top_y, surface_w, bar_h);

    fill_rect(surface, surface_w, surface_h, bar_rect, Color::rgba(11, 15, 25, 228));
    fill_rect(
        surface,
        surface_w,
        surface_h,
        Rect::new(0, top_y + bar_h - 1, surface_w, 1),
        Color::rgba(255, 255, 255, 24),
    );

    // 1. Spectra Logo & Title (Left)
    let logo_pill = Rect::new(12, top_y + 9, 26, 24);
    fill_rounded_rect(surface, surface_w, surface_h, logo_pill, 5, Color::rgb(56, 189, 248));
    draw_text(surface, surface_w, surface_h, 20, top_y + 13, ">", Color::rgb(15, 23, 42), 2);

    let title_x = 48;
    let max_title_chars = ((surface_w - 440).max(160) / 7) as usize;
    let title = if metadata.title.chars().count() > max_title_chars {
        let cut: String = metadata.title.chars().take(max_title_chars.saturating_sub(3)).collect();
        format!("{cut}...")
    } else {
        metadata.title.clone()
    };
    draw_text(surface, surface_w, surface_h, title_x, top_y + 8, &title, Color::WHITE, 1);

    let sub_line = if metadata.is_live_file {
        if metadata.artist.is_empty() || metadata.artist == "Local Media File" {
            metadata.path.clone()
        } else {
            metadata.artist.clone()
        }
    } else {
        metadata.artist.clone()
    };
    let sub_trimmed = if sub_line.chars().count() > 48 {
        let cut: String = sub_line.chars().take(45).collect();
        format!("...{cut}")
    } else {
        sub_line
    };
    draw_text(
        surface,
        surface_w,
        surface_h,
        title_x,
        top_y + 22,
        &sub_trimmed,
        Color::rgb(148, 163, 184),
        1,
    );

    // 2. Dynamic Badges (Resolution, Format, FPS/Rate, Backend) — shown for active media
    let mut badge_x = title_x + measure_text(&title, 1).max(measure_text(&sub_trimmed, 1)) + 16;
    let right_limit = surface_w - 190;

    if !metadata.is_live_file {
        let ready_text = "STANDBY";
        let rw = measure_text(ready_text, 1) + 12;
        if badge_x + rw < right_limit {
            let rrect = Rect::new(badge_x, top_y + 11, rw, 20);
            fill_rounded_rect(surface, surface_w, surface_h, rrect, 4, Color::rgba(56, 189, 248, 28));
            draw_rect_outline(surface, surface_w, surface_h, rrect, 1, Color::rgb(56, 189, 248));
            draw_text(surface, surface_w, surface_h, badge_x + 6, top_y + 15, ready_text, Color::rgb(56, 189, 248), 1);
        }
    } else {
        let res_text = if metadata.format.is_audio_only() {
            "HI-RES AUDIO".to_string()
        } else if metadata.width >= 7000 {
            "8K HDR".to_string()
        } else if metadata.width >= 3500 {
            "4K UHD".to_string()
        } else if metadata.width >= 2400 {
            "1440p 2K".to_string()
        } else if metadata.width >= 1800 {
            "1080p FHD".to_string()
        } else if metadata.width >= 1200 {
            "720p HD".to_string()
        } else if metadata.width > 0 {
            format!("{}p", metadata.height)
        } else {
            "MEDIA".to_string()
        };

        let res_col = if metadata.width >= 7000 {
            Color::rgb(244, 63, 94)
        } else if metadata.width >= 3500 {
            Color::rgb(6, 182, 212)
        } else if metadata.width >= 2400 {
            Color::rgb(168, 85, 247)
        } else {
            Color::rgb(16, 185, 129)
        };

        let res_w = measure_text(&res_text, 1) + 12;
        if badge_x + res_w < right_limit {
            let res_rect = Rect::new(badge_x, top_y + 11, res_w, 20);
            fill_rounded_rect(surface, surface_w, surface_h, res_rect, 4, Color::rgba(res_col.r, res_col.g, res_col.b, 40));
            draw_rect_outline(surface, surface_w, surface_h, res_rect, 1, res_col);
            draw_text(surface, surface_w, surface_h, badge_x + 6, top_y + 15, &res_text, res_col, 1);
            badge_x += res_w + 6;
        }

        let fmt_text = metadata.format.badge();
        let fmt_w = measure_text(fmt_text, 1) + 12;
        if badge_x + fmt_w < right_limit {
            let fmt_rect = Rect::new(badge_x, top_y + 11, fmt_w, 20);
            fill_rounded_rect(surface, surface_w, surface_h, fmt_rect, 4, Color::rgba(255, 255, 255, 24));
            draw_rect_outline(surface, surface_w, surface_h, fmt_rect, 1, Color::rgba(255, 255, 255, 50));
            draw_text(surface, surface_w, surface_h, badge_x + 6, top_y + 15, fmt_text, Color::WHITE, 1);
            badge_x += fmt_w + 6;
        }

        if !metadata.format.is_audio_only() && fps > 0.5 {
            let fps_text = format!("{:.0} FPS", fps);
            let fps_w = measure_text(&fps_text, 1) + 12;
            if badge_x + fps_w < right_limit {
                let fps_rect = Rect::new(badge_x, top_y + 11, fps_w, 20);
                fill_rounded_rect(surface, surface_w, surface_h, fps_rect, 4, Color::rgba(245, 158, 11, 30));
                draw_rect_outline(surface, surface_w, surface_h, fps_rect, 1, Color::rgb(245, 158, 11));
                draw_text(surface, surface_w, surface_h, badge_x + 6, top_y + 15, &fps_text, Color::rgb(251, 191, 36), 1);
                badge_x += fps_w + 6;
            }
        }

        let eng_badge = match metadata.backend {
            DecoderBackend::Synthesizer => "STANDBY",
            DecoderBackend::PureRustSymphonia => "RUST AUDIO",
            DecoderBackend::PureRustH264 => "RUST H264",
            DecoderBackend::FfmpegUniversal => "FFMPEG",
            DecoderBackend::HybridSymphoniaFfmpeg => "HYBRID",
        };
        let eng_w = measure_text(eng_badge, 1) + 12;
        if badge_x + eng_w < right_limit {
            let eng_rect = Rect::new(badge_x, top_y + 11, eng_w, 20);
            let col = Color::rgb(56, 189, 248);
            fill_rounded_rect(surface, surface_w, surface_h, eng_rect, 4, Color::rgba(col.r, col.g, col.b, 28));
            draw_rect_outline(surface, surface_w, surface_h, eng_rect, 1, col);
            draw_text(surface, surface_w, surface_h, badge_x + 6, top_y + 15, eng_badge, col, 1);
        }
    }

    // 3. Right Action Buttons: [Open] [i] [==] [[]]
    let mut btn_x = surface_w - 36;
    let fs_rect = Rect::new(btn_x, top_y + 7, 28, 28);
    fill_rounded_rect(surface, surface_w, surface_h, fs_rect, 4, Color::rgba(255, 255, 255, 20));
    draw_rect_outline(surface, surface_w, surface_h, fs_rect, 1, Color::rgba(255, 255, 255, 40));
    draw_text(surface, surface_w, surface_h, btn_x + 7, top_y + 15, if is_fullscreen { "X" } else { "[]" }, Color::WHITE, 1);

    btn_x -= 34;
    let pl_rect = Rect::new(btn_x, top_y + 7, 28, 28);
    fill_rounded_rect(surface, surface_w, surface_h, pl_rect, 4, Color::rgba(255, 255, 255, 20));
    draw_rect_outline(surface, surface_w, surface_h, pl_rect, 1, Color::rgba(255, 255, 255, 40));
    draw_text(surface, surface_w, surface_h, btn_x + 7, top_y + 15, "==", Color::WHITE, 1);

    btn_x -= 34;
    let info_rect = Rect::new(btn_x, top_y + 7, 28, 28);
    fill_rounded_rect(surface, surface_w, surface_h, info_rect, 4, Color::rgba(255, 255, 255, 20));
    draw_rect_outline(surface, surface_w, surface_h, info_rect, 1, Color::rgba(255, 255, 255, 40));
    draw_text(surface, surface_w, surface_h, btn_x + 11, top_y + 15, "i", Color::rgb(56, 189, 248), 1);

    btn_x -= 54;
    let open_rect = Rect::new(btn_x, top_y + 7, 48, 28);
    fill_rounded_rect(surface, surface_w, surface_h, open_rect, 4, Color::rgba(56, 189, 248, 45));
    draw_rect_outline(surface, surface_w, surface_h, open_rect, 1, Color::rgb(56, 189, 248));
    draw_text(surface, surface_w, surface_h, btn_x + 8, top_y + 15, "OPEN", Color::WHITE, 1);

    TopBarActionAreas {
        open_btn: open_rect,
        info_btn: info_rect,
        playlist_btn: pl_rect,
        fullscreen_btn: fs_rect,
    }
}
