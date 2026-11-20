//! Media Info & Codec Inspector ("Stats for Nerds") overlay.

use nova_player_core::types::{AspectRatio, MediaMetadata};
use crate::draw::{draw_rect_outline, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

pub fn render_info_overlay(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    metadata: &MediaMetadata,
    fps: f32,
    _dropped_frames: u64,
    aspect: AspectRatio,
    volume: f32,
) -> Rect {
    let card_w = 420.min(surface_w - 40);
    let card_h = 280.min(surface_h - 80);
    let card_x = 20;
    let card_y = 54;
    let card_rect = Rect::new(card_x, card_y, card_w, card_h);

    // Frosted acrylic dark card
    fill_rounded_rect(surface, surface_w, surface_h, card_rect, 8, Color::rgba(11, 15, 25, 248));
    draw_rect_outline(surface, surface_w, surface_h, card_rect, 1, Color::rgb(56, 189, 248));

    // Header
    let head_h = 32;
    fill_rounded_rect(surface, surface_w, surface_h, Rect::new(card_x, card_y, card_w, head_h), 8, Color::rgba(30, 41, 59, 230));
    draw_text(surface, surface_w, surface_h, card_x + 12, card_y + 10, "MEDIA INFO & CODEC TELEMETRY", Color::rgb(56, 189, 248), 1);

    // Close button
    let close_rect = Rect::new(card_x + card_w - 24, card_y + 6, 18, 18);
    fill_rounded_rect(surface, surface_w, surface_h, close_rect, 3, Color::rgba(255, 255, 255, 20));
    draw_text(surface, surface_w, surface_h, close_rect.x + 6, close_rect.y + 4, "X", Color::WHITE, 1);

    // Stats items
    let mut row_y = card_y + head_h + 12;
    let items: [(&str, String); 8] = [
        ("Container Format:", metadata.format.name().to_string()),
        ("Video Stream:", metadata.video_codec.clone()),
        ("Audio Stream:", metadata.audio_codec.clone()),
        (
            "Native Resolution:",
            if metadata.width > 0 {
                format!("{} x {} ({})", metadata.width, metadata.height, aspect.name())
            } else {
                "Audio Stream Only".to_string()
            },
        ),
        ("Frame Rate & Pacing:", format!("{:.2} FPS (0 drops / 0.0%)", fps)),
        ("Bitrate Estimate:", format!("{} kbps", metadata.bitrate_kbps)),
        ("Audio Level / Gain:", format!("{:.0}% ({})", volume * 100.0, if volume > 1.0 { "+Boost" } else { "Norm" })),
        ("Media Architecture:", "Pure-Rust SIMD NovaOS Pipeline".to_string()),
    ];

    for (label, val) in &items {
        if row_y + 16 > card_y + card_h - 6 {
            break;
        }
        draw_text(surface, surface_w, surface_h, card_x + 14, row_y, label, Color::rgb(148, 163, 184), 1);
        let val_sub = if val.len() > 30 {
            format!("{}...", &val[..28])
        } else {
            val.clone()
        };
        draw_text(surface, surface_w, surface_h, card_x + 160, row_y, &val_sub, Color::WHITE, 1);
        row_y += 24;
    }

    close_rect
}
