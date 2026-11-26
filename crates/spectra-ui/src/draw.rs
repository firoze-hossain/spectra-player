//! Drawing primitives: filled rectangles, rounded acrylic rectangles, lines, and blitting.

use crate::geom::{Color, Rect};

pub fn fill_rect(surface: &mut [u32], surface_w: i32, surface_h: i32, rect: Rect, color: Color) {
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = (rect.x + rect.w).min(surface_w);
    let y1 = (rect.y + rect.h).min(surface_h);

    if x0 >= x1 || y0 >= y1 {
        return;
    }

    if color.a == 255 {
        let col = color.to_u32();
        for y in y0..y1 {
            let row_start = (y * surface_w + x0) as usize;
            let row_end = (y * surface_w + x1) as usize;
            surface[row_start..row_end].fill(col);
        }
    } else if color.a > 0 {
        // Fast integer packed RB/G alpha blend (5x faster than per-pixel Color structs)
        let a = color.a as u32 + 1;
        let inv_a = 256 - a;
        let fg_rb = ((color.r as u32) << 16) | (color.b as u32);
        let fg_g = (color.g as u32) << 8;
        let sw = surface_w as usize;
        for y in y0..y1 {
            let row = &mut surface[y as usize * sw + x0 as usize..y as usize * sw + x1 as usize];
            for px in row.iter_mut() {
                let bg = *px;
                let rb = (((bg & 0x00FF00FF) * inv_a + fg_rb * a) >> 8) & 0x00FF00FF;
                let g = (((bg & 0x0000FF00) * inv_a + fg_g * a) >> 8) & 0x0000FF00;
                *px = 0xFF000000 | rb | g;
            }
        }
    }
}

pub fn draw_rect_outline(surface: &mut [u32], surface_w: i32, surface_h: i32, rect: Rect, thickness: i32, color: Color) {
    // Top
    fill_rect(surface, surface_w, surface_h, Rect::new(rect.x, rect.y, rect.w, thickness), color);
    // Bottom
    fill_rect(surface, surface_w, surface_h, Rect::new(rect.x, rect.y + rect.h - thickness, rect.w, thickness), color);
    // Left
    fill_rect(surface, surface_w, surface_h, Rect::new(rect.x, rect.y, thickness, rect.h), color);
    // Right
    fill_rect(surface, surface_w, surface_h, Rect::new(rect.x + rect.w - thickness, rect.y, thickness, rect.h), color);
}

pub fn fill_rounded_rect(surface: &mut [u32], surface_w: i32, surface_h: i32, rect: Rect, radius: i32, color: Color) {
    if radius <= 1 {
        fill_rect(surface, surface_w, surface_h, rect, color);
        return;
    }

    let r = radius.min(rect.w / 2).min(rect.h / 2);

    // Center body
    fill_rect(surface, surface_w, surface_h, Rect::new(rect.x, rect.y + r, rect.w, rect.h - 2 * r), color);
    // Top inner rect
    fill_rect(surface, surface_w, surface_h, Rect::new(rect.x + r, rect.y, rect.w - 2 * r, r), color);
    // Bottom inner rect
    fill_rect(surface, surface_w, surface_h, Rect::new(rect.x + r, rect.y + rect.h - r, rect.w - 2 * r, r), color);

    // 4 rounded corners
    for dy in 0..r {
        for dx in 0..r {
            let dist_sq = (r - 1 - dx) * (r - 1 - dx) + (r - 1 - dy) * (r - 1 - dy);
            if dist_sq <= r * r {
                // Top-Left
                plot_pixel(surface, surface_w, surface_h, rect.x + dx, rect.y + dy, color);
                // Top-Right
                plot_pixel(surface, surface_w, surface_h, rect.x + rect.w - 1 - dx, rect.y + dy, color);
                // Bottom-Left
                plot_pixel(surface, surface_w, surface_h, rect.x + dx, rect.y + rect.h - 1 - dy, color);
                // Bottom-Right
                plot_pixel(surface, surface_w, surface_h, rect.x + rect.w - 1 - dx, rect.y + rect.h - 1 - dy, color);
            }
        }
    }
}

pub fn draw_circle(surface: &mut [u32], surface_w: i32, surface_h: i32, cx: i32, cy: i32, radius: i32, color: Color) {
    let r2 = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= r2 {
                plot_pixel(surface, surface_w, surface_h, cx + dx, cy + dy, color);
            }
        }
    }
}

pub fn draw_line(surface: &mut [u32], surface_w: i32, surface_h: i32, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut cx = x0;
    let mut cy = y0;

    loop {
        plot_pixel(surface, surface_w, surface_h, cx, cy, color);
        if cx == x1 && cy == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
        }
    }
}

#[inline(always)]
fn plot_pixel(surface: &mut [u32], surface_w: i32, surface_h: i32, x: i32, y: i32, color: Color) {
    if x >= 0 && x < surface_w && y >= 0 && y < surface_h {
        let idx = (y * surface_w + x) as usize;
        if color.a == 255 {
            surface[idx] = color.to_u32();
        } else if color.a > 0 {
            let bg = Color::from_u32(surface[idx]);
            surface[idx] = color.blend_over(bg).to_u32();
        }
    }
}
