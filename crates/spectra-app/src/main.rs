//! # Spectra Desktop Application
//!
//! Cross-platform media studio and player for Linux, macOS, Windows, and NovaOS.
//! Built with native winit + softbuffer windowing, drag-and-drop, native file dialogs,
//! real audio output, multi-resolution video decoding, 10-band EQ, and subtitles.

#![allow(unused)]

mod audio_out;

use spectra_core::demux::detect::{AUDIO_EXTENSIONS, PLAYLIST_EXTENSIONS, SUBTITLE_EXTENSIONS, VIDEO_EXTENSIONS};
use spectra_core::types::MediaFormat;
use spectra_ui::components::input_modal::ActiveModal;
use spectra_ui::{SpectraPlayerView, UiCommand};

use std::env;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

const DEFAULT_WIDTH: usize = 1120;
const DEFAULT_HEIGHT: usize = 740;

fn save_png_or_ppm(path: &str, surface: &[u32], width: usize, height: usize) -> Result<(), Box<dyn std::error::Error>> {
    if path.to_ascii_lowercase().ends_with(".ppm") {
        let mut f = std::fs::File::create(path)?;
        use std::io::Write;
        write!(f, "P6\n{} {}\n255\n", width, height)?;
        let mut bytes = Vec::with_capacity(width * height * 3);
        for &p in surface {
            bytes.push(((p >> 16) & 0xFF) as u8);
            bytes.push(((p >> 8) & 0xFF) as u8);
            bytes.push((p & 0xFF) as u8);
        }
        f.write_all(&bytes)?;
        return Ok(());
    }

    let mut rgba = Vec::with_capacity(width * height * 4);
    for &p in surface {
        rgba.push(((p >> 16) & 0xFF) as u8);
        rgba.push(((p >> 8) & 0xFF) as u8);
        rgba.push((p & 0xFF) as u8);
        rgba.push(255);
    }
    let img = image::RgbaImage::from_raw(width as u32, height as u32, rgba)
        .ok_or("Invalid surface dimensions")?;
    img.save(path)?;
    Ok(())
}

fn print_banner() {
    println!("============================================================");
    println!("   SPECTRA v2.0.0 (Dynamic Cross-Platform Media Studio)");
    println!("   4K UHD & 8K Video • Real 10-Band EQ & FFT • Subtitles");
    println!("   MKV • MP4 • WebM • AVI • MOV • FLV • TS • FLAC • MP3 • OPUS");
    println!("   macOS • Linux • Windows • NovaOS Default Media Player");
    println!("============================================================");
}

fn print_help() {
    print_banner();
    println!("Usage: spectra [OPTIONS] [FILE_OR_URL...]");
    println!();
    println!("Options:");
    println!("  --help, -h               Print this help message");
    println!("  --version, -v            Print player version");
    println!("  --info                   Open with Media Info & Codec Inspector active");
    println!("  --playlist               Open with Playlist drawer visible");
    println!("  --visualizer             Enable 32-Band real-time Audio Spectrum Equalizer");
    println!("  --sub <path>             Load external subtitle (.srt/.vtt/.ass/.sub)");
    println!("  --fullscreen             Launch in fullscreen mode");
    println!("  --headless               Run headless rasterization test without GUI");
    println!("  --screenshot <path>      Render frame and save to PNG/PPM");
    println!("============================================================");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let mut initial_files: Vec<String> = Vec::new();
    let mut initial_sub: Option<String> = None;
    let mut headless = false;
    let mut screenshot_path: Option<String> = None;
    let mut open_info = false;
    let mut open_playlist = false;
    let mut open_viz = false;
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
                println!("Spectra 2.0.0 (Dynamic Cross-Platform Engine: Linux, macOS, Windows & NovaOS)");
                return Ok(());
            }
            "--info" => open_info = true,
            "--playlist" => open_playlist = true,
            "--visualizer" | "--audio-only" => open_viz = true,
            "--fullscreen" => fullscreen = true,
            "--headless" => headless = true,
            "--about" => open_about = true,
            "--sub" => {
                if i + 1 < args.len() {
                    initial_sub = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--menu" => {
                if i + 1 < args.len() {
                    open_menu = Some(args[i + 1].clone());
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
                initial_files.push(path.to_string());
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

    if initial_files.len() == 1 {
        view.engine.open_path(&initial_files[0]);
        println!("[Spectra] Opened media: {}", initial_files[0]);
    } else if initial_files.len() > 1 {
        view.engine.open_multiple_paths(&initial_files);
        println!("[Spectra] Queued {} media items.", initial_files.len());
    }

    if let Some(ref sub) = initial_sub {
        if let Err(e) = view.engine.load_subtitle_file(sub) {
            eprintln!("[Spectra] Warning loading subtitle: {e}");
        }
    }

    if headless || screenshot_path.is_some() {
        print_banner();
        println!("[Spectra] Executing headless render pipeline ({}x{})...", DEFAULT_WIDTH, DEFAULT_HEIGHT);
        let mut surface = vec![0u32; DEFAULT_WIDTH * DEFAULT_HEIGHT];
        if view.engine.metadata.is_live_file && !view.engine.metadata.format.is_audio_only() {
            let max_iters = if view.engine.codec_download_progress().is_some() { 1500 } else { 80 };
            for _ in 0..max_iters {
                view.tick(16);
                let got_frame = view
                    .engine
                    .session
                    .as_ref()
                    .and_then(|s| s.latest_frame.as_ref())
                    .is_some();
                if got_frame {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        } else {
            view.tick(1500);
        }
        view.render(&mut surface, DEFAULT_WIDTH, DEFAULT_HEIGHT);

        if let Some(ref path) = screenshot_path {
            save_png_or_ppm(path, &surface, DEFAULT_WIDTH, DEFAULT_HEIGHT)?;
            println!("[Spectra] Screenshot saved: {}", path);
        }
        println!("[Spectra] Headless verification completed.");
        return Ok(());
    }

    print_banner();
    if spectra_core::codec::media_backend::find_ffmpeg_bin().is_none() {
        view.engine.ensure_universal_codecs();
    }
    run_desktop_app(view)
}

struct SpectraDesktopApp {
    view: Arc<Mutex<SpectraPlayerView>>,
    window: Option<Arc<Window>>,
    context: Option<softbuffer::Context<Arc<Window>>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    win_w: usize,
    win_h: usize,
    mouse_x: i32,
    mouse_y: i32,
    mouse_down: bool,
    modifiers: ModifiersState,
    last_tick: Instant,
    tick_rem_ns: u128,
    next_redraw_at: Instant,
    last_title: String,
    _audio: Option<audio_out::AudioOutputHandle>,
}

impl SpectraDesktopApp {
    fn new(view: SpectraPlayerView) -> Self {
        let shared = Arc::new(Mutex::new(view));
        let audio = audio_out::start_audio_output(Arc::clone(&shared));
        if let Some(ref a) = audio {
            println!("[Spectra] Audio Output Online: {}", a.backend_name);
        } else {
            println!("[Spectra] Audio Output: Silent fallback (no OS audio device)");
        }
        let now = Instant::now();
        Self {
            view: shared,
            window: None,
            context: None,
            surface: None,
            win_w: DEFAULT_WIDTH,
            win_h: DEFAULT_HEIGHT,
            mouse_x: 0,
            mouse_y: 0,
            mouse_down: false,
            modifiers: ModifiersState::empty(),
            last_tick: now,
            tick_rem_ns: 0,
            next_redraw_at: now,
            last_title: String::new(),
            _audio: audio,
        }
    }

    fn process_ui_commands(&mut self, event_loop: &ActiveEventLoop) {
        let cmds = {
            let Ok(mut v) = self.view.lock() else {
                return;
            };
            v.take_commands()
        };

        for cmd in cmds {
            match cmd {
                UiCommand::OpenFileDialog => {
                    let mut all_exts: Vec<&str> = Vec::new();
                    all_exts.extend_from_slice(VIDEO_EXTENSIONS);
                    all_exts.extend_from_slice(AUDIO_EXTENSIONS);
                    all_exts.extend_from_slice(PLAYLIST_EXTENSIONS);
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("All Media & Playlists", &all_exts)
                        .add_filter("Video Files", VIDEO_EXTENSIONS)
                        .add_filter("Audio Files", AUDIO_EXTENSIONS)
                        .pick_file()
                    {
                        if let Ok(mut v) = self.view.lock() {
                            v.engine.open_path(&file.to_string_lossy());
                        }
                    }
                }
                UiCommand::OpenMultipleDialog => {
                    let mut all_exts: Vec<&str> = Vec::new();
                    all_exts.extend_from_slice(VIDEO_EXTENSIONS);
                    all_exts.extend_from_slice(AUDIO_EXTENSIONS);
                    if let Some(files) = rfd::FileDialog::new()
                        .add_filter("Media Files", &all_exts)
                        .pick_files()
                    {
                        let paths: Vec<String> =
                            files.into_iter().map(|p| p.to_string_lossy().to_string()).collect();
                        if let Ok(mut v) = self.view.lock() {
                            v.engine.open_multiple_paths(&paths);
                        }
                    }
                }
                UiCommand::OpenDirectoryDialog => {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        if let Ok(mut v) = self.view.lock() {
                            v.engine.open_directory(&dir.to_string_lossy());
                        }
                    }
                }
                UiCommand::OpenSubtitleDialog => {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("Subtitle Files", SUBTITLE_EXTENSIONS)
                        .pick_file()
                    {
                        if let Ok(mut v) = self.view.lock() {
                            if let Err(e) = v.engine.load_subtitle_file(&file.to_string_lossy()) {
                                v.show_osd(e);
                            }
                        }
                    }
                }
                UiCommand::SavePlaylistDialog => {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("M3U8 Playlist", &["m3u8", "m3u"])
                        .set_file_name("spectra_playlist.m3u8")
                        .save_file()
                    {
                        let content = {
                            let Ok(v) = self.view.lock() else { continue };
                            v.engine.playlist.to_m3u8()
                        };
                        if std::fs::write(&file, content).is_ok() {
                            if let Ok(mut v) = self.view.lock() {
                                v.show_osd(format!("Saved playlist: {}", file.display()));
                            }
                        }
                    }
                }
                UiCommand::PasteFromClipboard => {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        if let Ok(txt) = cb.get_text() {
                            let trimmed = txt.trim().to_string();
                            if !trimmed.is_empty() {
                                if let Ok(mut v) = self.view.lock() {
                                    if matches!(v.active_modal, ActiveModal::OpenUrl | ActiveModal::JumpToTime) {
                                        v.handle_text_input(&trimmed);
                                    } else {
                                        v.engine.open_path(&trimmed);
                                    }
                                }
                            }
                        }
                    }
                }
                UiCommand::TakeSnapshot => {
                    if let Ok(mut v) = self.view.lock() {
                        match v.engine.take_snapshot() {
                            Ok(p) => v.show_osd(format!("Snapshot saved: {}", p.rsplit(['/', '\\']).next().unwrap_or(&p))),
                            Err(e) => v.show_osd(e),
                        }
                    }
                }
                UiCommand::ToggleOsFullscreen => {
                    if let Some(ref win) = self.window {
                        let fs = self.view.lock().map(|v| v.is_fullscreen).unwrap_or(false);
                        win.set_fullscreen(if fs { Some(Fullscreen::Borderless(None)) } else { None });
                    }
                }
                UiCommand::QuitApp => {
                    event_loop.exit();
                }
            }
        }
    }
}

impl ApplicationHandler for SpectraDesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Spectra - Ultra HD Media Studio & Player")
            .with_inner_size(LogicalSize::new(DEFAULT_WIDTH as u32, DEFAULT_HEIGHT as u32))
            .with_min_inner_size(LogicalSize::new(480, 320));
        let win = Arc::new(event_loop.create_window(attrs).expect("Create desktop window"));
        let is_fs = self.view.lock().map(|v| v.is_fullscreen).unwrap_or(false);
        if is_fs {
            win.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        let ctx = softbuffer::Context::new(Arc::clone(&win)).expect("Create softbuffer context");
        let mut surf = softbuffer::Surface::new(&ctx, Arc::clone(&win)).expect("Create softbuffer surface");
        let size = win.inner_size();
        self.win_w = size.width.max(320) as usize;
        self.win_h = size.height.max(240) as usize;
        let _ = surf.resize(
            NonZeroU32::new(self.win_w as u32).unwrap(),
            NonZeroU32::new(self.win_h as u32).unwrap(),
        );

        println!("[Spectra] Native cross-platform desktop window created ({}x{}).", self.win_w, self.win_h);
        self.window = Some(win);
        self.context = Some(ctx);
        self.surface = Some(surf);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(sz) => {
                if sz.width >= 320 && sz.height >= 240 {
                    self.win_w = sz.width as usize;
                    self.win_h = sz.height as usize;
                    if let Some(ref mut surf) = self.surface {
                        let _ = surf.resize(
                            NonZeroU32::new(sz.width).unwrap(),
                            NonZeroU32::new(sz.height).unwrap(),
                        );
                    }
                }
            }
            WindowEvent::DroppedFile(path) => {
                let p = path.to_string_lossy().to_string();
                if let Ok(mut v) = self.view.lock() {
                    v.engine.open_path(&p);
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as i32;
                self.mouse_y = position.y as i32;
                if let Ok(mut v) = self.view.lock() {
                    v.handle_mouse_move(self.mouse_x, self.mouse_y, self.mouse_down, self.win_w, self.win_h);
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.mouse_down = true;
                        if let Ok(mut v) = self.view.lock() {
                            v.handle_click(self.mouse_x, self.mouse_y, self.win_w, self.win_h);
                        }
                    }
                    ElementState::Released => {
                        self.mouse_down = false;
                        if let Ok(mut v) = self.view.lock() {
                            v.handle_mouse_up();
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y / 24.0) as f32,
                };
                if let Ok(mut v) = self.view.lock() {
                    v.handle_scroll(dy);
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let ctrl = self.modifiers.control_key() || self.modifiers.super_key();
                let shift = self.modifiers.shift_key();

                // Modal text input handling
                let in_text_modal = self
                    .view
                    .lock()
                    .map(|v| matches!(v.active_modal, ActiveModal::OpenUrl | ActiveModal::JumpToTime))
                    .unwrap_or(false);

                if in_text_modal {
                    match &event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            if let Ok(mut v) = self.view.lock() {
                                v.active_modal = ActiveModal::None;
                            }
                            return;
                        }
                        Key::Named(NamedKey::Enter) => {
                            if let Ok(mut v) = self.view.lock() {
                                v.handle_modal_submit();
                            }
                            return;
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if let Ok(mut v) = self.view.lock() {
                                v.handle_modal_backspace();
                            }
                            return;
                        }
                        Key::Character(c) if ctrl && c.eq_ignore_ascii_case("v") => {
                            if let Ok(mut v) = self.view.lock() {
                                v.pending_commands.push(UiCommand::PasteFromClipboard);
                            }
                            return;
                        }
                        _ => {
                            if let Some(txt) = &event.text {
                                if let Ok(mut v) = self.view.lock() {
                                    v.handle_text_input(txt);
                                }
                            }
                            return;
                        }
                    }
                }

                // Global Ctrl/Shift shortcuts
                if ctrl {
                    if let Key::Character(c) = &event.logical_key {
                        let lc = c.to_ascii_lowercase();
                        let mut v = self.view.lock().unwrap();
                        match lc.as_str() {
                            "o" if shift => v.pending_commands.push(UiCommand::OpenMultipleDialog),
                            "o" => v.pending_commands.push(UiCommand::OpenFileDialog),
                            "f" => v.pending_commands.push(UiCommand::OpenDirectoryDialog),
                            "n" => {
                                v.active_modal = ActiveModal::OpenUrl;
                                v.modal_input.clear();
                            }
                            "v" => v.pending_commands.push(UiCommand::PasteFromClipboard),
                            "y" => v.pending_commands.push(UiCommand::SavePlaylistDialog),
                            "s" if shift => v.pending_commands.push(UiCommand::OpenSubtitleDialog),
                            "e" => v.show_effects = !v.show_effects,
                            "i" => v.show_info = !v.show_info,
                            "l" => v.show_playlist = !v.show_playlist,
                            "t" => {
                                v.active_modal = ActiveModal::JumpToTime;
                                v.modal_input.clear();
                            }
                            "p" => v.active_modal = ActiveModal::Preferences,
                            "h" => v.show_controls = !v.show_controls,
                            "r" => {
                                let deg = (v.engine.video_synth.adjustments.rotate_deg + 90) % 360;
                                v.engine.video_synth.adjustments.rotate_deg = deg;
                                v.show_osd(format!("Rotation: {} deg", deg));
                            }
                            "q" => event_loop.exit(),
                            _ => {}
                        }
                        return;
                    }
                }

                // Standard single-key shortcuts
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        let mut v = self.view.lock().unwrap();
                        if v.active_modal != ActiveModal::None {
                            v.active_modal = ActiveModal::None;
                        } else if v.show_about {
                            v.show_about = false;
                        } else if v.show_effects {
                            v.show_effects = false;
                        } else if v.is_fullscreen {
                            v.is_fullscreen = false;
                            v.pending_commands.push(UiCommand::ToggleOsFullscreen);
                        } else {
                            event_loop.exit();
                        }
                    }
                    Key::Named(NamedKey::Space) => self.view.lock().unwrap().handle_key(32),
                    Key::Named(NamedKey::ArrowLeft) => self.view.lock().unwrap().handle_key(0xFF51),
                    Key::Named(NamedKey::ArrowRight) => self.view.lock().unwrap().handle_key(0xFF53),
                    Key::Named(NamedKey::ArrowUp) => self.view.lock().unwrap().handle_key(0xFF52),
                    Key::Named(NamedKey::ArrowDown) => self.view.lock().unwrap().handle_key(0xFF54),
                    Key::Named(NamedKey::F1) => self.view.lock().unwrap().handle_key(0xFFBE),
                    Key::Character(c) => {
                        if c.eq_ignore_ascii_case("q") {
                            event_loop.exit();
                        } else if c.eq_ignore_ascii_case("s") && shift {
                            self.view.lock().unwrap().pending_commands.push(UiCommand::TakeSnapshot);
                        } else if let Some(ch) = c.chars().next() {
                            self.view.lock().unwrap().handle_key(ch as u32);
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let elapsed_ns = now.duration_since(self.last_tick).as_nanos() + self.tick_rem_ns;
                self.last_tick = now;
                let delta_ms = (elapsed_ns / 1_000_000) as u64;
                self.tick_rem_ns = elapsed_ns % 1_000_000;

                let title_now = {
                    let mut v = self.view.lock().unwrap();
                    v.tick(delta_ms);
                    if v.engine.metadata.is_live_file {
                        format!("Spectra - {}", v.engine.metadata.title)
                    } else {
                        "Spectra Media Player".to_string()
                    }
                };

                if title_now != self.last_title {
                    if let Some(ref win) = self.window {
                        win.set_title(&title_now);
                    }
                    self.last_title = title_now;
                }

                if let Some(ref mut surf) = self.surface {
                    if let Ok(mut buf) = surf.buffer_mut() {
                        let w = self.win_w;
                        let h = self.win_h;
                        if buf.len() >= w * h {
                            if let Ok(mut v) = self.view.lock() {
                                v.render(&mut buf[..w * h], w, h);
                            }
                            let _ = buf.present();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_ui_commands(event_loop);
        let now = Instant::now();
        let frame_step = Duration::from_micros(16_667);
        if now >= self.next_redraw_at {
            self.next_redraw_at = (self.next_redraw_at + frame_step).max(now + Duration::from_millis(2));
            if let Some(ref win) = self.window {
                win.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_redraw_at));
    }
}

fn run_desktop_app(view: SpectraPlayerView) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = SpectraDesktopApp::new(view);
    event_loop.run_app(&mut app)?;
    println!("[Spectra] Session ended cleanly.");
    Ok(())
}
