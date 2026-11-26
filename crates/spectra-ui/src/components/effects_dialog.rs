//! Interactive VLC-style Effects, 10-Band Audio Equalizer, Video Filters & Track Sync dialog.

use spectra_core::codec::audio_dsp::{EQ_BANDS, EQ_FREQUENCIES};
use spectra_core::engine::MediaPlayerEngine;
use spectra_core::types::EqualizerPreset;
use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectsTab {
    AudioEq,
    VideoEffects,
    TrackSync,
}

pub struct EffectsDialogAreas {
    pub dialog_rect: Rect,
    pub close_btn: Rect,
    pub tab_audio: Rect,
    pub tab_video: Rect,
    pub tab_sync: Rect,
    // Audio EQ
    pub eq_enable_btn: Rect,
    pub eq_reset_btn: Rect,
    pub eq_preset_btns: Vec<(EqualizerPreset, Rect)>,
    pub eq_band_sliders: [Rect; EQ_BANDS],
    // Video
    pub vid_brightness_bar: Rect,
    pub vid_contrast_bar: Rect,
    pub vid_saturation_bar: Rect,
    pub vid_gamma_bar: Rect,
    pub vid_grayscale_btn: Rect,
    pub vid_invert_btn: Rect,
    pub vid_deinterlace_btn: Rect,
    pub vid_flip_btn: Rect,
    pub vid_rotate_btn: Rect,
    pub vid_reset_btn: Rect,
    // Sync
    pub aud_delay_minus: Rect,
    pub aud_delay_plus: Rect,
    pub sub_delay_minus: Rect,
    pub sub_delay_plus: Rect,
    pub sub_scale_btns: Vec<(f32, Rect)>,
}

pub fn render_effects_dialog(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    tab: EffectsTab,
    engine: &MediaPlayerEngine,
) -> EffectsDialogAreas {
    let dw = 540.min(surface_w - 40);
    let dh = 360.min(surface_h - 80);
    let dx = (surface_w - dw) / 2;
    let dy = (surface_h - dh) / 2;
    let dialog_rect = Rect::new(dx, dy, dw, dh);

    fill_rounded_rect(surface, surface_w, surface_h, dialog_rect, 10, Color::rgba(15, 23, 42, 248));
    draw_rect_outline(surface, surface_w, surface_h, dialog_rect, 1, Color::rgb(56, 189, 248));

    // Header
    fill_rounded_rect(surface, surface_w, surface_h, Rect::new(dx, dy, dw, 34), 10, Color::rgba(30, 41, 59, 235));
    draw_text(surface, surface_w, surface_h, dx + 14, dy + 11, "ADJUSTMENTS & EFFECTS STUDIO", Color::rgb(56, 189, 248), 1);

    let close_btn = Rect::new(dx + dw - 28, dy + 7, 20, 20);
    fill_rounded_rect(surface, surface_w, surface_h, close_btn, 4, Color::rgba(255, 255, 255, 20));
    draw_text(surface, surface_w, surface_h, close_btn.x + 6, close_btn.y + 5, "X", Color::WHITE, 1);

    // Tabs
    let ty = dy + 42;
    let tw = (dw - 36) / 3;
    let tab_audio = Rect::new(dx + 12, ty, tw, 26);
    let tab_video = Rect::new(tab_audio.right() + 6, ty, tw, 26);
    let tab_sync = Rect::new(tab_video.right() + 6, ty, tw, 26);

    for (r, lbl, active) in [
        (tab_audio, "Audio 10-Band EQ", tab == EffectsTab::AudioEq),
        (tab_video, "Video Effects", tab == EffectsTab::VideoEffects),
        (tab_sync, "Synchronization", tab == EffectsTab::TrackSync),
    ] {
        let bg = if active { Color::rgb(14, 165, 233) } else { Color::rgba(255, 255, 255, 16) };
        fill_rounded_rect(surface, surface_w, surface_h, r, 5, bg);
        draw_text(surface, surface_w, surface_h, r.x + 14, r.y + 8, lbl, Color::WHITE, 1);
    }

    let mut eq_enable_btn = Rect::default();
    let mut eq_reset_btn = Rect::default();
    let mut eq_preset_btns = Vec::new();
    let mut eq_band_sliders = [Rect::default(); EQ_BANDS];

    let mut vid_brightness_bar = Rect::default();
    let mut vid_contrast_bar = Rect::default();
    let mut vid_saturation_bar = Rect::default();
    let mut vid_gamma_bar = Rect::default();
    let mut vid_grayscale_btn = Rect::default();
    let mut vid_invert_btn = Rect::default();
    let mut vid_deinterlace_btn = Rect::default();
    let mut vid_flip_btn = Rect::default();
    let mut vid_rotate_btn = Rect::default();
    let mut vid_reset_btn = Rect::default();

    let mut aud_delay_minus = Rect::default();
    let mut aud_delay_plus = Rect::default();
    let mut sub_delay_minus = Rect::default();
    let mut sub_delay_plus = Rect::default();
    let mut sub_scale_btns = Vec::new();

    let content_y = ty + 36;
    match tab {
        EffectsTab::AudioEq => {
            eq_enable_btn = Rect::new(dx + 14, content_y, 96, 22);
            let en_bg = if engine.audio.eq_enabled { Color::rgb(16, 185, 129) } else { Color::rgba(255, 255, 255, 18) };
            fill_rounded_rect(surface, surface_w, surface_h, eq_enable_btn, 4, en_bg);
            draw_text(surface, surface_w, surface_h, eq_enable_btn.x + 12, eq_enable_btn.y + 6, if engine.audio.eq_enabled { "EQ: ON" } else { "EQ: OFF" }, Color::WHITE, 1);

            eq_reset_btn = Rect::new(dx + dw - 86, content_y, 72, 22);
            fill_rounded_rect(surface, surface_w, surface_h, eq_reset_btn, 4, Color::rgba(239, 68, 68, 45));
            draw_text(surface, surface_w, surface_h, eq_reset_btn.x + 14, eq_reset_btn.y + 6, "Reset", Color::WHITE, 1);

            // Preset pills
            let presets = [
                (EqualizerPreset::Flat, "Flat"),
                (EqualizerPreset::Rock, "Rock"),
                (EqualizerPreset::Pop, "Pop"),
                (EqualizerPreset::BassBoost, "Bass"),
                (EqualizerPreset::Vocal, "Vocal"),
                (EqualizerPreset::Techno, "EDM"),
            ];
            let mut px = dx + 120;
            for (preset, name) in presets {
                let r = Rect::new(px, content_y, 50, 22);
                let active = engine.audio.equalizer == preset;
                fill_rounded_rect(surface, surface_w, surface_h, r, 4, if active { Color::rgb(56, 189, 248) } else { Color::rgba(255, 255, 255, 16) });
                draw_text(surface, surface_w, surface_h, r.x + 8, r.y + 6, name, if active { Color::rgb(15, 23, 42) } else { Color::WHITE }, 1);
                eq_preset_btns.push((preset, r));
                px += 54;
            }

            // 10 vertical EQ sliders
            let slider_top = content_y + 38;
            let slider_h = 175;
            let col_w = (dw - 36) / EQ_BANDS as i32;
            for b in 0..EQ_BANDS {
                let cx = dx + 18 + (b as i32) * col_w + col_w / 2;
                let track_rect = Rect::new(cx - 12, slider_top, 24, slider_h);
                eq_band_sliders[b] = track_rect;

                // Center 0 dB line
                let mid_y = slider_top + slider_h / 2;
                fill_rect(surface, surface_w, surface_h, Rect::new(cx - 8, mid_y, 16, 1), Color::rgba(255, 255, 255, 45));
                // Rail
                fill_rounded_rect(surface, surface_w, surface_h, Rect::new(cx - 3, slider_top, 6, slider_h), 3, Color::rgb(48, 54, 61));

                let gain = engine.audio.eq_bands_db[b].clamp(-12.0, 12.0);
                let norm = (gain + 12.0) / 24.0; // 0 = -12dB (bottom), 1 = +12dB (top)
                let knob_y = slider_top + ((1.0 - norm) * (slider_h - 10) as f32) as i32;
                let fill_top = knob_y.min(mid_y);
                let fill_h = (knob_y - mid_y).abs().max(2);
                fill_rect(surface, surface_w, surface_h, Rect::new(cx - 3, fill_top, 6, fill_h), Color::rgb(56, 189, 248));
                fill_rounded_rect(surface, surface_w, surface_h, Rect::new(cx - 8, knob_y, 16, 10), 3, Color::WHITE);

                let freq_lbl = if EQ_FREQUENCIES[b] >= 1000.0 {
                    format!("{}k", (EQ_FREQUENCIES[b] / 1000.0) as i32)
                } else {
                    format!("{}", EQ_FREQUENCIES[b] as i32)
                };
                draw_text(surface, surface_w, surface_h, cx - 10, slider_top + slider_h + 8, &freq_lbl, Color::rgb(148, 163, 184), 1);
                let db_lbl = format!("{:+.0}", gain);
                draw_text(surface, surface_w, surface_h, cx - 10, slider_top - 14, &db_lbl, Color::rgb(226, 232, 240), 1);
            }
        }
        EffectsTab::VideoEffects => {
            let adj = &engine.video_synth.adjustments;
            let bar_x = dx + 140;
            let bar_w = dw - 210;

            vid_brightness_bar = draw_slider_row(surface, surface_w, surface_h, dx + 16, content_y + 8, bar_x, bar_w, "Brightness:", (adj.brightness as f32 + 100.0) / 200.0, &format!("{}", adj.brightness));
            vid_contrast_bar = draw_slider_row(surface, surface_w, surface_h, dx + 16, content_y + 44, bar_x, bar_w, "Contrast:", (adj.contrast - 0.5) / 1.5, &format!("{:.2}x", adj.contrast));
            vid_saturation_bar = draw_slider_row(surface, surface_w, surface_h, dx + 16, content_y + 80, bar_x, bar_w, "Saturation:", adj.saturation / 2.0, &format!("{:.2}x", adj.saturation));
            vid_gamma_bar = draw_slider_row(surface, surface_w, surface_h, dx + 16, content_y + 116, bar_x, bar_w, "Gamma:", (adj.gamma - 0.5) / 2.0, &format!("{:.2}", adj.gamma));

            let row1_y = content_y + 162;
            vid_grayscale_btn = draw_toggle_pill(surface, surface_w, surface_h, dx + 16, row1_y, 115, "Grayscale", adj.grayscale);
            vid_invert_btn = draw_toggle_pill(surface, surface_w, surface_h, dx + 140, row1_y, 115, "Invert", adj.invert);
            vid_deinterlace_btn = draw_toggle_pill(surface, surface_w, surface_h, dx + 264, row1_y, 125, "Deinterlace", adj.deinterlace);

            let row2_y = row1_y + 36;
            vid_flip_btn = draw_toggle_pill(surface, surface_w, surface_h, dx + 16, row2_y, 115, "Flip Horiz", adj.flip_h);
            vid_rotate_btn = draw_toggle_pill(surface, surface_w, surface_h, dx + 140, row2_y, 145, &format!("Rotate ({})", adj.rotate_deg), adj.rotate_deg != 0);

            vid_reset_btn = Rect::new(dx + dw - 120, row2_y, 102, 26);
            fill_rounded_rect(surface, surface_w, surface_h, vid_reset_btn, 4, Color::rgba(239, 68, 68, 55));
            draw_text(surface, surface_w, surface_h, vid_reset_btn.x + 14, vid_reset_btn.y + 8, "Reset Video", Color::WHITE, 1);
        }
        EffectsTab::TrackSync => {
            let r1_y = content_y + 18;
            draw_text(surface, surface_w, surface_h, dx + 18, r1_y + 6, "Audio Track Delay:", Color::rgb(226, 232, 240), 1);
            aud_delay_minus = Rect::new(dx + 190, r1_y, 56, 26);
            aud_delay_plus = Rect::new(dx + 340, r1_y, 56, 26);
            fill_rounded_rect(surface, surface_w, surface_h, aud_delay_minus, 4, Color::rgba(255, 255, 255, 20));
            fill_rounded_rect(surface, surface_w, surface_h, aud_delay_plus, 4, Color::rgba(255, 255, 255, 20));
            draw_text(surface, surface_w, surface_h, aud_delay_minus.x + 10, aud_delay_minus.y + 8, "-50ms", Color::WHITE, 1);
            draw_text(surface, surface_w, surface_h, aud_delay_plus.x + 10, aud_delay_plus.y + 8, "+50ms", Color::WHITE, 1);
            draw_text(surface, surface_w, surface_h, dx + 260, r1_y + 8, &format!("{} ms", engine.audio.audio_delay_ms), Color::rgb(56, 189, 248), 1);

            let r2_y = content_y + 68;
            draw_text(surface, surface_w, surface_h, dx + 18, r2_y + 6, "Subtitle Delay:", Color::rgb(226, 232, 240), 1);
            sub_delay_minus = Rect::new(dx + 190, r2_y, 56, 26);
            sub_delay_plus = Rect::new(dx + 340, r2_y, 56, 26);
            fill_rounded_rect(surface, surface_w, surface_h, sub_delay_minus, 4, Color::rgba(255, 255, 255, 20));
            fill_rounded_rect(surface, surface_w, surface_h, sub_delay_plus, 4, Color::rgba(255, 255, 255, 20));
            draw_text(surface, surface_w, surface_h, sub_delay_minus.x + 8, sub_delay_minus.y + 8, "-100ms", Color::WHITE, 1);
            draw_text(surface, surface_w, surface_h, sub_delay_plus.x + 8, sub_delay_plus.y + 8, "+100ms", Color::WHITE, 1);
            draw_text(surface, surface_w, surface_h, dx + 260, r2_y + 8, &format!("{} ms", engine.subtitles.delay_ms), Color::rgb(74, 222, 128), 1);

            let r3_y = content_y + 118;
            draw_text(surface, surface_w, surface_h, dx + 18, r3_y + 6, "Subtitle Size:", Color::rgb(226, 232, 240), 1);
            let mut sx = dx + 190;
            for (scale, lbl) in [(0.85_f32, "Small"), (1.0_f32, "Normal"), (1.35_f32, "Large"), (1.75_f32, "XL")] {
                let r = Rect::new(sx, r3_y, 62, 26);
                let active = (engine.subtitles.font_scale - scale).abs() < 0.1;
                fill_rounded_rect(surface, surface_w, surface_h, r, 4, if active { Color::rgb(56, 189, 248) } else { Color::rgba(255, 255, 255, 20) });
                draw_text(surface, surface_w, surface_h, r.x + 12, r.y + 8, lbl, if active { Color::rgb(15, 23, 42) } else { Color::WHITE }, 1);
                sub_scale_btns.push((scale, r));
                sx += 70;
            }
        }
    }

    EffectsDialogAreas {
        dialog_rect,
        close_btn,
        tab_audio,
        tab_video,
        tab_sync,
        eq_enable_btn,
        eq_reset_btn,
        eq_preset_btns,
        eq_band_sliders,
        vid_brightness_bar,
        vid_contrast_bar,
        vid_saturation_bar,
        vid_gamma_bar,
        vid_grayscale_btn,
        vid_invert_btn,
        vid_deinterlace_btn,
        vid_flip_btn,
        vid_rotate_btn,
        vid_reset_btn,
        aud_delay_minus,
        aud_delay_plus,
        sub_delay_minus,
        sub_delay_plus,
        sub_scale_btns,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_slider_row(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    lx: i32,
    y: i32,
    bx: i32,
    bw: i32,
    label: &str,
    frac: f32,
    val_str: &str,
) -> Rect {
    draw_text(surface, sw, sh, lx, y + 5, label, Color::rgb(226, 232, 240), 1);
    let track = Rect::new(bx, y + 6, bw, 8);
    let hit = Rect::new(bx, y, bw, 22);
    fill_rounded_rect(surface, sw, sh, track, 4, Color::rgb(48, 54, 61));
    let fw = (bw as f32 * frac.clamp(0.0, 1.0)) as i32;
    if fw > 0 {
        fill_rounded_rect(surface, sw, sh, Rect::new(bx, y + 6, fw, 8), 4, Color::rgb(56, 189, 248));
    }
    fill_rounded_rect(surface, sw, sh, Rect::new(bx + fw - 4, y + 3, 9, 14), 3, Color::WHITE);
    draw_text(surface, sw, sh, bx + bw + 12, y + 5, val_str, Color::WHITE, 1);
    hit
}

fn draw_toggle_pill(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    x: i32,
    y: i32,
    w: i32,
    label: &str,
    active: bool,
) -> Rect {
    let r = Rect::new(x, y, w, 26);
    let bg = if active { Color::rgb(14, 165, 233) } else { Color::rgba(255, 255, 255, 18) };
    fill_rounded_rect(surface, sw, sh, r, 5, bg);
    draw_text(surface, sw, sh, x + 12, y + 8, label, Color::WHITE, 1);
    r
}
