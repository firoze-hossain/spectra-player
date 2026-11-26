//! Bottom player control toolbar: interactive timeline scrub bar with hover tooltip,
//! A-B loop markers, transport buttons, 200% volume wedge slider, speed, aspect, and CC toggle.

use spectra_core::playlist::LoopMode;
use spectra_core::types::{AspectRatio, PlaybackState};
use crate::draw::{draw_circle, draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy, Default)]
pub struct BottomBarActionAreas {
    pub bar_rect: Rect,
    pub scrub_bar: Rect,
    pub play_pause_btn: Rect,
    pub stop_btn: Rect,
    pub prev_btn: Rect,
    pub next_btn: Rect,
    pub fullscreen_btn: Rect,
    pub effects_eq_btn: Rect,
    pub playlist_btn: Rect,
    pub loop_btn: Rect,
    pub shuffle_btn: Rect,
    pub step_back_btn: Rect,
    pub step_fwd_btn: Rect,
    pub volume_mute_btn: Rect,
    pub volume_slider: Rect,
    pub speed_btn: Rect,
    pub aspect_btn: Rect,
    pub viz_btn: Rect,
    pub cc_btn: Rect,
    pub snap_btn: Rect,
}

#[allow(clippy::too_many_arguments)]
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
    viz_or_eq_active: bool,
    subtitles_active: bool,
    loop_mode: LoopMode,
    is_shuffling: bool,
    ab_loop: Option<(u64, Option<u64>)>,
    hover_pos: Option<(i32, i32)>,
) -> BottomBarActionAreas {
    let bar_h = 72;
    let margin = 8;
    let bar_w = surface_w - margin * 2;
    let bar_y = surface_h - bar_h - margin;
    let bar_rect = Rect::new(margin, bar_y, bar_w, bar_h);

    // 1. Frosted acrylic card background
    fill_rounded_rect(surface, surface_w, surface_h, Rect::new(margin + 2, bar_y + 3, bar_w, bar_h), 10, Color::rgba(0, 0, 0, 75));
    fill_rounded_rect(surface, surface_w, surface_h, bar_rect, 8, Color::rgba(17, 24, 39, 238));
    draw_rect_outline(surface, surface_w, surface_h, bar_rect, 1, Color::rgb(48, 54, 61));

    // 2. Timeline Row
    let timeline_y = bar_y + 11;
    let time_margin_x = 12;

    let cur_time_str = if state == PlaybackState::Stopped {
        "--:--".to_string()
    } else {
        format_time(position_ms)
    };
    let dur_time_str = if duration_ms == 0 {
        "--:--".to_string()
    } else {
        format_time(duration_ms)
    };

    draw_text(
        surface,
        surface_w,
        surface_h,
        margin + time_margin_x,
        timeline_y - 2,
        &cur_time_str,
        Color::rgb(226, 232, 240),
        1,
    );

    let time_w = 50;
    let scrub_x = margin + time_margin_x + time_w + 8;
    let scrub_w = (bar_w - (time_margin_x * 2 + time_w * 2 + 16)).max(40);
    let scrub_h = 6;
    let scrub_rect = Rect::new(scrub_x, timeline_y, scrub_w, scrub_h);
    let scrub_hit_rect = Rect::new(scrub_x, timeline_y - 6, scrub_w, 18);

    // Track background
    fill_rounded_rect(surface, surface_w, surface_h, scrub_rect, 3, Color::rgb(48, 54, 61));

    if duration_ms > 0 {
        let buf_progress = ((position_ms + 12_000) as f32 / duration_ms as f32).clamp(0.0, 1.0);
        let buf_w = (scrub_w as f32 * buf_progress) as i32;
        fill_rounded_rect(
            surface,
            surface_w,
            surface_h,
            Rect::new(scrub_rect.x, timeline_y, buf_w, scrub_h),
            3,
            Color::rgb(71, 85, 105),
        );

        let play_progress = (position_ms as f32 / duration_ms as f32).clamp(0.0, 1.0);
        let play_w = (scrub_w as f32 * play_progress) as i32;
        if play_w > 0 {
            fill_rounded_rect(
                surface,
                surface_w,
                surface_h,
                Rect::new(scrub_rect.x, timeline_y, play_w, scrub_h),
                3,
                Color::rgb(56, 189, 248),
            );
        }

        // A-B Loop markers
        if let Some((a_ms, b_opt)) = ab_loop {
            let ax = scrub_rect.x + (((a_ms as f32 / duration_ms as f32).clamp(0.0, 1.0)) * scrub_w as f32) as i32;
            fill_rect(surface, surface_w, surface_h, Rect::new(ax, timeline_y - 3, 2, 12), Color::rgb(74, 222, 128));
            if let Some(b_ms) = b_opt {
                let bx = scrub_rect.x + (((b_ms as f32 / duration_ms as f32).clamp(0.0, 1.0)) * scrub_w as f32) as i32;
                fill_rect(surface, surface_w, surface_h, Rect::new(bx, timeline_y - 3, 2, 12), Color::rgb(245, 158, 11));
            }
        }

        // Knob thumb
        let knob_x = scrub_rect.x + play_w;
        let knob_y = timeline_y + scrub_h / 2;
        draw_circle(surface, surface_w, surface_h, knob_x, knob_y, 6, Color::WHITE);
        draw_circle(surface, surface_w, surface_h, knob_x, knob_y, 3, Color::rgb(2, 132, 199));

        // Hover timestamp preview tooltip above scrub bar
        if let Some((hx, hy)) = hover_pos {
            if scrub_hit_rect.contains(hx, hy) {
                let rel = ((hx - scrub_rect.x) as f32 / scrub_w as f32).clamp(0.0, 1.0);
                let hover_ms = (rel * duration_ms as f32) as u64;
                let tip = format_time(hover_ms);
                let tip_w = measure_text(&tip, 1) + 12;
                let tip_x = (hx - tip_w / 2).clamp(margin, surface_w - tip_w - margin);
                let tip_rect = Rect::new(tip_x, bar_y - 22, tip_w, 18);
                fill_rounded_rect(surface, surface_w, surface_h, tip_rect, 4, Color::rgb(15, 23, 42));
                draw_rect_outline(surface, surface_w, surface_h, tip_rect, 1, Color::rgb(56, 189, 248));
                draw_text(surface, surface_w, surface_h, tip_x + 6, bar_y - 18, &tip, Color::WHITE, 1);
            }
        }
    }

    draw_text(
        surface,
        surface_w,
        surface_h,
        scrub_rect.right() + 8,
        timeline_y - 2,
        &dur_time_str,
        Color::rgb(148, 163, 184),
        1,
    );

    // 3. Transport Controls Row
    let ctrl_y = bar_y + 27;
    let mut cur_x = margin + 12;

    let play_rect = Rect::new(cur_x, ctrl_y + 2, 32, 28);
    fill_rounded_rect(surface, surface_w, surface_h, play_rect, 4, Color::rgb(56, 189, 248));
    if state == PlaybackState::Playing {
        fill_rect(surface, surface_w, surface_h, Rect::new(cur_x + 10, ctrl_y + 8, 4, 16), Color::rgb(15, 23, 42));
        fill_rect(surface, surface_w, surface_h, Rect::new(cur_x + 18, ctrl_y + 8, 4, 16), Color::rgb(15, 23, 42));
    } else {
        draw_text(surface, surface_w, surface_h, cur_x + 12, ctrl_y + 7, ">", Color::rgb(15, 23, 42), 2);
    }
    cur_x += 36;

    let stop_rect = Rect::new(cur_x, ctrl_y + 3, 26, 26);
    fill_rounded_rect(surface, surface_w, surface_h, stop_rect, 4, Color::rgba(255, 255, 255, 14));
    fill_rect(surface, surface_w, surface_h, Rect::new(cur_x + 8, ctrl_y + 11, 10, 10), Color::rgb(239, 68, 68));
    cur_x += 30;

    let prev_rect = Rect::new(cur_x, ctrl_y + 3, 26, 26);
    fill_rounded_rect(surface, surface_w, surface_h, prev_rect, 4, Color::rgba(255, 255, 255, 14));
    draw_text(surface, surface_w, surface_h, cur_x + 6, ctrl_y + 9, "|<", Color::WHITE, 1);
    cur_x += 30;

    let next_rect = Rect::new(cur_x, ctrl_y + 3, 26, 26);
    fill_rounded_rect(surface, surface_w, surface_h, next_rect, 4, Color::rgba(255, 255, 255, 14));
    draw_text(surface, surface_w, surface_h, cur_x + 6, ctrl_y + 9, ">|", Color::WHITE, 1);
    cur_x += 30;

    let fs_rect = Rect::new(cur_x, ctrl_y + 3, 28, 26);
    fill_rounded_rect(surface, surface_w, surface_h, fs_rect, 4, Color::rgba(255, 255, 255, 14));
    draw_text(surface, surface_w, surface_h, cur_x + 5, ctrl_y + 9, "[ ]", Color::WHITE, 1);
    cur_x += 32;

    let eq_rect = Rect::new(cur_x, ctrl_y + 3, 28, 26);
    let eq_bg = if viz_or_eq_active { Color::rgb(14, 165, 233) } else { Color::rgba(255, 255, 255, 14) };
    fill_rounded_rect(surface, surface_w, surface_h, eq_rect, 4, eq_bg);
    draw_text(surface, surface_w, surface_h, cur_x + 7, ctrl_y + 9, "EQ", Color::WHITE, 1);
    cur_x += 32;

    let pl_rect = Rect::new(cur_x, ctrl_y + 3, 28, 26);
    fill_rounded_rect(surface, surface_w, surface_h, pl_rect, 4, Color::rgba(255, 255, 255, 14));
    draw_text(surface, surface_w, surface_h, cur_x + 7, ctrl_y + 9, "PL", Color::WHITE, 1);
    cur_x += 32;

    let is_looping = loop_mode != LoopMode::None;
    let loop_rect = Rect::new(cur_x, ctrl_y + 3, 30, 26);
    let loop_bg = if is_looping { Color::rgb(56, 189, 248) } else { Color::rgba(255, 255, 255, 14) };
    let loop_fg = if is_looping { Color::rgb(15, 23, 42) } else { Color::WHITE };
    fill_rounded_rect(surface, surface_w, surface_h, loop_rect, 4, loop_bg);
    draw_text(surface, surface_w, surface_h, cur_x + 5, ctrl_y + 9, loop_mode.badge(), loop_fg, 1);
    cur_x += 34;

    let shuf_rect = Rect::new(cur_x, ctrl_y + 3, 30, 26);
    let shuf_bg = if is_shuffling { Color::rgb(56, 189, 248) } else { Color::rgba(255, 255, 255, 14) };
    let shuf_fg = if is_shuffling { Color::rgb(15, 23, 42) } else { Color::WHITE };
    fill_rounded_rect(surface, surface_w, surface_h, shuf_rect, 4, shuf_bg);
    draw_text(surface, surface_w, surface_h, cur_x + 5, ctrl_y + 9, "RND", shuf_fg, 1);
    cur_x += 34;

    let back_rect = Rect::new(cur_x, ctrl_y + 3, 28, 26);
    fill_rounded_rect(surface, surface_w, surface_h, back_rect, 4, Color::rgba(255, 255, 255, 14));
    draw_text(surface, surface_w, surface_h, cur_x + 4, ctrl_y + 9, "-10", Color::WHITE, 1);
    cur_x += 32;

    let fwd_rect = Rect::new(cur_x, ctrl_y + 3, 28, 26);
    fill_rounded_rect(surface, surface_w, surface_h, fwd_rect, 4, Color::rgba(255, 255, 255, 14));
    draw_text(surface, surface_w, surface_h, cur_x + 4, ctrl_y + 9, "+10", Color::WHITE, 1);

    // 4. Right Toolbar Items: Snapshot, CC, Aspect, Speed, Volume Wedge (0..200%)
    let mut right_x = bar_rect.right() - 12;

    right_x -= 30;
    let snap_rect = Rect::new(right_x, ctrl_y + 3, 28, 26);
    fill_rounded_rect(surface, surface_w, surface_h, snap_rect, 4, Color::rgba(255, 255, 255, 14));
    draw_text(surface, surface_w, surface_h, right_x + 3, ctrl_y + 9, "CAM", Color::rgb(203, 213, 225), 1);

    right_x -= 32;
    let cc_rect = Rect::new(right_x, ctrl_y + 3, 28, 26);
    let cc_bg = if subtitles_active { Color::rgba(34, 197, 94, 60) } else { Color::rgba(255, 255, 255, 14) };
    let cc_fg = if subtitles_active { Color::rgb(74, 222, 128) } else { Color::WHITE };
    fill_rounded_rect(surface, surface_w, surface_h, cc_rect, 4, cc_bg);
    draw_text(surface, surface_w, surface_h, right_x + 6, ctrl_y + 9, "CC", cc_fg, 1);

    let aspect_name = aspect.name();
    let aspect_w = measure_text(aspect_name, 1) + 12;
    right_x -= aspect_w + 5;
    let aspect_rect = Rect::new(right_x, ctrl_y + 3, aspect_w, 26);
    fill_rounded_rect(surface, surface_w, surface_h, aspect_rect, 4, Color::rgba(255, 255, 255, 14));
    draw_text(surface, surface_w, surface_h, right_x + 6, ctrl_y + 9, aspect_name, Color::WHITE, 1);

    let speed_text = format!("{:.2}x", speed).replace(".00x", ".0x");
    let speed_w = measure_text(&speed_text, 1) + 10;
    right_x -= speed_w + 5;
    let speed_rect = Rect::new(right_x, ctrl_y + 3, speed_w, 26);
    let spd_bg = if (speed - 1.0).abs() > 0.05 { Color::rgba(245, 158, 11, 50) } else { Color::rgba(255, 255, 255, 14) };
    fill_rounded_rect(surface, surface_w, surface_h, speed_rect, 4, spd_bg);
    draw_text(surface, surface_w, surface_h, right_x + 5, ctrl_y + 9, &speed_text, Color::WHITE, 1);

    let effective_vol = if is_muted { 0.0 } else { volume };
    let vol_int = (effective_vol * 100.0).round() as i32;
    let vol_str = format!("{}%", vol_int);
    let vol_str_w = measure_text(&vol_str, 1);
    right_x -= vol_str_w + 6;
    let vol_text_col = if effective_vol > 1.0 {
        Color::rgb(245, 158, 11)
    } else {
        Color::rgb(226, 232, 240)
    };
    draw_text(surface, surface_w, surface_h, right_x, ctrl_y + 9, &vol_str, vol_text_col, 1);

    let vol_w = 56;
    right_x -= vol_w + 6;
    let vol_slider_rect = Rect::new(right_x, ctrl_y + 4, vol_w, 24);

    for vx in 0..vol_w {
        let h_at_x = 2 + (vx * 16) / vol_w;
        let vy = vol_slider_rect.bottom() - h_at_x;
        fill_rect(surface, surface_w, surface_h, Rect::new(right_x + vx, vy, 1, h_at_x), Color::rgb(48, 54, 61));
    }

    let vol_frac = (effective_vol / 2.0).clamp(0.0, 1.0);
    let fill_w = (vol_w as f32 * vol_frac) as i32;
    for vx in 0..fill_w {
        let h_at_x = 2 + (vx * 16) / vol_w;
        let vy = vol_slider_rect.bottom() - h_at_x;
        let col = if vx > vol_w / 2 {
            Color::rgb(245, 158, 11) // >100% boost zone
        } else {
            Color::rgb(56, 189, 248)
        };
        fill_rect(surface, surface_w, surface_h, Rect::new(right_x + vx, vy, 1, h_at_x), col);
    }

    right_x -= 28;
    let vol_mute_rect = Rect::new(right_x, ctrl_y + 3, 24, 26);
    fill_rounded_rect(surface, surface_w, surface_h, vol_mute_rect, 4, Color::rgba(255, 255, 255, 14));
    let spk_glyph = if is_muted || volume == 0.0 { "x" } else { ")" };
    draw_text(surface, surface_w, surface_h, right_x + 8, ctrl_y + 9, spk_glyph, Color::WHITE, 1);

    BottomBarActionAreas {
        bar_rect,
        scrub_bar: scrub_hit_rect,
        play_pause_btn: play_rect,
        stop_btn: stop_rect,
        prev_btn: prev_rect,
        next_btn: next_rect,
        fullscreen_btn: fs_rect,
        effects_eq_btn: eq_rect,
        playlist_btn: pl_rect,
        loop_btn: loop_rect,
        shuffle_btn: shuf_rect,
        volume_mute_btn: vol_mute_rect,
        volume_slider: vol_slider_rect,
        step_back_btn: back_rect,
        step_fwd_btn: fwd_rect,
        speed_btn: speed_rect,
        aspect_btn: aspect_rect,
        viz_btn: eq_rect,
        cc_btn: cc_rect,
        snap_btn: snap_rect,
    }
}

pub fn format_time(ms: u64) -> String {
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
