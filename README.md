# Nova Player (Pure Rust Media Engine & Desktop Player)

> High-performance, lightweight, multi-format media player for NovaOS and Linux.
> Built from scratch in pure Rust 1.99.1 with zero C/C++, zero external FFmpeg, and zero VLC binaries.

---

## Key Features

- **Multi-Container Demuxing**:
  - Video: **MKV** (Matroska EBML), **MP4** (ISOBMFF), **WebM**, **AVI** (RIFF), **MOV**.
  - Audio: **FLAC** (Lossless 24/96), **MP3** (Layer III + ID3), **WAV** (PCM), **AAC**, **OGG** (Vorbis).
- **Ultra-High Resolution Video Pipeline**:
  - Dynamically adapts to any screen resolution and aspect ratio (16:9, 4:3, 21:9 Ultrawide, 1:1, Fill/Stretch).
  - High-performance SIMD/bilinear frame scaling with anti-aliased letterboxing and pillarboxing.
  - Native support for **4K UHD** (3840x2160) and **8K FUHD** (7680x4320) at smooth 60 FPS.
- **Audio DSP & 32-Band Spectrum Analyzer**:
  - 32-frequency-band real-time audio spectrum equalizer with peak holding and glass floor reflections.
  - Volume control with soft-saturation boost up to **125%** (VLC-style audio amplification without digital clipping).
  - 8-band Equalizer Presets: Flat, Rock, Pop, Classical, Bass Boost (+6dB), Vocal, Techno, Nova Acoustic.
- **Modern Acrylic UI**:
  - Translucent frosted glass control island with rounded pill controls.
  - Scrub bar with buffered progress, hover timecode tooltip, and playhead thumb.
  - Subtitle overlay (`[CC]`) with millisecond-accurate SRT / WebVTT cue synchronization.
  - Sliding media playlist drawer with track format badges and active track highlighting.
  - Real-time Media Info & Codec Telemetry ("Stats for Nerds") overlay.

---

## Workspace Structure

```
nova-player/
├── Cargo.toml                  # Workspace manifest
├── README.md                   # Documentation & specifications
├── crates/
│   ├── nova-player-core/       # Pure-Rust demuxers, decoders, DSP, and state machine
│   ├── nova-player-ui/         # Modern acrylic UI, video canvas, visualizer, widgets
│   └── nova-player-app/        # Native X11 desktop application and CLI runner
└── novaos/
    ├── build.sh                # Cross-compiler script for NovaOS ring-3 i686 ELF32
    └── player.rs               # NovaOS embedded player client module
```

---

## Keyboard Shortcuts

| Key | Action |
| :--- | :--- |
| `Space` | Play / Pause toggle |
| `Left` / `Right` | Seek backward / forward 5 seconds |
| `Up` / `Down` | Volume increase / decrease 5% |
| `M` | Mute / Unmute audio |
| `F` | Toggle Fullscreen mode |
| `V` | Toggle 32-Band Audio Spectrum Visualizer |
| `I` | Toggle Media Info & Codec Inspector |
| `L` | Toggle Media Playlist drawer |
| `A` | Cycle Aspect Ratio (Auto, 16:9, 4:3, 21:9, Fill) |
| `S` | Toggle Subtitles [CC] |
| `[` / `]` | Decrease / Increase playback speed (0.25x - 4.0x) |
| `Q` / `Esc` | Quit player |

---

## CLI Usage

```bash
# Launch default player with 4K UHD video demo
cargo run --release

# Open a specific video or audio file
cargo run --release -- /path/to/movie.mkv
cargo run --release -- /path/to/song.flac

# Launch with 32-Band Audio Visualizer enabled
cargo run --release -- --visualizer

# Launch in High-Resolution Audio Turntable mode
cargo run --release -- --audio-only

# Launch with Media Info & Codec Inspector active
cargo run --release -- --info

# Headless verification & screenshot rendering
cargo run --release -- --screenshot output.png
```
