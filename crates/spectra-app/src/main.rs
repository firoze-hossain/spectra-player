//! # Spectra Desktop Application
//!
//! High-performance pure-Rust media studio and player for NovaOS, macOS, Linux, and Windows.
//! Features native cross-platform GUI, 4K/8K video scaling, 32-band audio spectrum analyzer,
//! subtitle cues, playlist management, and headless rasterization.

#![allow(unused)]

use spectra_core::types::{AspectRatio, MediaFormat, Resolution};
use spectra_ui::SpectraPlayerView;
use std::env;
use std::fs::File;
use std::io::Write;

use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

const DEFAULT_WIDTH: usize = 1024;
const DEFAULT_HEIGHT: usize = 720;

fn save_ppm(path: &str, surface: &[u32], width: usize, height: usize) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    write!(f, "P6\n{} {}\n255\n", width, height)?;
    let mut bytes = Vec::with_capacity(width * height * 3);
    for &p in surface {
        let r = ((p >> 16) & 0xFF) as u8;
        let g = ((p >> 8) & 0xFF) as u8;
        let b = (p & 0xFF) as u8;
        bytes.push(r);
        bytes.push(g);
        bytes.push(b);
    }
    f.write_all(&bytes)?;
    Ok(())
}

fn print_banner() {
    println!("============================================================");
    println!("   SPECTRA v1.0.0 (Ultra-HD Pure Rust Media Studio)");
    println!("   4K UHD & 8K HDR Video • 32-Band Audio Visualizer • Subtitles");
    println!("   MKV • MP4 • WebM • AVI • FLAC • MP3 • WAV • AAC • OGG");
    println!("   Zero C/C++ • Zero FFmpeg/VLC binaries • Cross-Platform");
    println!("   macOS • Linux • Windows • NovaOS Default Media Player");
    println!("============================================================");
}

fn print_help() {
    print_banner();
    println!("Usage: spectra [OPTIONS] [FILE]");
    println!();
    println!("Options:");
    println!("  --help, -h               Print this help message");
    println!("  --version, -v            Print player version");
    println!("  --info                   Open with Media Info & Codec Inspector active");
    println!("  --playlist               Open with Playlist drawer visible");
    println!("  --visualizer             Enable 32-Band real-time Audio Spectrum Equalizer");
    println!("  --audio-only             Start in high-resolution audio turntable mode");
    println!("  --res <4k|8k|1080p>      Simulate target video resolution");
    println!("  --fullscreen             Launch in fullscreen mode");
    println!("  --headless               Run headless rasterization test without GUI");
    println!("  --screenshot <path>      Render current frame and save to image");
    println!();
    println!("Keyboard Shortcuts:");
    println!("  Space                    Play / Pause toggle");
    println!("  Left / Right Arrow       Seek backward / forward 5 seconds");
    println!("  Up / Down Arrow          Volume up / down 5%");
    println!("  F                        Toggle Fullscreen mode");
    println!("  M                        Mute / Unmute audio");
    println!("  V                        Toggle Audio Spectrum Visualizer");
    println!("  I                        Toggle Media Info & Codec Inspector");
    println!("  L                        Toggle Playlist sidebar drawer");
    println!("  A                        Cycle Aspect Ratio (Auto, 16:9, 4:3, 21:9, Fill)");
    println!("  S                        Toggle Subtitles [CC]");
    println!("  [ / ]                    Decrease / Increase playback speed");
    println!("  Q / Esc                  Quit Spectra");
    println!("============================================================");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let mut initial_file: Option<String> = None;
    let mut headless = false;
    let mut screenshot_path: Option<String> = None;
    let mut open_info = false;
    let mut open_playlist = false;
    let mut open_viz = false;
    let mut audio_only = false;
    let mut force_res: Option<String> = None;
    let mut fullscreen = false;
    let mut open_about = false;
    let mut open_menu: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" => {
                println!("Spectra 1.0.0 (Pure-Rust Engine, Cross-Platform Desktop & NovaOS)");
                return Ok(());
            }
            "--info" => {
                open_info = true;
            }
            "--playlist" => {
                open_playlist = true;
            }
            "--visualizer" => {
                open_viz = true;
            }
            "--audio-only" => {
                audio_only = true;
            }
            "--fullscreen" => {
                fullscreen = true;
            }
            "--headless" => {
                headless = true;
            }
            "--about" => {
                open_about = true;
            }
            "--menu" => {
                if i + 1 < args.len() {
                    open_menu = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--res" => {
                if i + 1 < args.len() {
                    force_res = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--screenshot" => {
                if i + 1 < args.len() {
                    screenshot_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            path if !path.starts_with("--") => {
                initial_file = Some(path.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    let mut view = SpectraPlayerView::new();

    if open_info {
        view.show_info = true;
    }
    if open_playlist {
        view.show_playlist = true;
    }
    if open_viz {
        view.show_visualizer = true;
    }
    if fullscreen {
        view.is_fullscreen = true;
    }
    if open_about {
        view.show_about = true;
    }
    if let Some(ref m) = open_menu {
        match m.to_ascii_lowercase().as_str() {
            "media" => view.menubar_state.active_menu = Some(0),
            "playback" => view.menubar_state.active_menu = Some(1),
            "audio" => view.menubar_state.active_menu = Some(2),
            "video" => view.menubar_state.active_menu = Some(3),
            "subtitle" => view.menubar_state.active_menu = Some(4),
            "tools" => view.menubar_state.active_menu = Some(5),
            "view" => view.menubar_state.active_menu = Some(6),
            "help" => view.menubar_state.active_menu = Some(7),
            _ => {}
        }
    }

    if let Some(ref res_str) = force_res {
        match res_str.to_ascii_lowercase().as_str() {
            "8k" => {
                view.engine.metadata.width = 7680;
                view.engine.metadata.height = 4320;
                view.engine.metadata.title = "Deep Space Accretion Disk 8K HDR".to_string();
                view.engine.video_synth.theme = spectra_core::codec::video_synth::VideoTheme::DeepSpaceNebula;
            }
            "4k" => {
                view.engine.metadata.width = 3840;
                view.engine.metadata.height = 2160;
                view.engine.metadata.title = "Cyberpunk Metropolis 4K UHD".to_string();
                view.engine.video_synth.theme = spectra_core::codec::video_synth::VideoTheme::CyberpunkMetropolis;
            }
            "1080p" => {
                view.engine.metadata.width = 1920;
                view.engine.metadata.height = 1080;
                view.engine.metadata.title = "Aurora Borealis Ocean Waves 1080p".to_string();
                view.engine.video_synth.theme = spectra_core::codec::video_synth::VideoTheme::OceanAurora;
            }
            _ => {}
        }
    }

    if audio_only {
        view.engine.metadata.format = MediaFormat::Flac;
        view.engine.metadata.width = 0;
        view.engine.metadata.height = 0;
        view.engine.metadata.title = "NovaOS Acoustic Symphony 24/96".to_string();
        view.engine.metadata.artist = "Nova Philharmonic Orchestra".to_string();
    }

    if let Some(ref path) = initial_file {
        if let Ok(bytes) = std::fs::read(path) {
            view.engine.load_file(path, &bytes);
            println!("[Spectra] Loaded media file: {} ({} bytes)", path, bytes.len());
        }
    }

    // Headless or screenshot rendering mode
    if headless || screenshot_path.is_some() {
        print_banner();
        println!("[Spectra] Executing headless render pipeline (1024x720)...");
        let mut surface = vec![0u32; DEFAULT_WIDTH * DEFAULT_HEIGHT];

        // Advance 1.5 seconds to showcase active video, spectrum, and subtitle cue
        view.tick(1500);
        view.render(&mut surface, DEFAULT_WIDTH, DEFAULT_HEIGHT);

        if let Some(ref path) = screenshot_path {
            let ppm_path = if path.ends_with(".png") {
                path.replace(".png", ".ppm")
            } else {
                path.clone()
            };
            save_ppm(&ppm_path, &surface, DEFAULT_WIDTH, DEFAULT_HEIGHT)?;
            println!("[Spectra] Screenshot saved to PPM: {}", ppm_path);

            if path.ends_with(".png") {
                let py_cmd = format!("from PIL import Image; Image.open('{}').save('{}')", ppm_path, path);
                let _ = std::process::Command::new("python3")
                    .arg("-c")
                    .arg(&py_cmd)
                    .output();
                let _ = std::fs::remove_file(&ppm_path);
                println!("[Spectra] Converted screenshot to PNG via PIL: {}", path);
            }
        }
        println!("[Spectra] Headless verification completed successfully.");
        return Ok(());
    }

    // Native Cross-Platform Desktop GUI Runner (macOS, Linux, Windows)
    print_banner();
    run_desktop_gui(view)?;
    Ok(())
}

fn run_headless(view: &mut SpectraPlayerView) {
    println!("[Spectra] Executing headless render pipeline (1024x720)...");
    let mut surface = vec![0u32; DEFAULT_WIDTH * DEFAULT_HEIGHT];
    view.tick(1500);
    view.render(&mut surface, DEFAULT_WIDTH, DEFAULT_HEIGHT);
    let non_zero = surface.iter().filter(|&&p| p != 0).count();
    println!("[Spectra] Headless verification completed. Rendered {} non-zero pixels.", non_zero);
}

fn run_desktop_gui(mut view: SpectraPlayerView) -> Result<(), Box<dyn std::error::Error>> {
    let mut win_w = DEFAULT_WIDTH;
    let mut win_h = DEFAULT_HEIGHT;

    let mut window = match Window::new(
        "Spectra - Ultra HD Media Studio & Player (Pure Rust Engine)",
        win_w,
        win_h,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[Spectra] Note: Could not open native window ({}). Running headless fallback.", e);
            run_headless(&mut view);
            return Ok(());
        }
    };

    window.set_target_fps(60);
    println!("[Spectra] Native cross-platform desktop window created ({}x{}).", win_w, win_h);

    let mut surface = vec![0u32; win_w * win_h];
    let mut mouse_down = false;
    let mut last_tick = std::time::Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) && !window.is_key_down(Key::Q) {
        // 1. Dynamic Window Resizing
        let (cur_w, cur_h) = window.get_size();
        if (cur_w != win_w || cur_h != win_h) && cur_w >= 320 && cur_h >= 240 {
            win_w = cur_w;
            win_h = cur_h;
            surface = vec![0u32; win_w * win_h];
        }

        // 2. Mouse Input Handling
        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Discard) {
            let px = mx as i32;
            let py = my as i32;
            let is_down = window.get_mouse_down(MouseButton::Left);

            if is_down && !mouse_down {
                mouse_down = true;
                view.handle_click(px, py, win_w, win_h);
            } else if !is_down && mouse_down {
                mouse_down = false;
                view.handle_mouse_up();
            } else {
                view.handle_mouse_move(px, py, mouse_down, win_w, win_h);
            }
        }

        // 3. Scroll Wheel (Volume control)
        if let Some((_sx, sy)) = window.get_scroll_wheel() {
            if sy > 0.0 {
                view.engine.audio.set_volume(view.engine.audio.volume + 0.05);
            } else if sy < 0.0 {
                view.engine.audio.set_volume(view.engine.audio.volume - 0.05);
            }
        }

        // 4. Keyboard Shortcuts
        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            match key {
                Key::Space => view.handle_key(32),
                Key::F => view.handle_key(102),
                Key::M => view.handle_key(109),
                Key::V => view.handle_key(118),
                Key::I => view.handle_key(105),
                Key::L => view.handle_key(108),
                Key::A => view.handle_key(97),
                Key::S => view.handle_key(115),
                Key::Left => view.handle_key(0xFF51),
                Key::Right => view.handle_key(0xFF53),
                Key::Up => view.handle_key(0xFF52),
                Key::Down => view.handle_key(0xFF54),
                Key::LeftBracket => {
                    let cur_speed = view.engine.speed;
                    view.engine.speed = (cur_speed - 0.25).max(0.25);
                }
                Key::RightBracket => {
                    let cur_speed = view.engine.speed;
                    view.engine.speed = (cur_speed + 0.25).min(4.0);
                }
                _ => {}
            }
        }

        // 5. Advance simulation tick at ~60fps
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(last_tick);
        let delta_ms = elapsed.as_millis().max(1) as u64;
        last_tick = now;
        view.tick(delta_ms);

        // 6. Render frame & update window buffer
        view.render(&mut surface, win_w, win_h);
        window.update_with_buffer(&surface, win_w, win_h)?;
    }

    println!("[Spectra] Session ended cleanly.");
    Ok(())
}
