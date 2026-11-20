//! Media Playlist manager with repeat, shuffle, and queue operations.

use crate::types::MediaFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    None,
    RepeatOne,
    RepeatAll,
}

#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub format: MediaFormat,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
}

impl PlaylistItem {
    pub fn new(path: &str, title: &str, artist: &str, format: MediaFormat, duration_ms: u64, width: u32, height: u32) -> Self {
        Self {
            path: path.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            format,
            duration_ms,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub items: Vec<PlaylistItem>,
    pub current_index: usize,
    pub loop_mode: LoopMode,
    pub is_shuffled: bool,
}

impl Default for Playlist {
    fn default() -> Self {
        Self {
            items: vec![
                PlaylistItem::new(
                    "video/cyberpunk_metropolis_4k.mkv",
                    "Cyberpunk Metropolis 4K UHD",
                    "Nova Visual Arts Studio",
                    MediaFormat::Mkv,
                    180_000,
                    3840,
                    2160,
                ),
                PlaylistItem::new(
                    "video/deep_space_blackhole_8k.mp4",
                    "Deep Space & Black Hole 8K FUHD",
                    "Cosmic Observation Lab",
                    MediaFormat::Mp4,
                    240_000,
                    7680,
                    4320,
                ),
                PlaylistItem::new(
                    "video/aurora_borealis_ocean_1080p.webm",
                    "Aurora Borealis Ocean Waves",
                    "Arctic Nature Expeditions",
                    MediaFormat::Webm,
                    150_000,
                    1920,
                    1080,
                ),
                PlaylistItem::new(
                    "audio/nova_acoustic_symphony.flac",
                    "NovaOS Acoustic Symphony 24/96",
                    "Nova Philharmonic Orchestra",
                    MediaFormat::Flac,
                    210_000,
                    0,
                    0,
                ),
                PlaylistItem::new(
                    "audio/cyber_bassline_master.mp3",
                    "Cyber Bassline Overdrive (320kbps)",
                    "Sub-Zero Synthetics",
                    MediaFormat::Mp3,
                    195_000,
                    0,
                    0,
                ),
            ],
            current_index: 0,
            loop_mode: LoopMode::RepeatAll,
            is_shuffled: false,
        }
    }
}

impl Playlist {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_item(&self) -> Option<&PlaylistItem> {
        self.items.get(self.current_index)
    }

    pub fn next(&mut self) -> Option<&PlaylistItem> {
        if self.items.is_empty() {
            return None;
        }
        match self.loop_mode {
            LoopMode::RepeatOne => {
                // keep current_index
            }
            LoopMode::RepeatAll => {
                self.current_index = (self.current_index + 1) % self.items.len();
            }
            LoopMode::None => {
                if self.current_index + 1 < self.items.len() {
                    self.current_index += 1;
                }
            }
        }
        self.current_item()
    }

    pub fn previous(&mut self) -> Option<&PlaylistItem> {
        if self.items.is_empty() {
            return None;
        }
        if self.current_index > 0 {
            self.current_index -= 1;
        } else if self.loop_mode == LoopMode::RepeatAll {
            self.current_index = self.items.len() - 1;
        }
        self.current_item()
    }

    pub fn select(&mut self, index: usize) -> Option<&PlaylistItem> {
        if index < self.items.len() {
            self.current_index = index;
            self.current_item()
        } else {
            None
        }
    }

    pub fn add(&mut self, item: PlaylistItem) {
        self.items.push(item);
    }
}
