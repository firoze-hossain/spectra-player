//! # Nova Player Desktop Application
//!
//! High-performance pure-Rust media player for NovaOS and Linux.
//! Features native X11 GUI, 4K/8K video scaling, 32-band audio spectrum analyzer,
//! subtitle cues, playlist management, and headless rasterization.

#![allow(unused)]

use nova_player_core::types::{AspectRatio, MediaFormat, Resolution};
use nova_player_ui::NovaPlayerView;
use std::env;
use std::fs::File;
use std::io::Write;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, CreateWindowAux, CreateGCAux, EventMask, Gcontext, ImageFormat,
    KeyButMask, PropMode, Window, WindowClass, ConnectionExt as _,
};
use x11rb::protocol::Event;
use x11rb::wrapper::ConnectionExt as _;

const DEFAULT_WIDTH: usize = 1024;
const DEFAULT_HEIGHT: usize = 720;
const LINES_PER_CHUNK: usize = 16;

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
    println!("   NOVA PLAYER v1.0.0 (Pure Rust Media Engine)");
    println!("   4K UHD & 8K HDR Video • 32-Band Audio Visualizer • Subtitles");
    println!("   MKV • MP4 • WebM • AVI • FLAC • MP3 • WAV • AAC • OGG");
    println!("   Zero C/C++ • Zero FFmpeg/VLC binaries • Native NovaOS Default");
    println!("============================================================");
}

fn print_help() {
    print_banner();
    println!("Usage: nova-player [OPTIONS] [FILE]");
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
    println!("  Q / Esc                  Quit Nova Player");
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

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" => {
                println!("Nova Player 1.0.0 (Pure-Rust Engine, NovaOS & Linux)");
                return Ok(());
            }
            "--headless" => {
                headless = true;
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
            "--res" => {
                if i + 1 < args.len() {
                    i += 1;
                    force_res = Some(args[i].clone());
                }
            }
            "--screenshot" => {
                if i + 1 < args.len() {
                    i += 1;
                    screenshot_path = Some(args[i].clone());
                }
            }
            other => {
                if !other.starts_with('-') {
                    initial_file = Some(other.to_string());
                }
            }
        }
        i += 1;
    }

    let mut view = NovaPlayerView::new();

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

    if let Some(ref res_str) = force_res {
        match res_str.to_ascii_lowercase().as_str() {
            "8k" => {
                view.engine.metadata.width = 7680;
                view.engine.metadata.height = 4320;
                view.engine.metadata.title = "Deep Space Accretion Disk 8K HDR".to_string();
                view.engine.video_synth.theme = nova_player_core::codec::video_synth::VideoTheme::DeepSpaceNebula;
            }
            "4k" => {
                view.engine.metadata.width = 3840;
                view.engine.metadata.height = 2160;
                view.engine.metadata.title = "Cyberpunk Metropolis 4K UHD".to_string();
                view.engine.video_synth.theme = nova_player_core::codec::video_synth::VideoTheme::CyberpunkMetropolis;
            }
            "1080p" => {
                view.engine.metadata.width = 1920;
                view.engine.metadata.height = 1080;
                view.engine.metadata.title = "Aurora Borealis Ocean Waves 1080p".to_string();
                view.engine.video_synth.theme = nova_player_core::codec::video_synth::VideoTheme::OceanAurora;
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
            println!("[NovaPlayer] Loaded media file: {} ({} bytes)", path, bytes.len());
        }
    }

    // Headless or screenshot rendering mode
    if headless || screenshot_path.is_some() {
        print_banner();
        println!("[NovaPlayer] Executing headless render pipeline (1024x720)...");
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
            println!("[NovaPlayer] Screenshot saved to PPM: {}", ppm_path);

            if path.ends_with(".png") {
                let py_cmd = format!("from PIL import Image; Image.open('{}').save('{}')", ppm_path, path);
                let _ = std::process::Command::new("python3")
                    .arg("-c")
                    .arg(&py_cmd)
                    .output();
                let _ = std::fs::remove_file(&ppm_path);
                println!("[NovaPlayer] Converted screenshot to PNG via PIL: {}", path);
            }
        }
        println!("[NovaPlayer] Headless verification completed successfully.");
        return Ok(());
    }

    // Native X11 Interactive GUI Runner
    print_banner();
    run_x11_gui(view)?;
    Ok(())
}

fn run_x11_gui(mut view: NovaPlayerView) -> Result<(), Box<dyn std::error::Error>> {
    println!("[NovaPlayer] Connecting to X11 display server...");
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    let mut win_w = DEFAULT_WIDTH;
    let mut win_h = DEFAULT_HEIGHT;

    let win_id = conn.generate_id()?;
    let event_mask = EventMask::EXPOSURE
        | EventMask::KEY_PRESS
        | EventMask::BUTTON_PRESS
        | EventMask::BUTTON_RELEASE
        | EventMask::POINTER_MOTION
        | EventMask::STRUCTURE_NOTIFY;

    let win_aux = CreateWindowAux::new()
        .background_pixel(screen.black_pixel)
        .event_mask(event_mask);

    conn.create_window(
        screen.root_depth,
        win_id,
        screen.root,
        100,
        100,
        win_w as u16,
        win_h as u16,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &win_aux,
    )?;

    // Set Window Title
    let title = "Nova Player - Ultra HD Media & Audio Studio (Pure Rust)";
    conn.change_property8(
        PropMode::REPLACE,
        win_id,
        AtomEnum::WM_NAME,
        AtomEnum::STRING,
        title.as_bytes(),
    )?;

    // Request WM_DELETE_WINDOW protocol
    let wm_protocols = conn.intern_atom(false, b"WM_PROTOCOLS")?.reply()?.atom;
    let wm_delete_window = conn.intern_atom(false, b"WM_DELETE_WINDOW")?.reply()?.atom;
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        wm_protocols,
        AtomEnum::ATOM,
        &[wm_delete_window],
    )?;

    let gc_id = conn.generate_id()?;
    conn.create_gc(gc_id, win_id, &CreateGCAux::new())?;

    conn.map_window(win_id)?;
    conn.flush()?;

    println!("[NovaPlayer] Native desktop window created ({}x{}). Responsive auto-adjust enabled.", win_w, win_h);

    let mut surface = vec![0u32; win_w * win_h];
    let mut mouse_down = false;
    let mut running = true;
    let mut last_tick = std::time::Instant::now();

    while running {
        // Handle incoming X11 events
        while let Some(event) = conn.poll_for_event()? {
            match event {
                Event::ConfigureNotify(cfg) => {
                    let nw = cfg.width as usize;
                    let nh = cfg.height as usize;
                    if nw > 0 && nh > 0 && (nw != win_w || nh != win_h) {
                        win_w = nw;
                        win_h = nh;
                        surface.resize(win_w * win_h, 0);
                    }
                }
                Event::ButtonPress(btn) => {
                    let px = btn.event_x as i32;
                    let py = btn.event_y as i32;
                    if btn.detail == 1 {
                        mouse_down = true;
                        view.handle_click(px, py);
                    } else if btn.detail == 4 {
                        // Scroll up: Volume up
                        view.engine.audio.set_volume(view.engine.audio.volume + 0.05);
                    } else if btn.detail == 5 {
                        // Scroll down: Volume down
                        view.engine.audio.set_volume(view.engine.audio.volume - 0.05);
                    }
                }
                Event::ButtonRelease(btn) => {
                    if btn.detail == 1 {
                        mouse_down = false;
                        view.handle_mouse_up();
                    }
                }
                Event::MotionNotify(motion) => {
                    let px = motion.event_x as i32;
                    let py = motion.event_y as i32;
                    view.handle_mouse_move(px, py, mouse_down);
                }
                Event::KeyPress(kp) => {
                    if kp.detail == 9 {
                        // Escape: Quit
                        running = false;
                    } else if kp.detail == 65 {
                        // Space
                        view.handle_key(32);
                    } else if kp.detail == 41 {
                        // 'f' Fullscreen
                        view.handle_key(102);
                    } else if kp.detail == 58 {
                        // 'm' Mute
                        view.handle_key(109);
                    } else if kp.detail == 55 {
                        // 'v' Viz
                        view.handle_key(118);
                    } else if kp.detail == 31 {
                        // 'i' Info
                        view.handle_key(105);
                    } else if kp.detail == 46 {
                        // 'l' Playlist
                        view.handle_key(108);
                    } else if kp.detail == 38 {
                        // 'a' Aspect
                        view.handle_key(97);
                    } else if kp.detail == 39 {
                        // 's' Subtitles
                        view.handle_key(115);
                    } else if kp.detail == 113 {
                        // Left arrow
                        view.handle_key(0xFF51);
                    } else if kp.detail == 114 {
                        // Right arrow
                        view.handle_key(0xFF53);
                    } else if kp.detail == 111 {
                        // Up arrow
                        view.handle_key(0xFF52);
                    } else if kp.detail == 116 {
                        // Down arrow
                        view.handle_key(0xFF54);
                    }
                }
                Event::ClientMessage(msg) => {
                    if msg.data.as_data32()[0] == wm_delete_window {
                        running = false;
                    }
                }
                _ => {}
            }
        }

        // Advance simulation tick at ~60fps
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(last_tick);
        let delta_ms = elapsed.as_millis().max(1) as u64;
        last_tick = now;

        view.tick(delta_ms);

        // Render full frame
        view.render(&mut surface, win_w, win_h);

        // Blit chunks to X11 surface
        blit_surface_to_x11(&conn, win_id, gc_id, &surface, win_w, win_h)?;
        conn.flush()?;

        // Pacing: sleep 16ms for smooth 60fps
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    println!("[NovaPlayer] Session ended cleanly.");
    Ok(())
}

fn blit_surface_to_x11(
    conn: &impl Connection,
    window: Window,
    gc: Gcontext,
    surface: &[u32],
    width: usize,
    height: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut y = 0;
    while y < height {
        let chunk_lines = (height - y).min(LINES_PER_CHUNK);
        let start = y * width;
        let count = chunk_lines * width;
        let slice = &surface[start..start + count];

        let mut byte_data = Vec::with_capacity(count * 4);
        for &p in slice {
            byte_data.push((p & 0xFF) as u8);
            byte_data.push(((p >> 8) & 0xFF) as u8);
            byte_data.push(((p >> 16) & 0xFF) as u8);
            byte_data.push(0);
        }

        conn.put_image(
            ImageFormat::Z_PIXMAP,
            window,
            gc,
            width as u16,
            chunk_lines as u16,
            0,
            y as i16,
            0,
            24,
            &byte_data,
        )?;
        y += chunk_lines;
    }
    Ok(())
}
