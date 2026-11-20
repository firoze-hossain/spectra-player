//! Bottom player control island: scrub seek bar, transport, volume, speed, and track selectors.

use nova_player_core::types::{AspectRatio, PlaybackState};
use crate::draw::{draw_circle, draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy, Default)]
pub struct BottomBarActionAreas {
    pub scrub_bar: Rect,
    pub play_pause_btn: Rect,
    pub prev_btn: Rect,
    pub next_btn: Rect,
    pub step_back_btn: Rect,
    pub step_fwd_btn: Rect,
    pub stop_btn: Rect,
    pub volume_mute_btn: Rect,
    pub volume_slider: Rect,
    pub speed_btn: Rect,
    pub aspect_btn: Rect,
    pub viz_btn: Rect,
    pub cc_btn: Rect,
}

pub fn render_bottom_bar(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    state: PlaybackState,
    position_ms: u64,
    duration_ms: u64,
    volume: f32,
    is_muted: bool,
    speed: f32,
    aspect: AspectRatio,
    viz_active: bool,
    subtitles_active: bool,
) -> BottomBarActionAreas {
    let bar_h = 74;
    let margin = 12;
    let bar_w = surface_w - margin * 2;
    let bar_y = surface_h - bar_h - margin;
    let bar_rect = Rect::new(margin, bar_y, bar_w, bar_h);

    // 1. Floating acrylic frosted glass card with subtle shadow and border
    fill_rounded_rect(surface, surface_w, surface_h, Rect::new(margin + 2, bar_y + 3, bar_w, bar_h), 12, Color::rgba(0, 0, 0, 70));
    fill_rounded_rect(surface, surface_w, surface_h, bar_rect, 10, Color::rgba(15, 23, 42, 240));
    draw_rect_outline(surface, surface_w, surface_h, bar_rect, 1, Color::rgba(255, 255, 255, 25));

    // 2. Scrub Seek Timeline Bar (Top of island)
    let scrub_margin_x = 18;
    let scrub_y = bar_y + 12;
    let scrub_w = bar_w - scrub_margin_x * 2;
    let scrub_h = 6;
    let scrub_rect = Rect::new(margin + scrub_margin_x, scrub_y, scrub_w, scrub_h);

    // Background track
    fill_rounded_rect(surface, surface_w, surface_h, scrub_rect, 3, Color::rgba(51, 65, 85, 220));

    // Buffered simulated preview
    let buf_progress = ((position_ms + 15_000) as f32 / duration_ms.max(1) as f32).clamp(0.0, 1.0);
    let buf_w = (scrub_w as f32 * buf_progress) as i32;
    fill_rounded_rect(surface, surface_w, surface_h, Rect::new(scrub_rect.x, scrub_y, buf_w, scrub_h), 3, Color::rgba(100, 116, 139, 180));

    // Played progress (electric cyan gradient)
    let play_progress = (position_ms as f32 / duration_ms.max(1) as f32).clamp(0.0, 1.0);
    let play_w = (scrub_w as f32 * play_progress) as i32;
    if play_w > 0 {
        fill_rounded_rect(surface, surface_w, surface_h, Rect::new(scrub_rect.x, scrub_y, play_w, scrub_h), 3, Color::rgb(56, 189, 248));
    }

    // Playhead thumb knob
    let knob_x = scrub_rect.x + play_w;
    let knob_y = scrub_y + scrub_h / 2;
    draw_circle(surface, surface_w, surface_h, knob_x, knob_y, 6, Color::rgb(255, 255, 255));
    draw_circle(surface, surface_w, surface_h, knob_x, knob_y, 3, Color::rgb(2, 132, 199));

    // 3. Bottom Controls Row
    let ctrl_y = bar_y + 28;
    let mut cur_x = margin + 18;

    // Transport buttons: Prev, -10s, Play/Pause, +10s, Next, Stop
    let prev_rect = Rect::new(cur_x, ctrl_y + 4, 26, 26);
    fill_rounded_rect(surface, surface_w, surface_h, prev_rect, 4, Color::rgba(255, 255, 255, 16));
    draw_text(surface, surface_w, surface_h, cur_x + 6, ctrl_y + 11, "|<", Color::WHITE, 1);
    cur_x += 32;

    let back_rect = Rect::new(cur_x, ctrl_y + 4, 30, 26);
    fill_rounded_rect(surface, surface_w, surface_h, back_rect, 4, Color::rgba(255, 255, 255, 16));
    draw_text(surface, surface_w, surface_h, cur_x + 5, ctrl_y + 11, "-10", Color::WHITE, 1);
    cur_x += 36;

    // Center Play/Pause circular accent button
    let play_rect = Rect::new(cur_x, ctrl_y, 34, 34);
    fill_rounded_rect(surface, surface_w, surface_h, play_rect, 17, Color::rgb(56, 189, 248));
    if state == PlaybackState::Playing {
        // Pause bars
        fill_rect(surface, surface_w, surface_h, Rect::new(cur_x + 11, ctrl_y + 10, 4, 14), Color::rgb(15, 23, 42));
        fill_rect(surface, surface_w, surface_h, Rect::new(cur_x + 19, ctrl_y + 10, 4, 14), Color::rgb(15, 23, 42));
    } else {
        // Play triangle
        draw_text(surface, surface_w, surface_h, cur_x + 13, ctrl_y + 10, ">", Color::rgb(15, 23, 42), 2);
    }
    cur_x += 42;

    let fwd_rect = Rect::new(cur_x, ctrl_y + 4, 30, 26);
    fill_rounded_rect(surface, surface_w, surface_h, fwd_rect, 4, Color::rgba(255, 255, 255, 16));
    draw_text(surface, surface_w, surface_h, cur_x + 5, ctrl_y + 11, "+10", Color::WHITE, 1);
    cur_x += 36;

    let next_rect = Rect::new(cur_x, ctrl_y + 4, 26, 26);
    fill_rounded_rect(surface, surface_w, surface_h, next_rect, 4, Color::rgba(255, 255, 255, 16));
    draw_text(surface, surface_w, surface_h, cur_x + 6, ctrl_y + 11, ">|", Color::WHITE, 1);
    cur_x += 32;

    let stop_rect = Rect::new(cur_x, ctrl_y + 4, 26, 26);
    fill_rounded_rect(surface, surface_w, surface_h, stop_rect, 4, Color::rgba(255, 255, 255, 16));
    fill_rect(surface, surface_w, surface_h, Rect::new(cur_x + 8, ctrl_y + 12, 10, 10), Color::rgb(239, 68, 68));
    cur_x += 38;

    // Time indicators: "00:01:24 / 00:03:00"
    let cur_time_str = format_time(position_ms);
    let dur_time_str = format_time(duration_ms);
    let time_full = format!("{} / {}", cur_time_str, dur_time_str);
    draw_text(surface, surface_w, surface_h, cur_x, ctrl_y + 11, &time_full, Color::rgb(226, 232, 240), 1);

    // 4. Right side controls: Volume, Speed, Aspect, Viz, CC
    let mut right_x = bar_rect.right() - 20;

    // Subtitle button [CC]
    right_x -= 34;
    let cc_rect = Rect::new(right_x, ctrl_y + 4, 30, 26);
    let cc_bg = if subtitles_active { Color::rgba(34, 197, 94, 60) } else { Color::rgba(255, 255, 255, 16) };
    let cc_fg = if subtitles_active { Color::rgb(74, 222, 128) } else { Color::rgb(148, 163, 184) };
    fill_rounded_rect(surface, surface_w, surface_h, cc_rect, 4, cc_bg);
    draw_text(surface, surface_w, surface_h, right_x + 7, ctrl_y + 11, "CC", cc_fg, 1);

    // Audio Visualizer toggle [≈]
    right_x -= 42;
    let viz_rect = Rect::new(right_x, ctrl_y + 4, 36, 26);
    let viz_bg = if viz_active { Color::rgba(168, 85, 247, 60) } else { Color::rgba(255, 255, 255, 16) };
    let viz_fg = if viz_active { Color::rgb(192, 132, 252) } else { Color::rgb(148, 163, 184) };
    fill_rounded_rect(surface, surface_w, surface_h, viz_rect, 4, viz_bg);
    draw_text(surface, surface_w, surface_h, right_x + 6, ctrl_y + 11, "VIZ", viz_fg, 1);

    // Aspect Ratio button
    let aspect_name = aspect.name();
    let aspect_w = measure_text(aspect_name, 1) + 12;
    right_x -= aspect_w + 6;
    let aspect_rect = Rect::new(right_x, ctrl_y + 4, aspect_w, 26);
    fill_rounded_rect(surface, surface_w, surface_h, aspect_rect, 4, Color::rgba(255, 255, 255, 16));
    draw_text(surface, surface_w, surface_h, right_x + 6, ctrl_y + 11, aspect_name, Color::WHITE, 1);

    // Speed button [1.0x]
    let speed_text = format!("{:.2}x", speed).replace(".00", ".0");
    let speed_w = measure_text(&speed_text, 1) + 12;
    right_x -= speed_w + 6;
    let speed_rect = Rect::new(right_x, ctrl_y + 4, speed_w, 26);
    fill_rounded_rect(surface, surface_w, surface_h, speed_rect, 4, Color::rgba(255, 255, 255, 16));
    draw_text(surface, surface_w, surface_h, right_x + 6, ctrl_y + 11, &speed_text, Color::WHITE, 1);

    // Volume Slider & Mute
    let vol_w = 64;
    right_x -= vol_w + 44;
    let vol_mute_rect = Rect::new(right_x, ctrl_y + 4, 24, 26);
    fill_rounded_rect(surface, surface_w, surface_h, vol_mute_rect, 4, Color::rgba(255, 255, 255, 16));
    let spk_glyph = if is_muted || volume == 0.0 { "x" } else { ")" };
    draw_text(surface, surface_w, surface_h, right_x + 8, ctrl_y + 11, spk_glyph, Color::WHITE, 1);

    let vol_slider_rect = Rect::new(right_x + 28, ctrl_y + 14, vol_w, 6);
    fill_rounded_rect(surface, surface_w, surface_h, vol_slider_rect, 3, Color::rgba(51, 65, 85, 200));

    let effective_vol = if is_muted { 0.0 } else { volume };
    let vol_pct = (effective_vol / 1.25).clamp(0.0, 1.0);
    let vol_fill_w = (vol_w as f32 * vol_pct) as i32;
    let vol_fill_col = if volume > 1.0 {
        Color::rgb(245, 158, 11) // Amber boost (100%..125%)
    } else {
        Color::rgb(56, 189, 248) // Normal cyan
    };
    if vol_fill_w > 0 {
        fill_rounded_rect(surface, surface_w, surface_h, Rect::new(vol_slider_rect.x, vol_slider_rect.y, vol_fill_w, 6), 3, vol_fill_col);
    }

    BottomBarActionAreas {
        scrub_bar: scrub_rect,
        play_pause_btn: play_rect,
        prev_btn: prev_rect,
        next_btn: next_rect,
        step_back_btn: back_rect,
        step_fwd_btn: fwd_rect,
        stop_btn: stop_rect,
        volume_mute_btn: vol_mute_rect,
        volume_slider: vol_slider_rect,
        speed_btn: speed_rect,
        aspect_btn: aspect_rect,
        viz_btn: viz_rect,
        cc_btn: cc_rect,
    }
}

fn format_time(ms: u64) -> String {
    let total_secs = ms / 1000;
    let s = total_secs % 60;
    let m = (total_secs / 60) % 60;
    let h = total_secs / 3600;
    if h > 0 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}
