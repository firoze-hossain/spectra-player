//! Video frame scaler, aspect ratio letterboxer, zoom, and viewport blitter.

use spectra_core::types::{AspectRatio, VideoFrame};
use crate::draw::fill_rect;
use crate::geom::{Color, Rect};

pub fn compute_viewport_rect(
    surface_w: i32,
    surface_h: i32,
    aspect: AspectRatio,
    frame_w: u32,
    frame_h: u32,
) -> Rect {
    compute_viewport_rect_zoom(surface_w, surface_h, aspect, frame_w, frame_h, 1.0)
}

pub fn compute_viewport_rect_zoom(
    surface_w: i32,
    surface_h: i32,
    aspect: AspectRatio,
    frame_w: u32,
    frame_h: u32,
    zoom: f32,
) -> Rect {
    let base = match aspect {
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

            let screen_ratio = surface_w as f32 / surface_h.max(1) as f32;

            if screen_ratio > target_ratio {
                let vp_h = surface_h;
                let vp_w = (surface_h as f32 * target_ratio) as i32;
                let vp_x = (surface_w - vp_w) / 2;
                Rect::new(vp_x, 0, vp_w, vp_h)
            } else {
                let vp_w = surface_w;
                let vp_h = (surface_w as f32 / target_ratio) as i32;
                let vp_y = (surface_h - vp_h) / 2;
                Rect::new(0, vp_y, vp_w, vp_h)
            }
        }
    };

    if (zoom - 1.0).abs() < 0.01 {
        return base;
    }

    let z = zoom.clamp(0.25, 3.0);
    let zw = ((base.w as f32) * z) as i32;
    let zh = ((base.h as f32) * z) as i32;
    let zx = (surface_w - zw) / 2;
    let zy = (surface_h - zh) / 2;
    Rect::new(zx, zy, zw.max(1), zh.max(1))
}

/// Blits and scales a video frame into the specified surface rectangle with fast fixed-point interpolation.
pub fn blit_video_frame(
    surface: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    viewport: Rect,
    frame: &VideoFrame,
) {
    // Clear black bars around viewport when needed
    if viewport.x > 0 {
        fill_rect(surface, surface_w, surface_h, Rect::new(0, 0, viewport.x, surface_h), Color::BLACK);
        fill_rect(
            surface,
            surface_w,
            surface_h,
            Rect::new(viewport.right(), 0, surface_w - viewport.right(), surface_h),
            Color::BLACK,
        );
    }
    if viewport.y > 0 {
        fill_rect(surface, surface_w, surface_h, Rect::new(0, 0, surface_w, viewport.y), Color::BLACK);
        fill_rect(
            surface,
            surface_w,
            surface_h,
            Rect::new(0, viewport.bottom(), surface_w, surface_h - viewport.bottom()),
            Color::BLACK,
        );
    }

    if frame.width == 0 || frame.height == 0 || viewport.w <= 0 || viewport.h <= 0 {
        return;
    }

    let src_w = frame.width as usize;
    let src_h = frame.height as usize;
    if frame.buffer.len() < src_w * src_h {
        return;
    }
    let dst_w = viewport.w as usize;
    let dst_h = viewport.h as usize;

    let x0 = viewport.x.max(0) as usize;
    let y0 = viewport.y.max(0) as usize;
    let x1 = (viewport.x + viewport.w).min(surface_w).max(0) as usize;
    let y1 = (viewport.y + viewport.h).min(surface_h).max(0) as usize;
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    if src_w == dst_w && src_h == dst_h && viewport.x >= 0 && viewport.y >= 0 {
        for y in y0..y1 {
            let src_y = y - y0;
            let src_row = &frame.buffer[src_y * src_w..(src_y + 1) * src_w];
            let dst_start = y * surface_w as usize + x0;
            let copy_len = (x1 - x0).min(src_w);
            surface[dst_start..dst_start + copy_len].copy_from_slice(&src_row[..copy_len]);
        }
    } else {
        // Zero-allocation fast fixed-point scaling (stack LUT up to 4096px wide)
        let x_ratio = ((src_w << 16) / dst_w.max(1)) as u32;
        let y_ratio = ((src_h << 16) / dst_h.max(1)) as u32;

        let vis_w = x1 - x0;
        let mut stack_lut = [0usize; 4096];
        let mut heap_lut: Vec<usize>;
        let src_x_tbl: &mut [usize] = if vis_w <= stack_lut.len() {
            &mut stack_lut[..vis_w]
        } else {
            heap_lut = vec![0usize; vis_w];
            &mut heap_lut
        };
        for (idx, x) in (x0..x1).enumerate() {
            let dst_x_rel = (x as i32 - viewport.x).max(0) as u32;
            let sx = ((dst_x_rel * x_ratio) >> 16) as usize;
            src_x_tbl[idx] = sx.min(src_w - 1);
        }

        let sw_usize = surface_w as usize;
        for y in y0..y1 {
            let dst_y_rel = (y as i32 - viewport.y).max(0) as u32;
            let src_y = (((dst_y_rel * y_ratio) >> 16) as usize).min(src_h - 1);
            let src_row = &frame.buffer[src_y * src_w..(src_y + 1) * src_w];
            let dst_row = &mut surface[y * sw_usize + x0..y * sw_usize + x1];
            for (dst_px, &sx) in dst_row.iter_mut().zip(src_x_tbl.iter()) {
                *dst_px = unsafe { *src_row.get_unchecked(sx) };
            }
        }
    }
}
