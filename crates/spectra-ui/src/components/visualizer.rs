//! Real-time Audio Visualizers:
//! - 32-Band FFT Spectrum Equalizer with peak-hold caps and glass floor reflection
//! - Oscilloscope Waveform
//! - Vinyl Turntable / Embedded Album Cover Art Studio Mode

use spectra_core::codec::audio_dsp::{SPECTRUM_BANDS, WAVEFORM_SAMPLES};
use spectra_core::types::{AudioVisualizerMode, MediaMetadata, VideoFrame};
use crate::draw::{draw_circle, draw_line, draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};
use crate::scaler::blit_video_frame;

#[allow(clippy::too_many_arguments)]
pub fn render_audio_visualizer(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    spectrum_levels: &[f32; SPECTRUM_BANDS],
    spectrum_peaks: &[f32; SPECTRUM_BANDS],
    waveform: &[f32; WAVEFORM_SAMPLES],
    metadata: &MediaMetadata,
    time_ms: u64,
    is_audio_only: bool,
    mode: AudioVisualizerMode,
) {
    if is_audio_only {
        // Dark studio backdrop
        fill_rect(surface, surface_w, surface_h, Rect::new(0, 0, surface_w, surface_h), Color::rgb(11, 15, 25));

        let cx = surface_w / 2;
        let cy = surface_h / 2 - 48;
        let vinyl_r = (surface_h / 6).clamp(62, 118);

        if let Some(ref art) = metadata.cover_art {
            // Render embedded album cover art framed in a sleek sleeve
            let art_sz = (vinyl_r * 2).min(220);
            let art_rect = Rect::new(cx - art_sz / 2, cy - art_sz / 2, art_sz, art_sz);
            let vf = VideoFrame {
                width: art.width,
                height: art.height,
                timestamp_ms: time_ms,
                buffer: art.pixels.clone(),
            };
            blit_video_frame(surface, surface_w, surface_h, art_rect, &vf);
            draw_rect_outline(surface, surface_w, surface_h, art_rect, 2, Color::rgb(56, 189, 248));
        } else {
            // Spinning vinyl turntable platter
            draw_circle(surface, surface_w, surface_h, cx, cy, vinyl_r, Color::rgb(24, 24, 27));
            draw_circle(surface, surface_w, surface_h, cx, cy, vinyl_r - 6, Color::rgb(39, 39, 42));
            draw_circle(surface, surface_w, surface_h, cx, cy, vinyl_r - 14, Color::rgb(24, 24, 27));
            draw_circle(surface, surface_w, surface_h, cx, cy, vinyl_r - 22, Color::rgb(39, 39, 42));

            let label_r = vinyl_r / 2;
            draw_circle(surface, surface_w, surface_h, cx, cy, label_r, Color::rgb(14, 165, 233));
            draw_circle(surface, surface_w, surface_h, cx, cy, 8, Color::WHITE);

            let angle = time_ms as f32 * 0.003;
            let shine_dx = (angle.cos() * (vinyl_r as f32 * 0.7)) as i32;
            let shine_dy = (angle.sin() * (vinyl_r as f32 * 0.7)) as i32;
            draw_circle(surface, surface_w, surface_h, cx + shine_dx, cy + shine_dy, 12, Color::rgba(255, 255, 255, 70));
        }

        let title_w = measure_text(&metadata.title, 2);
        draw_text(surface, surface_w, surface_h, cx - title_w / 2, cy + vinyl_r + 18, &metadata.title, Color::WHITE, 2);

        let artist_w = measure_text(&metadata.artist, 1);
        draw_text(surface, surface_w, surface_h, cx - artist_w / 2, cy + vinyl_r + 42, &metadata.artist, Color::rgb(148, 163, 184), 1);

        let info_line = format!(
            "{} • {} • {}Hz",
            metadata.audio_codec,
            metadata.format.name(),
            metadata.sample_rate
        );
        let info_w = measure_text(&info_line, 1);
        draw_text(surface, surface_w, surface_h, cx - info_w / 2, cy + vinyl_r + 58, &info_line, Color::rgb(56, 189, 248), 1);
    }

    if mode == AudioVisualizerMode::Off && !is_audio_only {
        return;
    }

    if mode == AudioVisualizerMode::Waveform {
        render_waveform(surface, surface_w, surface_h, waveform, is_audio_only);
        return;
    }

    // 32-Band FFT Spectrum Bars
    let bar_area_w = (surface_w - 80).min(780).max(160);
    let start_x = (surface_w - bar_area_w) / 2;
    let base_y = if is_audio_only { surface_h - 116 } else { surface_h - 108 };
    let max_bar_h = if is_audio_only { 92 } else { 68 };

    let band_gap = 3;
    let band_w = ((bar_area_w - (SPECTRUM_BANDS as i32 - 1) * band_gap) / (SPECTRUM_BANDS as i32)).max(2);

    for i in 0..SPECTRUM_BANDS {
        let x = start_x + (i as i32) * (band_w + band_gap);
        let level = spectrum_levels[i].clamp(0.02, 1.0);
        let bh = (level * max_bar_h as f32) as i32;
        let by = base_y - bh;

        let pct = i as f32 / SPECTRUM_BANDS as f32;
        let r = (40.0 + pct * 215.0) as u8;
        let g = (190.0 - pct * 100.0) as u8;
        let b = (250.0 - pct * 180.0) as u8;
        let bar_col = Color::rgba(r, g, b, 230);

        fill_rounded_rect(surface, surface_w, surface_h, Rect::new(x, by, band_w, bh), 2, bar_col);

        let peak = spectrum_peaks[i].clamp(0.0, 1.0);
        let peak_y = base_y - (peak * max_bar_h as f32) as i32 - 3;
        fill_rect(surface, surface_w, surface_h, Rect::new(x, peak_y, band_w, 2), Color::WHITE);

        let refl_h = (bh / 3).min(20);
        for dy in 0..refl_h {
            let alpha = ((1.0 - dy as f32 / refl_h.max(1) as f32) * 60.0) as u8;
            fill_rect(
                surface,
                surface_w,
                surface_h,
                Rect::new(x, base_y + 2 + dy, band_w, 1),
                Color::rgba(r, g, b, alpha),
            );
        }
    }
}

fn render_waveform(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    waveform: &[f32; WAVEFORM_SAMPLES],
    is_audio_only: bool,
) {
    let area_w = (surface_w - 80).min(780).max(160);
    let start_x = (surface_w - area_w) / 2;
    let mid_y = if is_audio_only { surface_h - 160 } else { surface_h - 140 };
    let amp = 44.0_f32;

    fill_rounded_rect(
        surface,
        surface_w,
        surface_h,
        Rect::new(start_x - 8, mid_y - 48, area_w + 16, 96),
        8,
        Color::rgba(11, 15, 25, 175),
    );
    draw_line(
        surface,
        surface_w,
        surface_h,
        start_x,
        mid_y,
        start_x + area_w,
        mid_y,
        Color::rgba(255, 255, 255, 35),
    );

    let mut prev_x = start_x;
    let mut prev_y = mid_y + (waveform[0].clamp(-1.0, 1.0) * amp) as i32;
    for i in 1..WAVEFORM_SAMPLES {
        let x = start_x + (i as i32 * area_w) / (WAVEFORM_SAMPLES as i32 - 1);
        let y = mid_y + (waveform[i].clamp(-1.0, 1.0) * amp) as i32;
        draw_line(surface, surface_w, surface_h, prev_x, prev_y, x, y, Color::rgb(56, 189, 248));
        draw_line(surface, surface_w, surface_h, prev_x, prev_y + 1, x, y + 1, Color::rgb(74, 222, 128));
        prev_x = x;
        prev_y = y;
    }
}
