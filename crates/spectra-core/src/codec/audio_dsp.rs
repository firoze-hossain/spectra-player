//! Audio Digital Signal Processing, spectrum analyzer, and PCM synthesizer.

use crate::types::EqualizerPreset;

pub const SPECTRUM_BANDS: usize = 32;

#[derive(Debug, Clone)]
pub struct AudioEngine {
    pub volume: f32, // 0.0 to 1.25 (125% VLC-style boost)
    pub is_muted: bool,
    pub equalizer: EqualizerPreset,
    pub sample_rate: u32,
    pub spectrum_levels: [f32; SPECTRUM_BANDS],
    pub spectrum_peaks: [f32; SPECTRUM_BANDS],
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self {
            volume: 1.0, // 100%
            is_muted: false,
            equalizer: EqualizerPreset::NovaAcoustic,
            sample_rate: 48000,
            spectrum_levels: [0.0; SPECTRUM_BANDS],
            spectrum_peaks: [0.0; SPECTRUM_BANDS],
        }
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.25);
        if self.volume > 0.0 {
            self.is_muted = false;
        }
    }

    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
    }

    pub fn effective_gain(&self) -> f32 {
        if self.is_muted {
            0.0
        } else {
            self.volume
        }
    }

    /// Computes dynamic 32-band spectrum levels based on current playback timestamp and equalizer.
    pub fn update_spectrum(&mut self, time_ms: u64, is_playing: bool) {
        if !is_playing || self.is_muted || self.volume == 0.0 {
            // Decay to zero smoothly
            for i in 0..SPECTRUM_BANDS {
                self.spectrum_levels[i] *= 0.85;
                self.spectrum_peaks[i] = (self.spectrum_peaks[i] - 0.03).max(0.0);
            }
            return;
        }

        let t = (time_ms as f32) / 1000.0;
        let gains = self.equalizer.gains_db();
        let gain_scale = self.effective_gain();

        for i in 0..SPECTRUM_BANDS {
            let freq_idx = (i * 8) / SPECTRUM_BANDS;
            let eq_boost = (gains[freq_idx] / 12.0) + 1.0; // scale dB to factor

            // Procedural rhythmic acoustic harmonics
            let beat = (t * 2.2).sin().abs(); // 132 BPM tempo
            let bass = if i < 8 { ((t * 4.4).sin().abs().powi(3)) * 0.9 } else { 0.1 };
            let mid = if i >= 8 && i < 22 { ((t * 3.1 + i as f32 * 0.4).cos().abs()) * 0.7 } else { 0.2 };
            let treble = if i >= 22 { ((t * 5.8 - i as f32 * 0.2).sin().abs()) * 0.5 } else { 0.1 };

            let raw = (bass + mid + treble + beat * 0.4) * eq_boost * gain_scale;
            let target = raw.clamp(0.05, 1.0);

            // Smooth interpolation
            self.spectrum_levels[i] = self.spectrum_levels[i] * 0.4 + target * 0.6;

            // Peak hold with gravity drop
            if self.spectrum_levels[i] > self.spectrum_peaks[i] {
                self.spectrum_peaks[i] = self.spectrum_levels[i];
            } else {
                self.spectrum_peaks[i] = (self.spectrum_peaks[i] - 0.015).max(0.0);
            }
        }
    }

    /// Generates a chunk of stereo 16-bit PCM samples for system audio output.
    pub fn generate_pcm_chunk(&self, num_samples: usize, time_ms: u64) -> Vec<i16> {
        let mut pcm = Vec::with_capacity(num_samples * 2);
        let gain = self.effective_gain();
        let t_base = (time_ms as f64) / 1000.0;
        let dt = 1.0 / (self.sample_rate as f64);

        for s in 0..num_samples {
            let t = t_base + (s as f64) * dt;

            // Generate harmonious synth waveform (A4 440Hz harmonic fundamental + sub-bass + shimmer)
            let sub = (t * 2.0 * core::f64::consts::PI * 110.0).sin() * 0.4;
            let mid = (t * 2.0 * core::f64::consts::PI * 440.0).sin() * 0.3;
            let high = (t * 2.0 * core::f64::consts::PI * 880.0).sin() * 0.15;

            let sample = (sub + mid + high) * (gain as f64);
            // Soft-clipping saturation to handle volume > 1.0 without harsh digital clicks
            let clipped = sample.tanh() * 32000.0;
            let s16 = clipped as i16;

            pcm.push(s16); // Left
            pcm.push(s16); // Right
        }

        pcm
    }
}
