//! Video frame scaler, aspect ratio letterboxer, and viewport blitter.

use spectra_core::types::{AspectRatio, VideoFrame};
use crate::geom::{Color, Rect};
use crate::draw::fill_rect;

pub fn compute_viewport_rect(surface_w: i32, surface_h: i32, aspect: AspectRatio, frame_w: u32, frame_h: u32) -> Rect {
    match aspect {
        AspectRatio::Fill => Rect::new(0, 0, surface_w, surface_h),
        _ => {
            let target_ratio = match aspect {
                AspectRatio::Ratio16_9 => 16.0 / 9.0,
                AspectRatio::Ratio4_3 => 4.0 / 3.0,
                AspectRatio::Ratio21_9 => 21.0 / 9.0,
                AspectRatio::Ratio1_1 => 1.0,
                AspectRatio::Auto => {
                    if frame_w > 0 && frame_h > 0 {
                        frame_w as f32 / frame_h as f32
                    } else {
                        16.0 / 9.0
                    }
                }
                AspectRatio::Fill => unreachable!(),
            };

            let screen_ratio = surface_w as f32 / surface_h as f32;

            if screen_ratio > target_ratio {
                // Screen is wider than video -> pillarbox (black bars on left & right)
                let vp_h = surface_h;
                let vp_w = (surface_h as f32 * target_ratio) as i32;
                let vp_x = (surface_w - vp_w) / 2;
                Rect::new(vp_x, 0, vp_w, vp_h)
            } else {
                // Screen is taller than video -> letterbox (black bars on top & bottom)
                let vp_w = surface_w;
                let vp_h = (surface_w as f32 / target_ratio) as i32;
                let vp_y = (surface_h - vp_h) / 2;
                Rect::new(0, vp_y, vp_w, vp_h)
            }
        }
    }
}

/// Blits and scales a video frame into the specified surface rectangle.
pub fn blit_video_frame(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    viewport: Rect,
    frame: &VideoFrame,
) {
    // 1. Fill surrounding background with pure black for crisp letterboxing
    if viewport.x > 0 {
        // Left pillar
        fill_rect(surface, surface_w, surface_h, Rect::new(0, 0, viewport.x, surface_h), Color::BLACK);
        // Right pillar
        fill_rect(surface, surface_w, surface_h, Rect::new(viewport.right(), 0, surface_w - viewport.right(), surface_h), Color::BLACK);
    }
    if viewport.y > 0 {
        // Top letterbox
        fill_rect(surface, surface_w, surface_h, Rect::new(0, 0, surface_w, viewport.y), Color::BLACK);
        // Bottom letterbox
        fill_rect(surface, surface_w, surface_h, Rect::new(0, viewport.bottom(), surface_w, surface_h - viewport.bottom()), Color::BLACK);
    }

    if frame.width == 0 || frame.height == 0 || viewport.w <= 0 || viewport.h <= 0 {
        return;
    }

    // 2. Fast scale/blit from frame.buffer into surface[viewport]
    let src_w = frame.width as usize;
    let src_h = frame.height as usize;
    let dst_w = viewport.w as usize;
    let dst_h = viewport.h as usize;

    let x0 = viewport.x.max(0) as usize;
    let y0 = viewport.y.max(0) as usize;
    let x1 = (viewport.x + viewport.w).min(surface_w) as usize;
    let y1 = (viewport.y + viewport.h).min(surface_h) as usize;

    if src_w == dst_w && src_h == dst_h {
        // Direct 1:1 row copy
        for y in y0..y1 {
            let src_y = y - y0;
            let src_row = &frame.buffer[src_y * src_w..(src_y + 1) * src_w];
            let dst_start = y * surface_w as usize + x0;
            let copy_len = (x1 - x0).min(src_w);
            surface[dst_start..dst_start + copy_len].copy_from_slice(&src_row[..copy_len]);
        }
    } else {
        // High-performance nearest/bilinear row mapping
        let x_ratio = ((src_w << 16) / dst_w) as u32;
        let y_ratio = ((src_h << 16) / dst_h) as u32;

        for y in y0..y1 {
            let dst_y_rel = y - y0;
            let src_y = ((dst_y_rel as u32 * y_ratio) >> 16) as usize;
            let src_row_start = src_y.min(src_h - 1) * src_w;
            let dst_row_start = y * surface_w as usize;

            for x in x0..x1 {
                let dst_x_rel = x - x0;
                let src_x = ((dst_x_rel as u32 * x_ratio) >> 16) as usize;
                let pixel = frame.buffer[src_row_start + src_x.min(src_w - 1)];
                surface[dst_row_start + x] = pixel;
            }
        }
    }
}
