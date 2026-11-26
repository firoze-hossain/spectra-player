//! Media Playlist manager with real shuffle, repeat modes, item management,
//! and M3U / M3U8 / PLS import and export.

use crate::demux::detect::detect_format_from_path;
use crate::types::{MediaFormat, MediaMetadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    None,
    RepeatOne,
    RepeatAll,
}

impl LoopMode {
    pub fn next_mode(&self) -> Self {
        match self {
            LoopMode::None => LoopMode::RepeatAll,
            LoopMode::RepeatAll => LoopMode::RepeatOne,
            LoopMode::RepeatOne => LoopMode::None,
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            LoopMode::None => "REP",
            LoopMode::RepeatAll => "ALL",
            LoopMode::RepeatOne => "1",
        }
    }
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
    pub is_live_file: bool,
}

impl PlaylistItem {
    pub fn new(
        path: &str,
        title: &str,
        artist: &str,
        format: MediaFormat,
        duration_ms: u64,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            path: path.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            format,
            duration_ms,
            width,
            height,
            is_live_file: false,
        }
    }

    pub fn from_metadata(meta: &MediaMetadata) -> Self {
        Self {
            path: meta.path.clone(),
            title: meta.title.clone(),
            artist: meta.artist.clone(),
            format: meta.format,
            duration_ms: meta.duration_ms,
            width: meta.width,
            height: meta.height,
            is_live_file: meta.is_live_file,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub items: Vec<PlaylistItem>,
    pub current_index: usize,
    pub loop_mode: LoopMode,
    pub is_shuffled: bool,
    shuffle_order: Vec<usize>,
    shuffle_pos: usize,
}

impl Default for Playlist {
    fn default() -> Self {
        Self::empty()
    }
}

impl Playlist {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            current_index: 0,
            loop_mode: LoopMode::RepeatAll,
            is_shuffled: false,
            shuffle_order: Vec::new(),
            shuffle_pos: 0,
        }
    }

    pub fn current_item(&self) -> Option<&PlaylistItem> {
        self.items.get(self.current_index)
    }

    pub fn set_shuffle(&mut self, enabled: bool, seed: u64) {
        self.is_shuffled = enabled;
        if enabled {
            self.rebuild_shuffle(seed);
        }
    }

    pub fn toggle_shuffle(&mut self, seed: u64) -> bool {
        self.set_shuffle(!self.is_shuffled, seed);
        self.is_shuffled
    }

    fn rebuild_shuffle(&mut self, mut seed: u64) {
        let n = self.items.len();
        self.shuffle_order = (0..n).collect();
        if n <= 1 {
            self.shuffle_pos = 0;
            return;
        }
        // Simple xorshift64 Fisher-Yates shuffle (no_std compatible)
        if seed == 0 {
            seed = 0x9E37_79B9_7F4A_7C15;
        }
        for i in (1..n).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let j = (seed as usize) % (i + 1);
            self.shuffle_order.swap(i, j);
        }
        // Place current_index at position 0 so next() moves to a different item
        if let Some(pos) = self.shuffle_order.iter().position(|&x| x == self.current_index) {
            self.shuffle_order.swap(0, pos);
        }
        self.shuffle_pos = 0;
    }

    pub fn next(&mut self) -> Option<&PlaylistItem> {
        if self.items.is_empty() {
            return None;
        }
        if self.loop_mode == LoopMode::RepeatOne {
            return self.current_item();
        }
        if self.is_shuffled && self.shuffle_order.len() == self.items.len() {
            if self.shuffle_pos + 1 < self.shuffle_order.len() {
                self.shuffle_pos += 1;
            } else if self.loop_mode == LoopMode::RepeatAll {
                self.shuffle_pos = 0;
            } else {
                return None;
            }
            self.current_index = self.shuffle_order[self.shuffle_pos];
            return self.current_item();
        }
        match self.loop_mode {
            LoopMode::RepeatOne => {}
            LoopMode::RepeatAll => {
                self.current_index = (self.current_index + 1) % self.items.len();
            }
            LoopMode::None => {
                if self.current_index + 1 < self.items.len() {
                    self.current_index += 1;
                } else {
                    return None;
                }
            }
        }
        self.current_item()
    }

    pub fn previous(&mut self) -> Option<&PlaylistItem> {
        if self.items.is_empty() {
            return None;
        }
        if self.is_shuffled && self.shuffle_order.len() == self.items.len() {
            if self.shuffle_pos > 0 {
                self.shuffle_pos -= 1;
            } else if self.loop_mode == LoopMode::RepeatAll {
                self.shuffle_pos = self.shuffle_order.len() - 1;
            }
            self.current_index = self.shuffle_order[self.shuffle_pos];
            return self.current_item();
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
            if let Some(pos) = self.shuffle_order.iter().position(|&x| x == index) {
                self.shuffle_pos = pos;
            }
            self.current_item()
        } else {
            None
        }
    }

    pub fn add(&mut self, item: PlaylistItem) {
        self.items.push(item);
        if self.is_shuffled {
            self.shuffle_order.push(self.items.len() - 1);
        }
    }

    /// Replaces demo-only items when user opens real media files.
    pub fn clear_demos_if_only_demos(&mut self) {
        if !self.items.is_empty() && self.items.iter().all(|i| !i.is_live_file) {
            self.items.clear();
            self.current_index = 0;
            self.shuffle_order.clear();
            self.shuffle_pos = 0;
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        self.items.remove(index);
        if self.items.is_empty() {
            self.current_index = 0;
            self.shuffle_order.clear();
            self.shuffle_pos = 0;
            return;
        }
        if self.current_index >= self.items.len() {
            self.current_index = self.items.len() - 1;
        } else if index < self.current_index {
            self.current_index = self.current_index.saturating_sub(1);
        }
        if self.is_shuffled {
            self.rebuild_shuffle(self.items.len() as u64 + 17);
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current_index = 0;
        self.shuffle_order.clear();
        self.shuffle_pos = 0;
    }

    /// Parses `.m3u` / `.m3u8` playlist text into items.
    pub fn parse_m3u(content: &str) -> Vec<PlaylistItem> {
        let mut out = Vec::new();
        let mut pending_title: Option<String> = None;
        let mut pending_dur_ms: u64 = 0;

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(info) = line.strip_prefix("#EXTINF:") {
                if let Some(comma) = info.find(',') {
                    let secs_str = info[..comma].trim();
                    if let Ok(secs) = secs_str.parse::<i64>() {
                        pending_dur_ms = if secs > 0 { (secs as u64) * 1000 } else { 0 };
                    }
                    let t = info[comma + 1..].trim();
                    if !t.is_empty() {
                        pending_title = Some(t.to_string());
                    }
                }
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            let fmt = detect_format_from_path(line);
            let title = pending_title.take().unwrap_or_else(|| {
                line.rsplit(['/', '\\']).next().unwrap_or(line).to_string()
            });
            let mut item = PlaylistItem::new(line, &title, "Playlist Entry", fmt, pending_dur_ms, 0, 0);
            item.is_live_file = true;
            out.push(item);
            pending_dur_ms = 0;
        }
        out
    }

    /// Serializes playlist items into `.m3u8` format.
    pub fn to_m3u8(&self) -> String {
        let mut s = String::from("#EXTM3U\n");
        for item in &self.items {
            let dur_s = (item.duration_ms / 1000) as i64;
            s.push_str(&format!("#EXTINF:{},{}\n{}\n", dur_s, item.title, item.path));
        }
        s
    }
}
