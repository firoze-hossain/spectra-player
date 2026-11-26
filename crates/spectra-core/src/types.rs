//! Core data types and metadata definitions for Spectra Media Studio.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFormat {
    Mp4,
    Mkv,
    Webm,
    Avi,
    Mov,
    Flv,
    Ts,
    Mpeg,
    Wmv,
    ThreeGp,
    Ogv,
    Wav,
    Mp3,
    Flac,
    Aac,
    M4a,
    Ogg,
    Opus,
    Wma,
    Alac,
    Aiff,
    Ac3,
    NetworkStream,
    Unknown,
}

impl MediaFormat {
    pub fn name(&self) -> &'static str {
        match self {
            MediaFormat::Mp4 => "MP4 (MPEG-4 Part 14)",
            MediaFormat::Mkv => "MKV (Matroska Video)",
            MediaFormat::Webm => "WebM (Open Web Media)",
            MediaFormat::Avi => "AVI (Audio Video Interleave)",
            MediaFormat::Mov => "MOV (Apple QuickTime)",
            MediaFormat::Flv => "FLV (Flash Video)",
            MediaFormat::Ts => "MPEG-TS (Transport Stream)",
            MediaFormat::Mpeg => "MPEG-PS / VOB Video",
            MediaFormat::Wmv => "WMV / ASF Media",
            MediaFormat::ThreeGp => "3GP Mobile Video",
            MediaFormat::Ogv => "OGV (Ogg Theora Video)",
            MediaFormat::Wav => "WAV (Waveform Audio)",
            MediaFormat::Mp3 => "MP3 (MPEG Audio Layer III)",
            MediaFormat::Flac => "FLAC (Free Lossless Audio)",
            MediaFormat::Aac => "AAC (Advanced Audio Coding)",
            MediaFormat::M4a => "M4A (MPEG-4 Audio)",
            MediaFormat::Ogg => "OGG (Vorbis Audio)",
            MediaFormat::Opus => "Opus Interactive Audio",
            MediaFormat::Wma => "WMA (Windows Media Audio)",
            MediaFormat::Alac => "ALAC (Apple Lossless)",
            MediaFormat::Aiff => "AIFF (Audio Interchange)",
            MediaFormat::Ac3 => "AC3 / E-AC3 Dolby Audio",
            MediaFormat::NetworkStream => "Network Media Stream",
            MediaFormat::Unknown => "Media Container",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            MediaFormat::Mp4 => "MP4",
            MediaFormat::Mkv => "MKV",
            MediaFormat::Webm => "WEBM",
            MediaFormat::Avi => "AVI",
            MediaFormat::Mov => "MOV",
            MediaFormat::Flv => "FLV",
            MediaFormat::Ts => "TS",
            MediaFormat::Mpeg => "MPG",
            MediaFormat::Wmv => "WMV",
            MediaFormat::ThreeGp => "3GP",
            MediaFormat::Ogv => "OGV",
            MediaFormat::Wav => "WAV",
            MediaFormat::Mp3 => "MP3",
            MediaFormat::Flac => "FLAC",
            MediaFormat::Aac => "AAC",
            MediaFormat::M4a => "M4A",
            MediaFormat::Ogg => "OGG",
            MediaFormat::Opus => "OPUS",
            MediaFormat::Wma => "WMA",
            MediaFormat::Alac => "ALAC",
            MediaFormat::Aiff => "AIFF",
            MediaFormat::Ac3 => "AC3",
            MediaFormat::NetworkStream => "NET",
            MediaFormat::Unknown => "MEDIA",
        }
    }

    pub fn is_audio_only(&self) -> bool {
        matches!(
            self,
            MediaFormat::Wav
                | MediaFormat::Mp3
                | MediaFormat::Flac
                | MediaFormat::Aac
                | MediaFormat::M4a
                | MediaFormat::Ogg
                | MediaFormat::Opus
                | MediaFormat::Wma
                | MediaFormat::Alac
                | MediaFormat::Aiff
                | MediaFormat::Ac3
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Buffering,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspectRatio {
    Auto,
    Ratio16_9,
    Ratio4_3,
    Ratio21_9,
    Ratio1_1,
    Fill,
}

impl AspectRatio {
    pub fn name(&self) -> &'static str {
        match self {
            AspectRatio::Auto => "Auto",
            AspectRatio::Ratio16_9 => "16:9",
            AspectRatio::Ratio4_3 => "4:3",
            AspectRatio::Ratio21_9 => "21:9",
            AspectRatio::Ratio1_1 => "1:1",
            AspectRatio::Fill => "Fill",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropMode {
    None,
    Ratio16_9,
    Ratio4_3,
    Ratio21_9,
    Ratio1_1,
}

impl CropMode {
    pub fn name(&self) -> &'static str {
        match self {
            CropMode::None => "No Crop",
            CropMode::Ratio16_9 => "Crop 16:9",
            CropMode::Ratio4_3 => "Crop 4:3",
            CropMode::Ratio21_9 => "Crop 21:9",
            CropMode::Ratio1_1 => "Crop 1:1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioVisualizerMode {
    Off,
    Spectrum32,
    Waveform,
    VinylTurntable,
}

impl AudioVisualizerMode {
    pub fn name(&self) -> &'static str {
        match self {
            AudioVisualizerMode::Off => "Off",
            AudioVisualizerMode::Spectrum32 => "32-Band FFT Spectrum",
            AudioVisualizerMode::Waveform => "Oscilloscope Waveform",
            AudioVisualizerMode::VinylTurntable => "Vinyl Turntable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderBackend {
    Synthesizer,
    PureRustSymphonia,
    PureRustH264,
    FfmpegUniversal,
    HybridSymphoniaFfmpeg,
}

impl DecoderBackend {
    pub fn name(&self) -> &'static str {
        match self {
            DecoderBackend::Synthesizer => "Standby / Ready",
            DecoderBackend::PureRustSymphonia => "Pure-Rust Symphonia Audio",
            DecoderBackend::PureRustH264 => "Pure-Rust H.264 + Symphonia",
            DecoderBackend::FfmpegUniversal => "FFmpeg Universal A/V Engine",
            DecoderBackend::HybridSymphoniaFfmpeg => "FFmpeg Video + Symphonia Audio",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Res720p,
    Res1080p,
    Res1440p,
    Res4K,
    Res8K,
    Custom(u32, u32),
}

impl Resolution {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Resolution::Res720p => (1280, 720),
            Resolution::Res1080p => (1920, 1080),
            Resolution::Res1440p => (2560, 1440),
            Resolution::Res4K => (3840, 2160),
            Resolution::Res8K => (7680, 4320),
            Resolution::Custom(w, h) => (*w, *h),
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            Resolution::Res720p => "720p HD",
            Resolution::Res1080p => "1080p FHD",
            Resolution::Res1440p => "1440p 2K",
            Resolution::Res4K => "4K UHD",
            Resolution::Res8K => "8K FUHD",
            Resolution::Custom(w, _h) => {
                if *w >= 7000 {
                    "8K HDR"
                } else if *w >= 3500 {
                    "4K UHD"
                } else if *w >= 2400 {
                    "2K QHD"
                } else if *w >= 1800 {
                    "1080p FHD"
                } else if *w >= 1200 {
                    "720p HD"
                } else {
                    "SD/HD"
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub timestamp_ms: u64,
    /// 32-bit XRGB frame pixels (0xFFRRGGBB)
    pub buffer: Vec<u32>,
}

impl VideoFrame {
    pub fn new(width: u32, height: u32, timestamp_ms: u64) -> Self {
        let size = (width as usize).saturating_mul(height as usize);
        Self {
            width,
            height,
            timestamp_ms,
            buffer: vec![0xFF0F172A; size],
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamTrackInfo {
    pub index: usize,
    pub stream_index: usize,
    pub codec: String,
    pub language: String,
    pub title: String,
    pub channels: u8,
    pub sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct CoverArt {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct MediaMetadata {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub format: MediaFormat,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub video_codec: String,
    pub audio_codec: String,
    pub pixel_format: String,
    pub bitrate_kbps: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub fps: f32,
    pub is_live_file: bool,
    pub backend: DecoderBackend,
    pub audio_tracks: Vec<StreamTrackInfo>,
    pub subtitle_tracks: Vec<StreamTrackInfo>,
    pub cover_art: Option<CoverArt>,
}

impl Default for MediaMetadata {
    fn default() -> Self {
        Self {
            path: String::new(),
            title: "Spectra Media Studio".to_string(),
            artist: "Open a file, folder, or stream URL".to_string(),
            album: String::new(),
            format: MediaFormat::Unknown,
            duration_ms: 0,
            width: 0,
            height: 0,
            video_codec: "None".to_string(),
            audio_codec: "None".to_string(),
            pixel_format: String::new(),
            bitrate_kbps: 0,
            sample_rate: 48000,
            channels: 2,
            fps: 0.0,
            is_live_file: false,
            backend: DecoderBackend::Synthesizer,
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            cover_art: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqualizerPreset {
    Flat,
    Rock,
    Pop,
    Classical,
    BassBoost,
    Vocal,
    Techno,
    NovaAcoustic,
    Custom,
}

impl EqualizerPreset {
    pub fn name(&self) -> &'static str {
        match self {
            EqualizerPreset::Flat => "Flat",
            EqualizerPreset::Rock => "Rock",
            EqualizerPreset::Pop => "Pop",
            EqualizerPreset::Classical => "Classical",
            EqualizerPreset::BassBoost => "Bass Boost (+6dB)",
            EqualizerPreset::Vocal => "Vocal Clarity",
            EqualizerPreset::Techno => "Techno / Electronic",
            EqualizerPreset::NovaAcoustic => "Nova Acoustic Studio",
            EqualizerPreset::Custom => "Custom",
        }
    }

    /// 8 frequency bands gains in dB: [64Hz, 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz, 8kHz]
    pub fn gains_db(&self) -> [f32; 8] {
        match self {
            EqualizerPreset::Flat | EqualizerPreset::Custom => [0.0; 8],
            EqualizerPreset::Rock => [4.5, 3.0, -1.0, -2.0, 1.0, 2.5, 4.0, 5.0],
            EqualizerPreset::Pop => [-1.0, 1.5, 3.0, 4.0, 3.5, 1.0, -1.0, -1.5],
            EqualizerPreset::Classical => [4.0, 3.0, 2.0, 1.5, -1.0, 1.5, 3.0, 3.5],
            EqualizerPreset::BassBoost => [7.0, 6.0, 4.5, 2.0, 0.0, 0.0, 0.0, 0.0],
            EqualizerPreset::Vocal => [-2.0, -1.0, 1.0, 3.5, 4.0, 3.5, 1.5, 0.0],
            EqualizerPreset::Techno => [5.5, 4.5, 0.0, -2.0, -1.5, 0.0, 4.0, 5.5],
            EqualizerPreset::NovaAcoustic => [3.0, 2.0, 1.0, 0.0, 2.0, 3.5, 4.5, 4.0],
        }
    }

    /// 10-band VLC-style graphic equalizer gains in dB:
    /// [60Hz, 170Hz, 310Hz, 600Hz, 1kHz, 3kHz, 6kHz, 12kHz, 14kHz, 16kHz]
    pub fn gains_db_10(&self) -> [f32; 10] {
        match self {
            EqualizerPreset::Flat | EqualizerPreset::Custom => [0.0; 10],
            EqualizerPreset::Rock => [4.8, 3.4, -2.0, -3.5, -1.5, 2.2, 4.5, 5.5, 5.5, 5.0],
            EqualizerPreset::Pop => [-1.2, 2.4, 4.0, 4.2, 2.8, -0.5, -1.5, -1.5, -1.0, -1.0],
            EqualizerPreset::Classical => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -3.0, -3.0, -3.0, -4.5],
            EqualizerPreset::BassBoost => [7.5, 6.5, 4.5, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            EqualizerPreset::Vocal => [-2.5, -1.5, 0.5, 3.5, 4.5, 4.0, 2.0, 0.0, -0.5, -1.0],
            EqualizerPreset::Techno => [5.5, 4.5, 0.5, -2.5, -2.0, 0.5, 4.0, 5.5, 5.5, 4.5],
            EqualizerPreset::NovaAcoustic => [3.0, 2.2, 1.0, 0.0, 1.5, 3.0, 4.0, 4.2, 3.5, 2.5],
        }
    }
}
