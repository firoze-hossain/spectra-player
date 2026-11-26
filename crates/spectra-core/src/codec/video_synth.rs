//! High-performance video post-processing adjustments + procedural demo synthesizer.

use crate::types::{CropMode, VideoFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoTheme {
    CyberpunkMetropolis,
    DeepSpaceNebula,
    OceanAurora,
}

#[derive(Debug, Clone)]
pub struct VideoAdjustments {
    pub brightness: i32,  // -100 .. 100
    pub contrast: f32,    // 0.5 .. 2.0
    pub saturation: f32,  // 0.0 .. 2.0
    pub gamma: f32,       // 0.5 .. 2.5
    pub grayscale: bool,
    pub invert: bool,
    pub deinterlace: bool,
    pub zoom: f32,        // 0.25 .. 3.0
    pub crop: CropMode,
    pub rotate_deg: u16,  // 0, 90, 180, 270
    pub flip_h: bool,
}

impl Default for VideoAdjustments {
    fn default() -> Self {
        Self {
            brightness: 0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
            grayscale: false,
            invert: false,
            deinterlace: false,
            zoom: 1.0,
            crop: CropMode::None,
            rotate_deg: 0,
            flip_h: false,
        }
    }
}

impl VideoAdjustments {
    pub fn has_color_effects(&self) -> bool {
        self.brightness != 0
            || (self.contrast - 1.0).abs() > 0.01
            || (self.saturation - 1.0).abs() > 0.01
            || (self.gamma - 1.0).abs() > 0.02
            || self.grayscale
            || self.invert
            || self.deinterlace
    }

    pub fn has_geometry_effects(&self) -> bool {
        self.rotate_deg != 0 || self.flip_h || self.crop != CropMode::None
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Applies real-time color/deinterlace/geometry adjustments to any VideoFrame (real decoded or synth).
    pub fn apply_to_frame(&self, frame: &mut VideoFrame) {
        if self.crop != CropMode::None {
            apply_crop(frame, self.crop);
        }
        if self.flip_h {
            apply_flip_h(frame);
        }
        if self.rotate_deg != 0 {
            apply_rotate(frame, self.rotate_deg);
        }
        if self.deinterlace {
            apply_bob_deinterlace(frame);
        }
        if self.has_color_effects() {
            apply_color_adjustments(frame, self);
        }
    }
}

pub struct VideoSynthesizer {
    pub theme: VideoTheme,
    pub adjustments: VideoAdjustments,
}

impl VideoSynthesizer {
    pub fn new(theme: VideoTheme) -> Self {
        Self {
            theme,
            adjustments: VideoAdjustments::default(),
        }
    }

    /// Renders a procedural demo video frame at specified dimensions and timestamp.
    pub fn render_frame(&self, width: u32, height: u32, time_ms: u64) -> VideoFrame {
        let mut frame = VideoFrame::new(width, height, time_ms);
        let t = (time_ms as f32) / 1000.0;
        let buf = &mut frame.buffer;

        match self.theme {
            VideoTheme::CyberpunkMetropolis => render_cyberpunk(buf, width, height, t),
            VideoTheme::DeepSpaceNebula => render_deep_space(buf, width, height, t),
            VideoTheme::OceanAurora => render_ocean_aurora(buf, width, height, t),
        }

        self.adjustments.apply_to_frame(&mut frame);
        frame
    }
}

fn apply_color_adjustments(frame: &mut VideoFrame, adj: &VideoAdjustments) {
    let br = adj.brightness as f32 * 1.28;
    let ct = adj.contrast;
    let sat = adj.saturation;
    let inv_gamma = if (adj.gamma - 1.0).abs() > 0.02 {
        1.0 / adj.gamma.clamp(0.4, 2.5)
    } else {
        1.0
    };
    let use_gamma = (inv_gamma - 1.0).abs() > 0.01;
    let gray = adj.grayscale;
    let inv = adj.invert;

    // Precompute 256-entry gamma+contrast+brightness LUT when saturation == 1 && !gray
    if !gray && (sat - 1.0).abs() <= 0.01 {
        let mut lut = [0u32; 256];
        for i in 0..256 {
            let mut v = if inv { (255 - i) as f32 } else { i as f32 };
            v = ((v - 128.0) * ct + 128.0) + br;
            if use_gamma {
                v = (v.clamp(0.0, 255.0) / 255.0).powf(inv_gamma) * 255.0;
            }
            lut[i] = v.clamp(0.0, 255.0) as u32;
        }
        for pixel in frame.buffer.iter_mut() {
            let r = lut[((*pixel >> 16) & 0xFF) as usize];
            let g = lut[((*pixel >> 8) & 0xFF) as usize];
            let b = lut[(*pixel & 0xFF) as usize];
            *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
        }
        return;
    }

    for pixel in frame.buffer.iter_mut() {
        let mut r = ((*pixel >> 16) & 0xFF) as f32;
        let mut g = ((*pixel >> 8) & 0xFF) as f32;
        let mut b = (*pixel & 0xFF) as f32;

        if inv {
            r = 255.0 - r;
            g = 255.0 - g;
            b = 255.0 - b;
        }

        r = ((r - 128.0) * ct + 128.0) + br;
        g = ((g - 128.0) * ct + 128.0) + br;
        b = ((b - 128.0) * ct + 128.0) + br;

        let lum = 0.299 * r + 0.587 * g + 0.114 * b;
        if gray {
            r = lum;
            g = lum;
            b = lum;
        } else if (sat - 1.0).abs() > 0.01 {
            r = lum + (r - lum) * sat;
            g = lum + (g - lum) * sat;
            b = lum + (b - lum) * sat;
        }

        if use_gamma {
            r = (r.clamp(0.0, 255.0) / 255.0).powf(inv_gamma) * 255.0;
            g = (g.clamp(0.0, 255.0) / 255.0).powf(inv_gamma) * 255.0;
            b = (b.clamp(0.0, 255.0) / 255.0).powf(inv_gamma) * 255.0;
        }

        let ru = r.clamp(0.0, 255.0) as u32;
        let gu = g.clamp(0.0, 255.0) as u32;
        let bu = b.clamp(0.0, 255.0) as u32;
        *pixel = 0xFF000000 | (ru << 16) | (gu << 8) | bu;
    }
}

fn apply_bob_deinterlace(frame: &mut VideoFrame) {
    let w = frame.width as usize;
    let h = frame.height as usize;
    if w == 0 || h < 3 {
        return;
    }
    for y in (1..h - 1).step_by(2) {
        let up = (y - 1) * w;
        let mid = y * w;
        let dn = (y + 1) * w;
        for x in 0..w {
            let p1 = frame.buffer[up + x];
            let p2 = frame.buffer[dn + x];
            let r = (((p1 >> 16) & 0xFF) + ((p2 >> 16) & 0xFF)) >> 1;
            let g = (((p1 >> 8) & 0xFF) + ((p2 >> 8) & 0xFF)) >> 1;
            let b = ((p1 & 0xFF) + (p2 & 0xFF)) >> 1;
            frame.buffer[mid + x] = 0xFF000000 | (r << 16) | (g << 8) | b;
        }
    }
}

fn apply_flip_h(frame: &mut VideoFrame) {
    let w = frame.width as usize;
    let h = frame.height as usize;
    for y in 0..h {
        let row = &mut frame.buffer[y * w..(y + 1) * w];
        row.reverse();
    }
}

fn apply_rotate(frame: &mut VideoFrame, deg: u16) {
    let w = frame.width as usize;
    let h = frame.height as usize;
    if w == 0 || h == 0 {
        return;
    }
    match deg % 360 {
        90 => {
            let mut out = vec![0u32; w * h];
            for y in 0..h {
                for x in 0..w {
                    out[x * h + (h - 1 - y)] = frame.buffer[y * w + x];
                }
            }
            frame.buffer = out;
            frame.width = h as u32;
            frame.height = w as u32;
        }
        180 => {
            frame.buffer.reverse();
        }
        270 => {
            let mut out = vec![0u32; w * h];
            for y in 0..h {
                for x in 0..w {
                    out[(w - 1 - x) * h + y] = frame.buffer[y * w + x];
                }
            }
            frame.buffer = out;
            frame.width = h as u32;
            frame.height = w as u32;
        }
        _ => {}
    }
}

fn apply_crop(frame: &mut VideoFrame, crop: CropMode) {
    let target_ar = match crop {
        CropMode::None => return,
        CropMode::Ratio16_9 => 16.0 / 9.0,
        CropMode::Ratio4_3 => 4.0 / 3.0,
        CropMode::Ratio21_9 => 21.0 / 9.0,
        CropMode::Ratio1_1 => 1.0,
    };
    let w = frame.width as usize;
    let h = frame.height as usize;
    if w == 0 || h == 0 {
        return;
    }
    let cur_ar = w as f32 / h as f32;
    let (cw, ch) = if cur_ar > target_ar {
        (((h as f32 * target_ar) as usize).max(2).min(w), h)
    } else {
        (w, ((w as f32 / target_ar) as usize).max(2).min(h))
    };
    let x0 = (w - cw) / 2;
    let y0 = (h - ch) / 2;
    let mut out = Vec::with_capacity(cw * ch);
    for y in y0..(y0 + ch) {
        out.extend_from_slice(&frame.buffer[y * w + x0..y * w + x0 + cw]);
    }
    frame.buffer = out;
    frame.width = cw as u32;
    frame.height = ch as u32;
}

/// Cyberpunk Metropolis theme: Neon skylines, holographic grid, floating lights.
fn render_cyberpunk(buf: &mut [u32], w: u32, h: u32, t: f32) {
    let horizon = (h as f32 * 0.58) as i32;
    let cx = w as f32 * 0.5;

    for y in 0..h {
        let y_f = y as f32;
        let is_sky = (y as i32) < horizon;

        for x in 0..w {
            let idx = (y * w + x) as usize;
            let x_f = x as f32;

            if is_sky {
                let norm_y = y_f / horizon.max(1) as f32;
                let r = (20.0 + norm_y * 110.0 + (x_f * 0.02 + t * 0.8).sin() * 8.0).clamp(0.0, 255.0) as u32;
                let g = (10.0 + norm_y * 30.0).clamp(0.0, 255.0) as u32;
                let b = (45.0 + norm_y * 140.0 + (x_f * 0.015 - t).cos() * 10.0).clamp(0.0, 255.0) as u32;

                let dx = x_f - cx;
                let dy = y_f - (horizon as f32 - 40.0);
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 90.0 {
                    let glow = 1.0 - (dist / 90.0);
                    let sun_r = (255.0 * glow).min(255.0) as u32;
                    let sun_g = (120.0 * glow * glow).min(255.0) as u32;
                    let sun_b = (180.0 * glow).min(255.0) as u32;
                    buf[idx] = 0xFF000000 | (r.max(sun_r) << 16) | (g.max(sun_g) << 8) | b.max(sun_b);
                    continue;
                }

                let bldg_col = (x / 28) % 7;
                let bldg_h = match bldg_col {
                    0 => 110,
                    1 => 170,
                    2 => 140,
                    3 => 210,
                    4 => 160,
                    5 => 190,
                    _ => 130,
                };
                if (horizon - y as i32) < bldg_h && ((x % 28) > 2) {
                    let is_window = ((y / 8) % 3 == 0) && ((x / 6) % 2 == 0);
                    if is_window {
                        let win_color = if (x + y) % 5 == 0 { 0xFF00F0FF } else { 0xFFFF007F };
                        buf[idx] = win_color;
                    } else {
                        buf[idx] = 0xFF0B0F19;
                    }
                    continue;
                }

                buf[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            } else {
                let depth = (y_f - horizon as f32).max(1.0);
                let z = (h as f32 - horizon as f32) / depth;
                let world_x = (x_f - cx) * z * 0.025;
                let world_z = z + t * 4.0;

                let grid_x = (world_x.fract().abs() < 0.06) as i32;
                let grid_z = ((world_z * 0.25).fract().abs() < 0.06) as i32;

                if grid_x != 0 || grid_z != 0 {
                    let pulse = (world_z * 0.5 - t * 2.0).sin().abs();
                    let gr_r = (50.0 + pulse * 180.0) as u32;
                    let gr_g = (200.0 + pulse * 55.0) as u32;
                    let gr_b = 255;
                    buf[idx] = 0xFF000000 | (gr_r << 16) | (gr_g << 8) | gr_b;
                } else {
                    let fog = (1.0 / (depth * 0.08 + 1.0)).clamp(0.0, 1.0);
                    let bg_r = (12.0 + fog * 40.0) as u32;
                    let bg_g = (8.0 + fog * 20.0) as u32;
                    let bg_b = (24.0 + fog * 60.0) as u32;
                    buf[idx] = 0xFF000000 | (bg_r << 16) | (bg_g << 8) | bg_b;
                }
            }
        }
    }
}

/// Deep Space Nebula & Accretion Disk theme.
fn render_deep_space(buf: &mut [u32], w: u32, h: u32, t: f32) {
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;

    for y in 0..h {
        let y_f = y as f32;
        let dy = y_f - cy;

        for x in 0..w {
            let idx = (y * w + x) as usize;
            let x_f = x as f32;
            let dx = x_f - cx;

            let dist = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx) + t * 0.3;

            let disk_r = 140.0;
            let disk_w = 45.0;
            let in_disk = (dist - disk_r).abs() < disk_w;

            if dist < 65.0 {
                buf[idx] = 0xFF030712;
            } else if in_disk {
                let swirl = (angle * 4.0).sin().abs();
                let heat = 1.0 - ((dist - disk_r).abs() / disk_w);
                let doppler = (dx / dist).clamp(-1.0, 1.0);

                let r = ((180.0 + doppler * 60.0 + swirl * 40.0) * heat).clamp(0.0, 255.0) as u32;
                let g = ((90.0 + swirl * 80.0) * heat).clamp(0.0, 255.0) as u32;
                let b = ((220.0 - doppler * 100.0) * heat).clamp(0.0, 255.0) as u32;

                buf[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            } else {
                let star = ((x * 43 + y * 71 + (t * 2.0) as u32) % 317 == 0) as u32;
                let neb = ((x_f * 0.008 + t * 0.1).sin() * (y_f * 0.008 - t * 0.08).cos()).abs();

                let r = (neb * 65.0 + star as f32 * 220.0).clamp(0.0, 255.0) as u32;
                let g = (neb * 25.0 + star as f32 * 220.0).clamp(0.0, 255.0) as u32;
                let b = (neb * 110.0 + star as f32 * 255.0).clamp(0.0, 255.0) as u32;

                buf[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }
    }
}

/// Ocean Sunset & Northern Lights Aurora theme.
fn render_ocean_aurora(buf: &mut [u32], w: u32, h: u32, t: f32) {
    let sea_y = (h as f32 * 0.62) as u32;

    for y in 0..h {
        let y_f = y as f32;

        for x in 0..w {
            let idx = (y * w + x) as usize;
            let x_f = x as f32;

            if y < sea_y {
                let sky_grad = y_f / sea_y.max(1) as f32;
                let ribbon1 = (x_f * 0.012 + t * 0.6).sin() * 25.0 + (x_f * 0.02 - t * 0.4).cos() * 15.0;
                let ribbon_y = (sea_y as f32 * 0.35) + ribbon1;
                let dist_rib = (y_f - ribbon_y).abs();
                let aurora = if dist_rib < 45.0 { 1.0 - dist_rib / 45.0 } else { 0.0 };

                let r = (10.0 + sky_grad * 30.0 + aurora * 30.0).clamp(0.0, 255.0) as u32;
                let g = (15.0 + sky_grad * 45.0 + aurora * 230.0).clamp(0.0, 255.0) as u32;
                let b = (35.0 + sky_grad * 80.0 + aurora * 180.0).clamp(0.0, 255.0) as u32;

                buf[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            } else {
                let water_depth = (y - sea_y) as f32;
                let wave = ((x_f * 0.03 + t * 2.5).sin() * (water_depth * 0.1).sin()).abs();

                let r = (8.0 + wave * 25.0) as u32;
                let g = (20.0 + wave * 110.0) as u32;
                let b = (45.0 + wave * 140.0) as u32;

                buf[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }
    }
}
