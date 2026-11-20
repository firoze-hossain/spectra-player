//! Core data types and metadata definitions for Nova Player.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFormat {
    Mp4,
    Mkv,
    Webm,
    Avi,
    Mov,
    Wav,
    Mp3,
    Flac,
    Aac,
    Ogg,
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
            MediaFormat::Wav => "WAV (Waveform Audio)",
            MediaFormat::Mp3 => "MP3 (MPEG Audio Layer III)",
            MediaFormat::Flac => "FLAC (Free Lossless Audio)",
            MediaFormat::Aac => "AAC (Advanced Audio Coding)",
            MediaFormat::Ogg => "OGG (Vorbis Audio)",
            MediaFormat::Unknown => "Unknown Media Format",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            MediaFormat::Mp4 => "MP4",
            MediaFormat::Mkv => "MKV",
            MediaFormat::Webm => "WEBM",
            MediaFormat::Avi => "AVI",
            MediaFormat::Mov => "MOV",
            MediaFormat::Wav => "WAV",
            MediaFormat::Mp3 => "MP3",
            MediaFormat::Flac => "FLAC",
            MediaFormat::Aac => "AAC",
            MediaFormat::Ogg => "OGG",
            MediaFormat::Unknown => "MEDIA",
        }
    }

    pub fn is_audio_only(&self) -> bool {
        matches!(
            self,
            MediaFormat::Wav | MediaFormat::Mp3 | MediaFormat::Flac | MediaFormat::Aac | MediaFormat::Ogg
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
            AspectRatio::Ratio21_9 => "21:9 (Ultrawide)",
            AspectRatio::Ratio1_1 => "1:1",
            AspectRatio::Fill => "Fill / Stretch",
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
    /// 32-bit XRGB frame pixels (0x00RRGGBB)
    pub buffer: Vec<u32>,
}

impl VideoFrame {
    pub fn new(width: u32, height: u32, timestamp_ms: u64) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            timestamp_ms,
            buffer: vec![0xFF0F172A; size], // Default dark slate
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub format: MediaFormat,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub video_codec: String,
    pub audio_codec: String,
    pub bitrate_kbps: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub fps: f32,
}

impl Default for MediaMetadata {
    fn default() -> Self {
        Self {
            title: "NovaOS High-Fidelity 4K Demo".to_string(),
            artist: "Nova Media Studio".to_string(),
            album: "NovaOS Enterprise Media Suite".to_string(),
            format: MediaFormat::Mkv,
            duration_ms: 180_000, // 3 minutes
            width: 3840,
            height: 2160,
            video_codec: "H.264 / AVC High Profile (4K UHD)".to_string(),
            audio_codec: "FLAC 24-bit 96kHz Lossless".to_string(),
            bitrate_kbps: 45000,
            sample_rate: 48000,
            channels: 2,
            fps: 60.0,
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
        }
    }

    /// 8 frequency bands gains in dB: [64Hz, 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz, 8kHz]
    pub fn gains_db(&self) -> [f32; 8] {
        match self {
            EqualizerPreset::Flat => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            EqualizerPreset::Rock => [4.5, 3.0, -1.0, -2.0, 1.0, 2.5, 4.0, 5.0],
            EqualizerPreset::Pop => [-1.0, 1.5, 3.0, 4.0, 3.5, 1.0, -1.0, -1.5],
            EqualizerPreset::Classical => [4.0, 3.0, 2.0, 1.5, -1.0, 1.5, 3.0, 3.5],
            EqualizerPreset::BassBoost => [7.0, 6.0, 4.5, 2.0, 0.0, 0.0, 0.0, 0.0],
            EqualizerPreset::Vocal => [-2.0, -1.0, 1.0, 3.5, 4.0, 3.5, 1.5, 0.0],
            EqualizerPreset::Techno => [5.5, 4.5, 0.0, -2.0, -1.5, 0.0, 4.0, 5.5],
            EqualizerPreset::NovaAcoustic => [3.0, 2.0, 1.0, 0.0, 2.0, 3.5, 4.5, 4.0],
        }
    }
}
