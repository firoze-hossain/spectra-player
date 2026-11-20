//! Pure-Rust Subtitle parser (SRT and WebVTT) with millisecond cue synchronization.

#[derive(Debug, Clone)]
pub struct SubtitleCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SubtitleTrack {
    pub language: String,
    pub title: String,
    pub cues: Vec<SubtitleCue>,
    pub is_enabled: bool,
}

impl Default for SubtitleTrack {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            title: "English [CC] (Default)".to_string(),
            is_enabled: true,
            cues: vec![
                SubtitleCue {
                    start_ms: 1000,
                    end_ms: 4500,
                    text: "Welcome to Nova Player — High-Fidelity Pure-Rust Media".to_string(),
                },
                SubtitleCue {
                    start_ms: 5000,
                    end_ms: 9000,
                    text: "Rendering 4K UHD & 8K Ultra High Definition at 60 FPS".to_string(),
                },
                SubtitleCue {
                    start_ms: 9500,
                    end_ms: 14000,
                    text: "Zero C/C++ Dependencies • Native NovaOS Desktop Player".to_string(),
                },
                SubtitleCue {
                    start_ms: 14500,
                    end_ms: 19500,
                    text: "Multi-Format Containers: MKV, MP4, WebM, AVI, FLAC, MP3, WAV".to_string(),
                },
                SubtitleCue {
                    start_ms: 20000,
                    end_ms: 26000,
                    text: "Audio Equalizer & 32-Band Real-Time Spectrum Visualizer Active".to_string(),
                },
            ],
        }
    }
}

impl SubtitleTrack {
    pub fn new(title: &str, language: &str) -> Self {
        Self {
            language: language.to_string(),
            title: title.to_string(),
            cues: Vec::new(),
            is_enabled: true,
        }
    }

    /// Looks up subtitle text active at the current playback time.
    pub fn active_cue_text(&self, time_ms: u64) -> Option<&str> {
        if !self.is_enabled {
            return None;
        }
        for cue in &self.cues {
            if time_ms >= cue.start_ms && time_ms <= cue.end_ms {
                return Some(&cue.text);
            }
        }
        None
    }

    /// Parses standard SubRip (.srt) text content.
    pub fn parse_srt(content: &str, title: &str) -> Self {
        let mut track = Self::new(title, "en");
        let blocks = content.split("\n\n");

        for block in blocks {
            let lines: Vec<&str> = block.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
            if lines.len() >= 2 {
                // Find time range line (contains "-->")
                for (i, line) in lines.iter().enumerate() {
                    if line.contains("-->") {
                        if let Some((start, end)) = parse_timestamp_range(line) {
                            let text = lines[i + 1..].join(" ");
                            if !text.is_empty() {
                                track.cues.push(SubtitleCue {
                                    start_ms: start,
                                    end_ms: end,
                                    text,
                                });
                            }
                        }
                        break;
                    }
                }
            }
        }

        track
    }
}

/// Parses "00:01:20,500 --> 00:01:24,000" or WebVTT "00:01:20.500 --> 00:01:24.000"
fn parse_timestamp_range(line: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return None;
    }
    let start = parse_time_str(parts[0].trim())?;
    let end = parse_time_str(parts[1].trim())?;
    Some((start, end))
}

fn parse_time_str(s: &str) -> Option<u64> {
    // Format: HH:MM:SS,mmm or HH:MM:SS.mmm
    let clean = s.replace(',', ".");
    let segs: Vec<&str> = clean.split(':').collect();
    if segs.len() != 3 {
        return None;
    }

    let h: u64 = segs[0].parse().ok()?;
    let m: u64 = segs[1].parse().ok()?;
    let s_parts: Vec<&str> = segs[2].split('.').collect();
    let sec: u64 = s_parts[0].parse().ok()?;
    let ms: u64 = if s_parts.len() > 1 {
        let frac = s_parts[1];
        if frac.len() == 3 {
            frac.parse().ok()?
        } else if frac.len() == 2 {
            frac.parse::<u64>().ok()? * 10
        } else {
            frac.parse::<u64>().ok()?
        }
    } else {
        0
    };

    Some(h * 3600_000 + m * 60_000 + sec * 1000 + ms)
}
