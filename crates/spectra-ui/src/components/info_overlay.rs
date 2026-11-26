//! Real Media Info & Codec Telemetry ("Stats for Nerds") overlay.

use spectra_core::types::{AspectRatio, MediaMetadata};
use crate::draw::{draw_rect_outline, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

#[allow(clippy::too_many_arguments)]
pub fn render_info_overlay(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    metadata: &MediaMetadata,
    fps: f32,
    measured_fps: f32,
    dropped_frames: u64,
    aspect: AspectRatio,
    volume: f32,
    speed: f32,
    active_sub: Option<&str>,
) -> Rect {
    let card_w = 460.min(surface_w - 40);
    let card_h = 330.min(surface_h - 100);
    let card_x = 20;
    let card_y = 74;
    let card_rect = Rect::new(card_x, card_y, card_w, card_h);

    fill_rounded_rect(surface, surface_w, surface_h, card_rect, 8, Color::rgba(11, 15, 25, 244));
    draw_rect_outline(surface, surface_w, surface_h, card_rect, 1, Color::rgb(56, 189, 248));

    let head_h = 32;
    fill_rounded_rect(
        surface,
        surface_w,
        surface_h,
        Rect::new(card_x, card_y, card_w, head_h),
        8,
        Color::rgba(30, 41, 59, 235),
    );
    draw_text(
        surface,
        surface_w,
        surface_h,
        card_x + 12,
        card_y + 10,
        "MEDIA INFORMATION & CODEC TELEMETRY",
        Color::rgb(56, 189, 248),
        1,
    );

    let close_rect = Rect::new(card_x + card_w - 26, card_y + 7, 18, 18);
    fill_rounded_rect(surface, surface_w, surface_h, close_rect, 3, Color::rgba(255, 255, 255, 22));
    draw_text(surface, surface_w, surface_h, close_rect.x + 6, close_rect.y + 4, "X", Color::WHITE, 1);

    let mut row_y = card_y + head_h + 12;
    let items: [(&str, String); 11] = [
        ("Source:", if metadata.path.is_empty() { "Demo Stream".to_string() } else { metadata.path.clone() }),
        ("Container:", metadata.format.name().to_string()),
        ("Decoder Engine:", metadata.backend.name().to_string()),
        ("Video Codec:", format!("{} ({})", metadata.video_codec, metadata.pixel_format)),
        ("Audio Codec:", metadata.audio_codec.clone()),
        (
            "Resolution:",
            if metadata.width > 0 {
                format!("{} x {} ({})", metadata.width, metadata.height, aspect.name())
            } else {
                "Audio Stream Only".to_string()
            },
        ),
        (
            "Frame Pacing:",
            format!("{:.1} src / {:.1} ui FPS ({} drops)", fps, measured_fps, dropped_frames),
        ),
        ("Bitrate:", format!("{} kbps", metadata.bitrate_kbps)),
        (
            "Audio Output:",
            format!("{}Hz {}ch | {:.0}% ({:.2}x)", metadata.sample_rate, metadata.channels, volume * 100.0, speed),
        ),
        (
            "Audio / Sub Tracks:",
            format!("{} audio / {} sub", metadata.audio_tracks.len().max(1), metadata.subtitle_tracks.len()),
        ),
        ("Active Subtitle:", active_sub.unwrap_or("None").to_string()),
    ];

    for (label, val) in &items {
        if row_y + 16 > card_y + card_h - 6 {
            break;
        }
        draw_text(surface, surface_w, surface_h, card_x + 14, row_y, label, Color::rgb(148, 163, 184), 1);
        let max_chars = ((card_w - 175) / 6).max(16) as usize;
        let val_sub = if val.chars().count() > max_chars {
            let cut: String = val.chars().take(max_chars.saturating_sub(3)).collect();
            format!("{cut}...")
        } else {
            val.clone()
        };
        draw_text(surface, surface_w, surface_h, card_x + 156, row_y, &val_sub, Color::WHITE, 1);
        row_y += 24;
    }

    close_rect
}
