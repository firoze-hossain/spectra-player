//! Pure-Rust Subtitle engine supporting SubRip (.srt), WebVTT (.vtt), ASS/SSA (.ass/.ssa),
//! and MicroDVD (.sub), with millisecond cue synchronization, delay offset, and tag stripping.

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
    pub source_path: Option<String>,
    pub cues: Vec<SubtitleCue>,
    pub is_enabled: bool,
    pub delay_ms: i64,
    pub font_scale: f32,
}

impl Default for SubtitleTrack {
    fn default() -> Self {
        Self::empty()
    }
}

impl SubtitleTrack {
    pub fn new(title: &str, language: &str) -> Self {
        Self {
            language: language.to_string(),
            title: title.to_string(),
            source_path: None,
            cues: Vec::new(),
            is_enabled: true,
            delay_ms: 0,
            font_scale: 1.0,
        }
    }

    pub fn empty() -> Self {
        Self {
            language: "none".to_string(),
            title: "No Subtitles".to_string(),
            source_path: None,
            cues: Vec::new(),
            is_enabled: false,
            delay_ms: 0,
            font_scale: 1.0,
        }
    }

    /// Looks up subtitle text active at the given playback time (accounting for delay_ms).
    pub fn active_cue_text(&self, time_ms: u64) -> Option<&str> {
        if !self.is_enabled || self.cues.is_empty() {
            return None;
        }
        let adjusted = (time_ms as i64) - self.delay_ms;
        if adjusted < 0 {
            return None;
        }
        let t = adjusted as u64;
        for cue in &self.cues {
            if t >= cue.start_ms && t <= cue.end_ms {
                return Some(&cue.text);
            }
        }
        None
    }

    /// Auto-detects format by path extension or content and parses cues.
    pub fn parse_auto(content: &str, path_or_title: &str) -> Self {
        let clean = content.strip_prefix('\u{feff}').unwrap_or(content);
        let lower = path_or_title.to_ascii_lowercase();
        let mut track = if lower.ends_with(".ass") || lower.ends_with(".ssa") || clean.contains("[Script Info]") {
            Self::parse_ass(clean, path_or_title)
        } else if lower.ends_with(".vtt") || clean.trim_start().starts_with("WEBVTT") {
            Self::parse_vtt(clean, path_or_title)
        } else if lower.ends_with(".sub") && clean.trim_start().starts_with('{') {
            Self::parse_microdvd(clean, path_or_title, 25.0)
        } else {
            Self::parse_srt(clean, path_or_title)
        };
        track.source_path = Some(path_or_title.to_string());
        track.cues.sort_by_key(|c| c.start_ms);
        track
    }

    /// Parses standard SubRip (.srt) text content.
    pub fn parse_srt(content: &str, title: &str) -> Self {
        let mut track = Self::new(short_name(title), "und");
        let normalized = content.strip_prefix('\u{feff}').unwrap_or(content).replace("\r\n", "\n");
        let blocks = normalized.split("\n\n");

        for block in blocks {
            let lines: Vec<&str> = block
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();
            if lines.len() >= 2 {
                for (i, line) in lines.iter().enumerate() {
                    if line.contains("-->") {
                        if let Some((start, end)) = parse_timestamp_range(line) {
                            let raw_text = lines[i + 1..].join("\n");
                            let text = strip_markup_tags(&raw_text);
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

    /// Parses WebVTT (.vtt) text content.
    pub fn parse_vtt(content: &str, title: &str) -> Self {
        let mut track = Self::new(short_name(title), "und");
        let normalized = content.strip_prefix('\u{feff}').unwrap_or(content).replace("\r\n", "\n");

        for block in normalized.split("\n\n") {
            let trimmed = block.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("WEBVTT")
                || trimmed.starts_with("NOTE")
                || trimmed.starts_with("STYLE")
                || trimmed.starts_with("REGION")
            {
                continue;
            }
            let lines: Vec<&str> = trimmed.lines().map(|l| l.trim()).collect();
            for (i, line) in lines.iter().enumerate() {
                if line.contains("-->") {
                    if let Some((start, end)) = parse_timestamp_range(line) {
                        let raw_text = lines[i + 1..].join("\n");
                        let text = strip_markup_tags(&raw_text);
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

        track
    }

    /// Parses SubStation Alpha (.ass / .ssa) dialogue events.
    pub fn parse_ass(content: &str, title: &str) -> Self {
        let mut track = Self::new(short_name(title), "und");
        let normalized = content.strip_prefix('\u{feff}').unwrap_or(content).replace("\r\n", "\n");

        let mut in_events = false;
        let mut start_idx = 1usize;
        let mut end_idx = 2usize;
        let mut text_idx = 9usize;
        let mut field_count = 10usize;

        for line in normalized.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_events = line.eq_ignore_ascii_case("[Events]");
                continue;
            }
            if !in_events {
                continue;
            }
            if let Some(rest) = line.strip_prefix("Format:") {
                let cols: Vec<&str> = rest.split(',').map(|c| c.trim()).collect();
                field_count = cols.len().max(1);
                for (idx, col) in cols.iter().enumerate() {
                    if col.eq_ignore_ascii_case("Start") {
                        start_idx = idx;
                    } else if col.eq_ignore_ascii_case("End") {
                        end_idx = idx;
                    } else if col.eq_ignore_ascii_case("Text") {
                        text_idx = idx;
                    }
                }
            } else if let Some(rest) = line.strip_prefix("Dialogue:") {
                let parts: Vec<&str> = rest.splitn(field_count, ',').collect();
                if parts.len() > text_idx && parts.len() > start_idx && parts.len() > end_idx {
                    if let (Some(s), Some(e)) = (
                        parse_ass_time(parts[start_idx].trim()),
                        parse_ass_time(parts[end_idx].trim()),
                    ) {
                        let clean = strip_ass_overrides(parts[text_idx].trim());
                        if !clean.is_empty() {
                            track.cues.push(SubtitleCue {
                                start_ms: s,
                                end_ms: e,
                                text: clean,
                            });
                        }
                    }
                }
            }
        }

        track
    }

    /// Parses MicroDVD (.sub) `{start}{end}text` subtitles.
    pub fn parse_microdvd(content: &str, title: &str, fps: f32) -> Self {
        let mut track = Self::new(short_name(title), "und");
        let fps = if fps > 1.0 { fps } else { 25.0 };
        for line in content.lines() {
            let line = line.trim();
            if !line.starts_with('{') {
                continue;
            }
            if let Some(end1) = line.find('}') {
                let f_start: Option<u64> = line[1..end1].parse().ok();
                let rest = &line[end1 + 1..];
                if let Some(rest2) = rest.strip_prefix('{') {
                    if let Some(end2) = rest2.find('}') {
                        let f_end: Option<u64> = rest2[..end2].parse().ok();
                        let text = rest2[end2 + 1..].replace('|', "\n");
                        if let (Some(s), Some(e)) = (f_start, f_end) {
                            let start_ms = ((s as f64 * 1000.0) / fps as f64) as u64;
                            let end_ms = ((e as f64 * 1000.0) / fps as f64) as u64;
                            let clean = strip_ass_overrides(&text);
                            if !clean.is_empty() {
                                track.cues.push(SubtitleCue {
                                    start_ms,
                                    end_ms,
                                    text: clean,
                                });
                            }
                        }
                    }
                }
            }
        }
        track
    }
}

fn short_name(path_or_title: &str) -> &str {
    path_or_title.rsplit(['/', '\\']).next().unwrap_or(path_or_title)
}

fn strip_markup_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn strip_ass_overrides(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_brace = false;
    for ch in s.chars() {
        match ch {
            '{' => in_brace = true,
            '}' => in_brace = false,
            _ if !in_brace => out.push(ch),
            _ => {}
        }
    }
    out.replace("\\N", "\n").replace("\\n", "\n").replace("\\h", " ").trim().to_string()
}

/// Parses "00:01:20,500 --> 00:01:24,000" or WebVTT "01:20.500 --> 01:24.000 align:middle"
fn parse_timestamp_range(line: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return None;
    }
    let start_tok = parts[0].split_whitespace().next()?;
    let end_tok = parts[1].split_whitespace().next()?;
    let start = parse_time_str(start_tok)?;
    let end = parse_time_str(end_tok)?;
    Some((start, end))
}

fn parse_time_str(s: &str) -> Option<u64> {
    let clean = s.replace(',', ".");
    let segs: Vec<&str> = clean.split(':').collect();
    let (h, m, sec_part) = match segs.len() {
        3 => (segs[0].parse::<u64>().ok()?, segs[1].parse::<u64>().ok()?, segs[2]),
        2 => (0u64, segs[0].parse::<u64>().ok()?, segs[1]),
        _ => return None,
    };

    let s_parts: Vec<&str> = sec_part.split('.').collect();
    let sec: u64 = s_parts[0].parse().ok()?;
    let ms: u64 = if s_parts.len() > 1 {
        let frac = s_parts[1];
        if frac.len() >= 3 {
            frac[..3].parse().ok()?
        } else if frac.len() == 2 {
            frac.parse::<u64>().ok()? * 10
        } else if frac.len() == 1 {
            frac.parse::<u64>().ok()? * 100
        } else {
            0
        }
    } else {
        0
    };

    Some(h * 3600_000 + m * 60_000 + sec * 1000 + ms)
}

fn parse_ass_time(s: &str) -> Option<u64> {
    // H:MM:SS.cc (centiseconds)
    let segs: Vec<&str> = s.split(':').collect();
    if segs.len() != 3 {
        return None;
    }
    let h: u64 = segs[0].parse().ok()?;
    let m: u64 = segs[1].parse().ok()?;
    let s_parts: Vec<&str> = segs[2].split('.').collect();
    let sec: u64 = s_parts[0].parse().ok()?;
    let cs: u64 = if s_parts.len() > 1 {
        let f = s_parts[1];
        if f.len() >= 2 {
            f[..2].parse::<u64>().ok()? * 10
        } else {
            f.parse::<u64>().ok()? * 100
        }
    } else {
        0
    };
    Some(h * 3600_000 + m * 60_000 + sec * 1000 + cs)
}
