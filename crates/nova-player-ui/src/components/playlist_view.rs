//! Sliding interactive playlist drawer component.

use nova_player_core::playlist::Playlist;
use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

pub struct PlaylistClickAreas {
    pub close_btn: Rect,
    pub item_rects: Vec<(usize, Rect)>,
}

pub fn render_playlist_drawer(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    playlist: &Playlist,
) -> PlaylistClickAreas {
    let drawer_w = 330.min(surface_w - 40);
    let drawer_x = surface_w - drawer_w;
    let drawer_y = 44;
    let drawer_h = surface_h - drawer_y - 12;
    let drawer_rect = Rect::new(drawer_x, drawer_y, drawer_w, drawer_h);

    // Drop shadow
    fill_rect(surface, surface_w, surface_h, Rect::new(drawer_x - 6, drawer_y, 6, drawer_h), Color::rgba(0, 0, 0, 80));

    // Acrylic frosted background
    fill_rounded_rect(surface, surface_w, surface_h, drawer_rect, 8, Color::rgba(15, 23, 42, 245));
    draw_rect_outline(surface, surface_w, surface_h, drawer_rect, 1, Color::rgba(255, 255, 255, 30));

    // Header
    let header_h = 40;
    fill_rounded_rect(surface, surface_w, surface_h, Rect::new(drawer_x, drawer_y, drawer_w, header_h), 8, Color::rgba(30, 41, 59, 200));
    draw_text(surface, surface_w, surface_h, drawer_x + 16, drawer_y + 14, "MEDIA PLAYLIST", Color::WHITE, 1);

    let count_text = format!("{} items", playlist.items.len());
    draw_text(surface, surface_w, surface_h, drawer_x + 130, drawer_y + 14, &count_text, Color::rgb(148, 163, 184), 1);

    // Close button [X]
    let close_rect = Rect::new(drawer_x + drawer_w - 32, drawer_y + 8, 24, 24);
    fill_rounded_rect(surface, surface_w, surface_h, close_rect, 4, Color::rgba(255, 255, 255, 20));
    draw_text(surface, surface_w, surface_h, close_rect.x + 8, close_rect.y + 7, "X", Color::WHITE, 1);

    // Track cards
    let mut item_rects = Vec::new();
    let card_margin = 8;
    let card_w = drawer_w - card_margin * 2;
    let card_h = 56;
    let start_y = drawer_y + header_h + 8;

    for (i, item) in playlist.items.iter().enumerate() {
        let card_y = start_y + (i as i32) * (card_h + 6);
        if card_y + card_h > drawer_y + drawer_h - 10 {
            break;
        }

        let card_rect = Rect::new(drawer_x + card_margin, card_y, card_w, card_h);
        let is_current = i == playlist.current_index;

        if is_current {
            // Glowing cyan card
            fill_rounded_rect(surface, surface_w, surface_h, card_rect, 6, Color::rgba(14, 165, 233, 40));
            draw_rect_outline(surface, surface_w, surface_h, card_rect, 1, Color::rgb(56, 189, 248));
            // Active play indicator pill
            fill_rounded_rect(surface, surface_w, surface_h, Rect::new(card_rect.x + 6, card_y + 18, 20, 20), 10, Color::rgb(56, 189, 248));
            draw_text(surface, surface_w, surface_h, card_rect.x + 13, card_y + 22, ">", Color::rgb(15, 23, 42), 1);
        } else {
            fill_rounded_rect(surface, surface_w, surface_h, card_rect, 6, Color::rgba(255, 255, 255, 12));
            draw_rect_outline(surface, surface_w, surface_h, card_rect, 1, Color::rgba(255, 255, 255, 20));
            let num_text = format!("{:02}", i + 1);
            draw_text(surface, surface_w, surface_h, card_rect.x + 10, card_y + 22, &num_text, Color::rgb(100, 116, 139), 1);
        }

        let text_x = card_rect.x + 34;
        let title_sub = if item.title.len() > 24 {
            format!("{}...", &item.title[..22])
        } else {
            item.title.clone()
        };
        let title_col = if is_current { Color::WHITE } else { Color::rgb(226, 232, 240) };
        draw_text(surface, surface_w, surface_h, text_x, card_y + 10, &title_sub, title_col, 1);

        let artist_sub = if item.artist.len() > 24 {
            format!("{}...", &item.artist[..22])
        } else {
            item.artist.clone()
        };
        draw_text(surface, surface_w, surface_h, text_x, card_y + 24, &artist_sub, Color::rgb(148, 163, 184), 1);

        // Format Badge & Duration
        let fmt_b = item.format.badge();
        let dur_str = format_dur(item.duration_ms);
        let badge_line = format!("{} • {}", fmt_b, dur_str);
        draw_text(surface, surface_w, surface_h, text_x, card_y + 38, &badge_line, Color::rgb(56, 189, 248), 1);

        item_rects.push((i, card_rect));
    }

    PlaylistClickAreas {
        close_btn: close_rect,
        item_rects,
    }
}

fn format_dur(ms: u64) -> String {
    let s = (ms / 1000) % 60;
    let m = (ms / 1000) / 60;
    format!("{:02}:{:02}", m, s)
}
