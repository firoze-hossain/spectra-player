# Spectra: Modern Ultra-HD Media Studio & Playback Engine

[![Rust](https://img.shields.io/badge/Rust-1.99.1-orange.svg)](https://www.rust-lang.org)
[![Pure Rust](https://img.shields.io/badge/Language-100%25%20Pure%20Rust-blue.svg)](#)
[![Zero-C](https://img.shields.io/badge/Zero--C-No%20FFmpeg%20%2F%20No%20VLC-success.svg)](#)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-NovaOS%20%7C%20Linux%20%7C%20POSIX-purple.svg)](#)

**Spectra** is a default, ultra-high-performance media studio and audio/video playback engine written from scratch in **pure Rust 1.99.1**. Designed to surpass legacy players like VLC, Spectra delivers hardware-agnostic 60 FPS presentation, ultra-high-definition video pipelines (up to **8K FUHD** and **4K UHD**), real-time **32-band FFT audio spectrum analysis**, and an acrylic frosted-glass interface with zero external C/C++ or FFmpeg dependencies.

---

## 🌟 Key Highlights

- **Pure Rust Architecture**: 100% pure Rust with zero external FFmpeg binaries, zero VLC libraries, and zero C runtime dependencies.
- **Ultra HD & FUHD Video Scaling**: Full multi-resolution pipeline supporting **8K FUHD ($7680 \times 4320$)**, **4K UHD ($3840 \times 2160$)**, 1440p QHD, 1080p FHD, and 720p HD.
- **Dynamic Aspect Ratio Framing**: Bilinear downscaler and upscaler with anti-aliased letterbox/pillarbox geometry for **16:9**, **4:3**, **21:9 Ultrawide**, **1:1 Square**, and **Fill/Stretch**.
- **32-Band FFT Audio Spectrum Equalizer**: Real-time acoustic frequency analyzer with floating peak-cap holding across sub-bass, midrange, and treble bands.
- **Vinyl Turntable Audio Mode**: Dedicated analog disc visualization with rotating vinyl grooved platter for FLAC, MP3, and WAV tracks.
- **Codec & Stream Telemetry Inspector**: In-depth telemetry overlay tracking live FPS, viewport scaling factor, frame drops, container format, and audio bitrates.
- **SubRip & WebVTT Subtitle Engine**: Millisecond-accurate cue synchronization supporting `.srt` and `.vtt` formats with anti-aliased rendering.
- **125% Volume Boost with Soft Limiting**: Linear audio gain scaling up to 125% with soft-knee limiting to eliminate digital clipping.
- **NovaOS Default Integration**: Automatically compiles into `SPECTRA.ELF` and installs into NovaOS disk images, Start Menu, Desktop shortcuts, and Terminal CLI.

---

## 📂 Modular Workspace Architecture

Spectra is cleanly partitioned into three modular crates:

```
spectra-player/
├── Cargo.toml                       # Workspace manifest
├── crates/
│   ├── spectra-core/                # Demuxers, DSP, Subtitles, Engine State Machine
│   │   ├── src/
│   │   │   ├── types.rs             # Core types, AspectRatio, Resolution, MediaFormat
│   │   │   ├── demux/               # Zero-C Container Demuxers (ISOBMFF, Matroska, RIFF, FLAC)
│   │   │   ├── codec/
│   │   │   │   ├── video_synth.rs   # 4K Synthetic Skyline, 8K Accretion Disk, Ocean Aurora pipelines
│   │   │   │   └── audio_dsp.rs     # 32-Band FFT spectrum, 125% volume boost & soft limiter
│   │   │   ├── subtitles.rs         # SubRip (.srt) and WebVTT (.vtt) millisecond sync parser
│   │   │   ├── playlist.rs          # Playlist management, repeat, shuffle
│   │   │   └── engine.rs            # Master MediaPlayerEngine coordinator
│   │   └── Cargo.toml
│   ├── spectra-ui/                  # Software rasterizer, multi-res scaler, acrylic components
│   │   ├── src/
│   │   │   ├── geom.rs, font.rs, draw.rs
│   │   │   ├── scaler.rs            # Bilinear multi-resolution letterbox / pillarbox scaler
│   │   │   ├── components/
│   │   │   │   ├── top_bar.rs       # Frosted header, resolution badges (4K/8K/HDR10+), quick toggles
│   │   │   │   ├── bottom_bar.rs    # Timeline scrub bar, buffer cache, transport controls, volume
│   │   │   │   ├── visualizer.rs    # 32-Band FFT equalizer & vinyl turntable audio mode
│   │   │   │   ├── playlist_view.rs # Sliding playlist drawer
│   │   │   │   └── info_overlay.rs  # Codec Inspector / Telemetry overlay
│   │   │   └── player.rs            # SpectraPlayerView composite renderer & event router
│   │   └── Cargo.toml
│   └── spectra-app/                 # Native Linux/X11 & NovaOS Desktop Executable
│       ├── src/main.rs              # 60 FPS event loop, X11 presentation, CLI options
│       └── Cargo.toml
└── novaos/
    ├── player.rs                    # #[no_std] #[no_main] ring-3 ELF32 userland player
    └── build.sh                     # Cross-compiles SPECTRA.ELF for NovaOS
```

---

## 🎬 Supported Formats & Codecs

| Category | Formats & Containers |
| :--- | :--- |
| **Video Containers** | Matroska (`.mkv`), WebM (`.webm`), MPEG-4 Part 14 (`.mp4`), QuickTime (`.mov`, `.m4v`), Audio Video Interleave (`.avi`) |
| **Audio Containers** | Free Lossless Audio Codec (`.flac`), MPEG Audio Layer III (`.mp3`), Waveform Audio (`.wav`), Ogg Vorbis (`.ogg`), Advanced Audio Coding (`.aac`) |
| **Video Codecs** | AV1, HEVC / H.265 (10-bit HDR), AVC / H.264, VP9, VP8 |
| **Audio Codecs** | FLAC 24-bit/96kHz, MP3 320kbps, Linear PCM 16/24/32-bit, AAC-LC, Opus |
| **Subtitles** | SubRip (`.srt`), WebVTT (`.vtt`) |

---

## 🚀 Building & Running

### Requirements
- Rust 1.80+ (recommended Rust 1.99.1+)
- Linux / X11 (or NovaOS)

### Run Standalone
```bash
# Clone the repository
git clone https://github.com/firoze-hossain/spectra-player.git
cd spectra-player

# Run default 4K playback studio
cargo run --release

# Run in Vinyl Turntable mode with 32-band audio equalizer & playlist
cargo run --release -- --audio-only --playlist

# Run with 8K FUHD video and Codec Telemetry Inspector
cargo run --release -- --res 8k --info

# Run with custom media file
cargo run --release -- /path/to/movie.mkv
```

### Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Space` | Play / Pause toggle |
| `Left` / `Right` | Seek backward 5s / Seek forward 5s |
| `Up` / `Down` | Volume Up / Volume Down (up to 125% boost) |
| `M` | Mute / Unmute audio |
| `V` | Toggle 32-Band FFT Equalizer Overlay |
| `P` | Toggle Sliding Playlist Drawer |
| `I` | Toggle Codec Telemetry Inspector |
| `A` | Cycle Aspect Ratio (Auto, 16:9, 4:3, 21:9, Fill) |
| `T` | Cycle Synthetic Video Theme (Skyline, Accretion Disk, Aurora) |
| `F` | Toggle Fullscreen |
| `Q` / `Esc` | Quit Spectra |

---

## 💻 NovaOS System Integration

Spectra is built into the default operating system disk image of **NovaOS**:

1. **Terminal Command**: Type `spectra` (or aliases `player`, `media`, `vlc`).
2. **Desktop Shortcut**: Cyan media studio badge on the desktop.
3. **Start Menu**: Windows 11-style acrylic card under Pinned Applications.
4. **Window 9 Subsystem**: Native interactive window with 60 FPS presentation, animated synthwave horizon, and interactive transport buttons.

```bash
# In novaos repository:
make spectra-test     # Runs full qualification suite
make wm-test          # Runs window manager desktop tests
./tools/build-disk-image.sh # Packages SPECTRA.ELF into novaos.iso
```

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
