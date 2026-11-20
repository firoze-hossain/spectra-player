//! 32-band real-time audio spectrum analyzer and vinyl turntable visualizer.

use spectra_core::codec::audio_dsp::SPECTRUM_BANDS;
use spectra_core::types::MediaMetadata;
use crate::draw::{draw_circle, fill_rect, fill_rounded_rect};
use crate::font::{draw_text, measure_text};
use crate::geom::{Color, Rect};

pub fn render_audio_visualizer(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    spectrum_levels: &[f32; SPECTRUM_BANDS],
    spectrum_peaks: &[f32; SPECTRUM_BANDS],
    metadata: &MediaMetadata,
    time_ms: u64,
    is_audio_only: bool,
) {
    if is_audio_only {
        // Render rich dark studio backdrop
        fill_rect(surface, surface_w, surface_h, Rect::new(0, 0, surface_w, surface_h), Color::rgb(11, 15, 25));

        // Spinning vinyl turntable album art in center
        let cx = surface_w / 2;
        let cy = surface_h / 2 - 40;
        let vinyl_r = (surface_h / 6).clamp(60, 110);

        // Vinyl outer disc (dark charcoal with concentric grooves)
        draw_circle(surface, surface_w, surface_h, cx, cy, vinyl_r, Color::rgb(24, 24, 27));
        draw_circle(surface, surface_w, surface_h, cx, cy, vinyl_r - 6, Color::rgb(39, 39, 42));
        draw_circle(surface, surface_w, surface_h, cx, cy, vinyl_r - 14, Color::rgb(24, 24, 27));
        draw_circle(surface, surface_w, surface_h, cx, cy, vinyl_r - 22, Color::rgb(39, 39, 42));

        // Inner glowing album label
        let label_r = vinyl_r / 2;
        draw_circle(surface, surface_w, surface_h, cx, cy, label_r, Color::rgb(14, 165, 233));
        draw_circle(surface, surface_w, surface_h, cx, cy, 8, Color::rgb(255, 255, 255)); // spindle hole

        // Ambient radial light shine
        let angle = time_ms as f32 * 0.003;
        let shine_dx = (angle.cos() * (vinyl_r as f32 * 0.7)) as i32;
        let shine_dy = (angle.sin() * (vinyl_r as f32 * 0.7)) as i32;
        draw_circle(surface, surface_w, surface_h, cx + shine_dx, cy + shine_dy, 12, Color::rgba(255, 255, 255, 70));

        // Track title & artist beneath album
        let title_w = measure_text(&metadata.title, 2);
        draw_text(surface, surface_w, surface_h, cx - title_w / 2, cy + vinyl_r + 20, &metadata.title, Color::WHITE, 2);

        let artist_w = measure_text(&metadata.artist, 1);
        draw_text(surface, surface_w, surface_h, cx - artist_w / 2, cy + vinyl_r + 44, &metadata.artist, Color::rgb(148, 163, 184), 1);

        let info_line = format!("{} • {} • {}kHz", metadata.audio_codec, metadata.format.name(), metadata.sample_rate / 1000);
        let info_w = measure_text(&info_line, 1);
        draw_text(surface, surface_w, surface_h, cx - info_w / 2, cy + vinyl_r + 62, &info_line, Color::rgb(56, 189, 248), 1);
    }

    // Render 32-Band Spectrum Equalizer Bars
    let bar_area_w = (surface_w - 80).min(760);
    let start_x = (surface_w - bar_area_w) / 2;
    let base_y = if is_audio_only { surface_h - 120 } else { surface_h - 110 };
    let max_bar_h = if is_audio_only { 90 } else { 65 };

    let band_gap = 3;
    let band_w = (bar_area_w - (SPECTRUM_BANDS as i32 - 1) * band_gap) / (SPECTRUM_BANDS as i32);

    for i in 0..SPECTRUM_BANDS {
        let x = start_x + (i as i32) * (band_w + band_gap);
        let level = spectrum_levels[i].clamp(0.02, 1.0);
        let bh = (level * max_bar_h as f32) as i32;
        let by = base_y - bh;

        // Dynamic spectral color gradient: cyan -> violet -> orange
        let pct = i as f32 / SPECTRUM_BANDS as f32;
        let r = (40.0 + pct * 215.0) as u8;
        let g = (190.0 - pct * 100.0) as u8;
        let b = (250.0 - pct * 180.0) as u8;
        let bar_col = Color::rgba(r, g, b, 230);

        // Spectrum bar body
        fill_rounded_rect(surface, surface_w, surface_h, Rect::new(x, by, band_w, bh), 2, bar_col);

        // Peak hold cap line
        let peak = spectrum_peaks[i].clamp(0.0, 1.0);
        let peak_y = base_y - (peak * max_bar_h as f32) as i32 - 3;
        fill_rect(surface, surface_w, surface_h, Rect::new(x, peak_y, band_w, 2), Color::WHITE);

        // Glass floor reflection (decaying downward)
        let refl_h = (bh / 3).min(20);
        for dy in 0..refl_h {
            let alpha = ((1.0 - dy as f32 / refl_h as f32) * 60.0) as u8;
            fill_rect(surface, surface_w, surface_h, Rect::new(x, base_y + 2 + dy, band_w, 1), Color::rgba(r, g, b, alpha));
        }
    }
}
