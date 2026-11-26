//! Sliding interactive playlist drawer component with Add, Folder, Save, Clear, and per-item Remove.

use spectra_core::playlist::Playlist;
use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

pub struct PlaylistClickAreas {
    pub drawer_rect: Rect,
    pub close_btn: Rect,
    pub add_file_btn: Rect,
    pub add_dir_btn: Rect,
    pub save_btn: Rect,
    pub clear_btn: Rect,
    pub item_rects: Vec<(usize, Rect)>,
    pub remove_rects: Vec<(usize, Rect)>,
}

pub fn render_playlist_drawer(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    playlist: &Playlist,
    scroll_offset: usize,
) -> PlaylistClickAreas {
    let drawer_w = 340.min(surface_w - 40);
    let drawer_x = surface_w - drawer_w;
    let drawer_y = 66;
    let drawer_h = (surface_h - drawer_y - 88).max(120);
    let drawer_rect = Rect::new(drawer_x, drawer_y, drawer_w, drawer_h);

    fill_rect(
        surface,
        surface_w,
        surface_h,
        Rect::new(drawer_x - 6, drawer_y, 6, drawer_h),
        Color::rgba(0, 0, 0, 80),
    );
    fill_rounded_rect(surface, surface_w, surface_h, drawer_rect, 8, Color::rgba(15, 23, 42, 245));
    draw_rect_outline(surface, surface_w, surface_h, drawer_rect, 1, Color::rgba(255, 255, 255, 30));

    // Header
    let header_h = 38;
    fill_rounded_rect(
        surface,
        surface_w,
        surface_h,
        Rect::new(drawer_x, drawer_y, drawer_w, header_h),
        8,
        Color::rgba(30, 41, 59, 220),
    );
    draw_text(surface, surface_w, surface_h, drawer_x + 14, drawer_y + 13, "PLAYLIST", Color::WHITE, 1);

    let count_text = format!("{} items", playlist.items.len());
    draw_text(surface, surface_w, surface_h, drawer_x + 82, drawer_y + 13, &count_text, Color::rgb(148, 163, 184), 1);

    let close_rect = Rect::new(drawer_x + drawer_w - 30, drawer_y + 7, 22, 22);
    fill_rounded_rect(surface, surface_w, surface_h, close_rect, 4, Color::rgba(255, 255, 255, 20));
    draw_text(surface, surface_w, surface_h, close_rect.x + 7, close_rect.y + 6, "X", Color::WHITE, 1);

    // Action toolbar row: [+File] [+Dir] [Save] [Clear]
    let tool_y = drawer_y + header_h + 6;
    let btn_w = (drawer_w - 40) / 4;
    let add_file_btn = Rect::new(drawer_x + 8, tool_y, btn_w, 22);
    let add_dir_btn = Rect::new(add_file_btn.right() + 6, tool_y, btn_w, 22);
    let save_btn = Rect::new(add_dir_btn.right() + 6, tool_y, btn_w, 22);
    let clear_btn = Rect::new(save_btn.right() + 6, tool_y, btn_w, 22);

    for (r, lbl, col) in [
        (add_file_btn, "+FILE", Color::rgba(56, 189, 248, 45)),
        (add_dir_btn, "+DIR", Color::rgba(16, 185, 129, 45)),
        (save_btn, "SAVE", Color::rgba(255, 255, 255, 18)),
        (clear_btn, "CLR", Color::rgba(239, 68, 68, 45)),
    ] {
        fill_rounded_rect(surface, surface_w, surface_h, r, 4, col);
        draw_text(surface, surface_w, surface_h, r.x + 12, r.y + 6, lbl, Color::WHITE, 1);
    }

    let mut item_rects = Vec::new();
    let mut remove_rects = Vec::new();
    let card_margin = 8;
    let card_w = drawer_w - card_margin * 2;
    let card_h = 54;
    let start_y = tool_y + 28;

    if playlist.items.is_empty() {
        draw_text(
            surface,
            surface_w,
            surface_h,
            drawer_x + 20,
            start_y + 30,
            "Playlist is empty",
            Color::rgb(203, 213, 225),
            1,
        );
        draw_text(
            surface,
            surface_w,
            surface_h,
            drawer_x + 20,
            start_y + 48,
            "Click +FILE or +DIR to add media",
            Color::rgb(148, 163, 184),
            1,
        );
    }

    let start_idx = scroll_offset.min(playlist.items.len().saturating_sub(1));
    for (vis_idx, i) in (start_idx..playlist.items.len()).enumerate() {
        let item = &playlist.items[i];
        let card_y = start_y + (vis_idx as i32) * (card_h + 6);
        if card_y + card_h > drawer_y + drawer_h - 8 {
            break;
        }

        let card_rect = Rect::new(drawer_x + card_margin, card_y, card_w, card_h);
        let is_current = i == playlist.current_index;

        if is_current {
            fill_rounded_rect(surface, surface_w, surface_h, card_rect, 6, Color::rgba(14, 165, 233, 45));
            draw_rect_outline(surface, surface_w, surface_h, card_rect, 1, Color::rgb(56, 189, 248));
            fill_rounded_rect(surface, surface_w, surface_h, Rect::new(card_rect.x + 6, card_y + 17, 20, 20), 10, Color::rgb(56, 189, 248));
            draw_text(surface, surface_w, surface_h, card_rect.x + 13, card_y + 21, ">", Color::rgb(15, 23, 42), 1);
        } else {
            fill_rounded_rect(surface, surface_w, surface_h, card_rect, 6, Color::rgba(255, 255, 255, 12));
            draw_rect_outline(surface, surface_w, surface_h, card_rect, 1, Color::rgba(255, 255, 255, 20));
            let num_text = format!("{:02}", i + 1);
            draw_text(surface, surface_w, surface_h, card_rect.x + 10, card_y + 21, &num_text, Color::rgb(100, 116, 139), 1);
        }

        let text_x = card_rect.x + 34;
        let title_sub = if item.title.chars().count() > 26 {
            let cut: String = item.title.chars().take(23).collect();
            format!("{cut}...")
        } else {
            item.title.clone()
        };
        let title_col = if is_current { Color::WHITE } else { Color::rgb(226, 232, 240) };
        draw_text(surface, surface_w, surface_h, text_x, card_y + 9, &title_sub, title_col, 1);

        let artist_sub = if item.artist.chars().count() > 26 {
            let cut: String = item.artist.chars().take(23).collect();
            format!("{cut}...")
        } else {
            item.artist.clone()
        };
        draw_text(surface, surface_w, surface_h, text_x, card_y + 23, &artist_sub, Color::rgb(148, 163, 184), 1);

        let fmt_b = item.format.badge();
        let dur_str = if item.duration_ms > 0 { format_dur(item.duration_ms) } else { "--:--".to_string() };
        let res_str = if item.width > 0 {
            format!("{}x{} • ", item.width, item.height)
        } else {
            String::new()
        };
        let badge_line = format!("{}{} • {}", res_str, fmt_b, dur_str);
        draw_text(surface, surface_w, surface_h, text_x, card_y + 37, &badge_line, Color::rgb(56, 189, 248), 1);

        let rem_rect = Rect::new(card_rect.right() - 22, card_y + 6, 16, 16);
        fill_rounded_rect(surface, surface_w, surface_h, rem_rect, 3, Color::rgba(255, 255, 255, 16));
        draw_text(surface, surface_w, surface_h, rem_rect.x + 5, rem_rect.y + 3, "x", Color::rgb(148, 163, 184), 1);

        remove_rects.push((i, rem_rect));
        item_rects.push((i, card_rect));
    }

    PlaylistClickAreas {
        drawer_rect,
        close_btn: close_rect,
        add_file_btn,
        add_dir_btn,
        save_btn,
        clear_btn,
        item_rects,
        remove_rects,
    }
}

fn format_dur(ms: u64) -> String {
    let s = (ms / 1000) % 60;
    let m = (ms / 1000) / 60;
    format!("{:02}:{:02}", m, s)
}
