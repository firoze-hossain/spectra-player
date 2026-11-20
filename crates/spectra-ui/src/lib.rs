//! # Nova Player UI Engine
//!
//! Modern acrylic UI, hardware-agnostic software rasterizer, 32-band audio visualizers,
//! video scaler, and interactive transport controls for Nova Player.

pub mod components;
pub mod draw;
pub mod font;
pub mod geom;
pub mod player;
pub mod scaler;

pub use draw::{draw_circle, draw_line, draw_rect_outline, fill_rect, fill_rounded_rect};
pub use font::{draw_char, draw_text, measure_text};
pub use geom::{Color, Rect};
pub use player::SpectraPlayerView;
pub type NovaPlayerView = SpectraPlayerView;
pub use scaler::{blit_video_frame, compute_viewport_rect};

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::types::AspectRatio;

    #[test]
    fn test_viewport_aspect_ratio_computation() {
        // 16:9 on 1024x768 screen -> letterbox (top/bottom bars)
        let vp = compute_viewport_rect(1024, 768, AspectRatio::Ratio16_9, 1920, 1080);
        assert_eq!(vp.w, 1024);
        assert_eq!(vp.h, 576);
        assert_eq!(vp.x, 0);
        assert_eq!(vp.y, (768 - 576) / 2);

        // 16:9 on ultra-wide 1920x800 screen -> pillarbox (left/right bars)
        let vp2 = compute_viewport_rect(1920, 800, AspectRatio::Ratio16_9, 1920, 1080);
        assert_eq!(vp2.h, 800);
        assert_eq!(vp2.w, (800.0 * 16.0 / 9.0) as i32);
    }

    #[test]
    fn test_player_view_render_and_click_dispatch() {
        let mut view = SpectraPlayerView::new();
        let mut surface = vec![0u32; 1024 * 720];

        // Render full frame
        view.render(&mut surface, 1024, 720);

        // Verify non-empty surface pixels
        assert!(surface.iter().any(|&p| p != 0));

        // Click on play/pause button
        let play_btn = view.bottom_areas.unwrap().play_pause_btn;
        let clicked = view.handle_click(play_btn.x + 5, play_btn.y + 5);
        assert!(clicked);
        assert_eq!(view.engine.state, spectra_core::types::PlaybackState::Paused);

        // Click again to resume
        view.handle_click(play_btn.x + 5, play_btn.y + 5);
        assert_eq!(view.engine.state, spectra_core::types::PlaybackState::Playing);
    }

    #[test]
    fn test_playlist_and_info_overlay_toggles() {
        let mut view = SpectraPlayerView::new();
        let mut surface = vec![0u32; 800 * 600];

        view.render(&mut surface, 800, 600);

        // Click playlist button in top bar
        let pl_btn = view.top_areas.unwrap().playlist_btn;
        view.handle_click(pl_btn.x + 4, pl_btn.y + 4);
        assert!(view.show_playlist);

        // Click info button in top bar
        let info_btn = view.top_areas.unwrap().info_btn;
        view.handle_click(info_btn.x + 4, info_btn.y + 4);
        assert!(view.show_info);
    }
}
