# Spectra: Cross-Platform VLC-Class Media Studio & Playback Engine

[![Rust](https://img.shields.io/badge/Rust-1.99.1-orange.svg)](https://www.rust-lang.org)
[![Cross-Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows%20%7C%20NovaOS-blue.svg)](#)
[![Subtitles](https://img.shields.io/badge/Subtitles-SRT%20%7C%20VTT%20%7C%20ASS%20%7C%20SUB-success.svg)](#)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

**Spectra v2.0.0** is a fast, lightweight, cross-platform VLC-grade media player and studio engine written in **Rust**, supporting **Linux (X11/Wayland)**, **macOS (Cocoa/Metal/CoreAudio)**, **Windows (Win32/WASAPI)**, and **NovaOS** (`SPECTRA.ELF` + Desktop Window 9).

---

## 🌟 Key Capabilities

- **Cross-Platform Windowing & Native Dialogs**: Built on `winit` + `softbuffer` + `rfd` + `arboard` for native OS File, Multiple Files, Folder, Save Playlist, Drag-and-Drop, Clipboard URL open, and Borderless Fullscreen across **Linux, macOS, and Windows**.
- **Universal Container & Codec Support**:
  - **Video**: `.mp4`, `.m4v`, `.mkv`, `.webm`, `.avi`, `.mov`, `.flv`, `.ts`, `.m2ts`, `.mpg`, `.mpeg`, `.wmv`, `.3gp`, `.ogv`, `.vob` + image viewer (`.png`, `.jpg`, `.jpeg`, `.webp`, `.bmp`, `.gif`).
  - **Audio**: `.mp3`, `.flac`, `.wav`, `.ogg`, `.opus`, `.aac`, `.m4a`, `.alac`, `.aiff`, `.ac3`, `.eac3`, `.wma`.
  - **Network & Playlists**: HTTP/HTTPS/HLS (`.m3u8`)/RTSP streams and `.m3u` / `.m3u8` playlist import & export.
  - **All Resolutions**: SD, 720p HD, 1080p FHD, 1440p QHD, 4K UHD (`3840x2160`), and 8K FUHD (`7680x4320`) with auto-throttled decoding and bilinear/aspect framing.
- **Zero-Config Audio + Universal Codec Pack**:
  - Instant **Pure-Rust audio** decoding via Symphonia + Opus and **Pure-Rust H.264/MP4** fallback (`mp4` + `rusty_h264-decoder`).
  - Seamless **FFmpeg Universal Backend** (`Tools -> Download Universal Codec Pack`) for all HEVC, AV1, VP9, MKV, AVI, WebM, MOV, and network streams without reinstalling the app.
  - **Zero C-Header Linux Audio**: Dynamically loads PulseAudio (`libpulse-simple.so.0`) or ALSA (`libasound.so.2`) at runtime on Linux, and uses `cpal` CoreAudio/WASAPI on macOS/Windows.
- **Real 10-Band Biquad EQ & 32-Band FFT Visualizer**:
  - Real 10-band biquad peaking IIR equalizer (`31Hz..16kHz`) with presets, **200% preamp boost**, and soft-knee limiter.
  - Real 512-point Hann-windowed radix-2 FFT spectrum analyzer, oscilloscope waveform, and embedded album cover art vinyl turntable.
- **Full Subtitle & Sync Studio**:
  - Parses **SubRip (`.srt`)**, **WebVTT (`.vtt`)**, **SubStation Alpha (`.ass`/`.ssa`)**, and **MicroDVD (`.sub`)**.
  - Automatic sidecar subtitle discovery + embedded stream extraction, real-time delay offset (`G`/`H`), and font scaling.
- **Complete VLC Transport & Video Effects**:
  - A-B loop points (`L`), single-frame stepping (`E`), PNG snapshot capture (`Shift+S`), speed (`0.25x..4.0x`), aspect ratios, crop modes, rotate 90°/180°/270°, horizontal flip, deinterlace, hover timeline timestamp preview, recent history, and M3U8 export.
- **NovaOS System Integration**: Compiles cleanly into `#![no_std]` bare-metal ELF32 `SPECTRA.ELF` with real NovaOS VFS & AC97 WAV playback and interactive WM Window 9 transport/scrubbing/telemetry.

---

## 📂 Workspace Architecture

```
spectra-player/
├── Cargo.toml
├── crates/
│   ├── spectra-core/                # Demuxers, Symphonia/H.264/FFmpeg decoders, 10-band EQ & FFT DSP, Subtitles, M3U8 Playlist
│   ├── spectra-ui/                  # Bilinear/crop/rotate scaler, VLC menu bar, transport bar, Effects/EQ studio, Modals, OSD
│   └── spectra-app/                 # Cross-platform desktop host (winit + softbuffer + Pulse/ALSA/cpal + rfd + arboard + CLI)
└── novaos/
    ├── ffi.rs                       # NovaOS ring-3 syscalls (console, VFS read, AC97 audio mixer, heap)
    ├── player.rs                    # #![no_std] #[no_main] ELF32 executable for NovaOS
    └── build.sh                     # Cross-compiles and installs SPECTRA.ELF, PLAYER.ELF, NOVAPLAY.ELF
```

---

## 🚀 Building & Running

```bash
# Run Spectra Desktop Player
cargo run --release

# Open specific files, network streams, playlists, or subtitles
cargo run --release -- /path/to/movie.mkv --sub /path/to/subtitles.srt
cargo run --release -- --audio-only --visualizer --playlist /path/to/album/

# Headless verification & PNG snapshot
cargo run --release -- --info --screenshot verify.png

# Run workspace tests
cargo test --workspace
```

### Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Space` | Play / Pause |
| `S` | Stop |
| `Left` / `Right` | Seek backward / forward 5s (`Shift` for 1s, `Ctrl` for 30s) |
| `Up` / `Down` | Volume +/- 5% (up to 200% boost) |
| `M` | Mute / Unmute |
| `N` / `P` | Next / Previous track |
| `[` / `]` | Speed slower / faster |
| `E` | Step next frame |
| `L` | Set / clear A-B Loop |
| `F` / `F11` | Toggle Fullscreen |
| `Ctrl+O` / `Ctrl+N` / `Ctrl+D` | Open File / Network Stream / Folder |
| `Ctrl+E` | Audio & Video Effects Studio (10-Band EQ, Color, Sync) |
| `Ctrl+L` | Toggle Playlist Drawer |
| `Ctrl+J` | Jump to Specific Time |
| `Ctrl+I` / `Ctrl+H` | Codec Telemetry / Keyboard Shortcuts |
| `V` / `A` / `C` / `K` | Cycle Visualizer / Aspect Ratio / Crop / Subtitle Track |
| `G` / `H` | Subtitle delay -50ms / +50ms |
| `Shift+S` | Save PNG video snapshot |

---

## 💻 NovaOS Integration

```bash
# From novaos repository:
make spectra-test           # Runs spectra-player cargo test --workspace
make wm-test                # Runs NovaOS Window Manager desktop & WmPlayerApp tests
./tools/build-disk-image.sh # Packages SPECTRA.ELF into novaos.iso
```

---

## 📄 License

Licensed under the MIT License — see [LICENSE](LICENSE) for details.
