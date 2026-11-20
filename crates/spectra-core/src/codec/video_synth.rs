//! High-performance procedural video frame generator and decoder for 4K / 8K playback.

use crate::types::VideoFrame;

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
    pub grayscale: bool,
    pub invert: bool,
}

impl Default for VideoAdjustments {
    fn default() -> Self {
        Self {
            brightness: 0,
            contrast: 1.0,
            saturation: 1.0,
            grayscale: false,
            invert: false,
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

    /// Renders a full video frame at specified target dimensions and timestamp.
    pub fn render_frame(&self, width: u32, height: u32, time_ms: u64) -> VideoFrame {
        let mut frame = VideoFrame::new(width, height, time_ms);
        let t = (time_ms as f32) / 1000.0;
        let buf = &mut frame.buffer;

        match self.theme {
            VideoTheme::CyberpunkMetropolis => {
                render_cyberpunk(buf, width, height, t);
            }
            VideoTheme::DeepSpaceNebula => {
                render_deep_space(buf, width, height, t);
            }
            VideoTheme::OceanAurora => {
                render_ocean_aurora(buf, width, height, t);
            }
        }

        // Apply real-time post-processing adjustments if non-default
        if self.adjustments.brightness != 0
            || (self.adjustments.contrast - 1.0).abs() > 0.01
            || (self.adjustments.saturation - 1.0).abs() > 0.01
            || self.adjustments.grayscale
            || self.adjustments.invert
        {
            self.apply_adjustments(buf);
        }

        frame
    }

    fn apply_adjustments(&self, buf: &mut [u32]) {
        let br = self.adjustments.brightness;
        let ct = self.adjustments.contrast;
        let sat = self.adjustments.saturation;
        let gray = self.adjustments.grayscale;
        let inv = self.adjustments.invert;

        for pixel in buf.iter_mut() {
            let mut r = ((*pixel >> 16) & 0xFF) as f32;
            let mut g = ((*pixel >> 8) & 0xFF) as f32;
            let mut b = (*pixel & 0xFF) as f32;

            if inv {
                r = 255.0 - r;
                g = 255.0 - g;
                b = 255.0 - b;
            }

            // Brightness & Contrast
            r = ((r - 128.0) * ct + 128.0) + br as f32;
            g = ((g - 128.0) * ct + 128.0) + br as f32;
            b = ((b - 128.0) * ct + 128.0) + br as f32;

            // Saturation / Grayscale
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

            let ru = r.clamp(0.0, 255.0) as u32;
            let gu = g.clamp(0.0, 255.0) as u32;
            let bu = b.clamp(0.0, 255.0) as u32;

            *pixel = 0xFF000000 | (ru << 16) | (gu << 8) | bu;
        }
    }
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
                // Sky gradient: deep indigo to vibrant magenta near horizon
                let norm_y = y_f / horizon as f32;
                let r = (20.0 + norm_y * 110.0 + (x_f * 0.02 + t * 0.8).sin() * 8.0).clamp(0.0, 255.0) as u32;
                let g = (10.0 + norm_y * 30.0).clamp(0.0, 255.0) as u32;
                let b = (45.0 + norm_y * 140.0 + (x_f * 0.015 - t).cos() * 10.0).clamp(0.0, 255.0) as u32;

                // Neon sun / holographic orb
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

                // City skyscraper silhouettes
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
                    // Skyscraper body with glowing windows
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
                // Ground 3D Perspective Grid
                let depth = (y_f - horizon as f32).max(1.0);
                let z = (h as f32 - horizon as f32) / depth;
                let world_x = (x_f - cx) * z * 0.025;
                let world_z = z + t * 4.0;

                let grid_x = (world_x.fract().abs() < 0.06) as i32;
                let grid_z = ((world_z * 0.25).fract().abs() < 0.06) as i32;

                if grid_x != 0 || grid_z != 0 {
                    // Bright cyan / magenta perspective grid lines
                    let pulse = (world_z * 0.5 - t * 2.0).sin().abs();
                    let gr_r = (50.0 + pulse * 180.0) as u32;
                    let gr_g = (200.0 + pulse * 55.0) as u32;
                    let gr_b = 255;
                    buf[idx] = 0xFF000000 | (gr_r << 16) | (gr_g << 8) | gr_b;
                } else {
                    // Dark reflective tarmac with purple sheen
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

            // Swirling gravitational accretion disk
            let disk_r = 140.0;
            let disk_w = 45.0;
            let in_disk = (dist - disk_r).abs() < disk_w;

            if dist < 65.0 {
                // Event horizon black hole core
                buf[idx] = 0xFF030712;
            } else if in_disk {
                // Fiery accretion ring with relativistic Doppler shift (blue/violet on left, red/orange on right)
                let swirl = (angle * 4.0).sin().abs();
                let heat = 1.0 - ((dist - disk_r).abs() / disk_w);
                let doppler = (dx / dist).clamp(-1.0, 1.0);

                let r = ((180.0 + doppler * 60.0 + swirl * 40.0) * heat).clamp(0.0, 255.0) as u32;
                let g = ((90.0 + swirl * 80.0) * heat).clamp(0.0, 255.0) as u32;
                let b = ((220.0 - doppler * 100.0) * heat).clamp(0.0, 255.0) as u32;

                buf[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            } else {
                // Cosmic starfield & violet nebula dust
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
                // Sky & shimmering green/cyan aurora borealis ribbons
                let sky_grad = y_f / sea_y as f32;
                let ribbon1 = (x_f * 0.012 + t * 0.6).sin() * 25.0 + (x_f * 0.02 - t * 0.4).cos() * 15.0;
                let ribbon_y = (sea_y as f32 * 0.35) + ribbon1;
                let dist_rib = (y_f - ribbon_y).abs();
                let aurora = if dist_rib < 45.0 { 1.0 - dist_rib / 45.0 } else { 0.0 };

                let r = ((10.0 + sky_grad * 30.0 + aurora * 30.0)).clamp(0.0, 255.0) as u32;
                let g = ((15.0 + sky_grad * 45.0 + aurora * 230.0)).clamp(0.0, 255.0) as u32;
                let b = ((35.0 + sky_grad * 80.0 + aurora * 180.0)).clamp(0.0, 255.0) as u32;

                buf[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            } else {
                // Ocean water with animated wave ripples and specular aurora reflections
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
