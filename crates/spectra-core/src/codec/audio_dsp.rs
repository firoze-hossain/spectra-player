//! Audio Digital Signal Processing:
//! - Real 10-band biquad IIR parametric equalizer (60Hz..16kHz)
//! - Up to 200% VLC-style software preamp gain with soft-knee tanh limiter
//! - Real-time 1024-point Cooley-Tukey radix-2 FFT driving 32-band spectrum & oscilloscope

use crate::types::{AudioVisualizerMode, EqualizerPreset};

pub const SPECTRUM_BANDS: usize = 32;
pub const EQ_BANDS: usize = 10;
pub const WAVEFORM_SAMPLES: usize = 128;
pub const FFT_SIZE: usize = 512;

pub const EQ_FREQUENCIES: [f32; EQ_BANDS] = [
    60.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 12000.0, 14000.0, 16000.0,
];

#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1_l: f32,
    x2_l: f32,
    y1_l: f32,
    y2_l: f32,
    x1_r: f32,
    x2_r: f32,
    y1_r: f32,
    y2_r: f32,
}

impl BiquadState {
    fn update_peaking(&mut self, sample_rate: f32, freq: f32, gain_db: f32, q: f32) {
        if gain_db.abs() < 0.05 || freq >= sample_rate * 0.45 {
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
            self.a1 = 0.0;
            self.a2 = 0.0;
            return;
        }
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * core::f32::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);

        let a0 = 1.0 + alpha / a;
        self.b0 = (1.0 + alpha * a) / a0;
        self.b1 = (-2.0 * cos_w0) / a0;
        self.b2 = (1.0 - alpha * a) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha / a) / a0;
    }

    #[inline(always)]
    fn process_stereo(&mut self, l: f32, r: f32) -> (f32, f32) {
        let out_l = self.b0 * l + self.b1 * self.x1_l + self.b2 * self.x2_l
            - self.a1 * self.y1_l
            - self.a2 * self.y2_l;
        self.x2_l = self.x1_l;
        self.x1_l = l;
        self.y2_l = self.y1_l;
        self.y1_l = if out_l.is_finite() { out_l } else { 0.0 };

        let out_r = self.b0 * r + self.b1 * self.x1_r + self.b2 * self.x2_r
            - self.a1 * self.y1_r
            - self.a2 * self.y2_r;
        self.x2_r = self.x1_r;
        self.x1_r = r;
        self.y2_r = self.y1_r;
        self.y1_r = if out_r.is_finite() { out_r } else { 0.0 };

        (self.y1_l, self.y1_r)
    }
}

#[derive(Debug, Clone)]
pub struct AudioEngine {
    pub volume: f32, // 0.0 to 2.0 (200% VLC-style software boost)
    pub is_muted: bool,
    pub equalizer: EqualizerPreset,
    pub eq_enabled: bool,
    pub eq_preamp_db: f32,
    pub eq_bands_db: [f32; EQ_BANDS],
    pub sample_rate: u32,
    pub audio_delay_ms: i64,
    pub visualizer_mode: AudioVisualizerMode,
    pub spectrum_levels: [f32; SPECTRUM_BANDS],
    pub spectrum_peaks: [f32; SPECTRUM_BANDS],
    pub waveform_samples: [f32; WAVEFORM_SAMPLES],
    pub has_live_pcm: bool,
    biquads: [BiquadState; EQ_BANDS],
    biquad_sample_rate: u32,
    biquads_dirty: bool,
    fft_ring: [f32; FFT_SIZE],
    fft_write_pos: usize,
}

impl Default for AudioEngine {
    fn default() -> Self {
        let preset = EqualizerPreset::Flat;
        let gains = preset.gains_db_10();
        Self {
            volume: 1.0,
            is_muted: false,
            equalizer: preset,
            eq_enabled: true,
            eq_preamp_db: 0.0,
            eq_bands_db: gains,
            sample_rate: 48000,
            audio_delay_ms: 0,
            visualizer_mode: AudioVisualizerMode::Spectrum32,
            spectrum_levels: [0.0; SPECTRUM_BANDS],
            spectrum_peaks: [0.0; SPECTRUM_BANDS],
            waveform_samples: [0.0; WAVEFORM_SAMPLES],
            has_live_pcm: false,
            biquads: [BiquadState::default(); EQ_BANDS],
            biquad_sample_rate: 0,
            biquads_dirty: true,
            fft_ring: [0.0; FFT_SIZE],
            fft_write_pos: 0,
        }
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 2.0);
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

    pub fn set_equalizer_preset(&mut self, preset: EqualizerPreset) {
        self.equalizer = preset;
        if preset != EqualizerPreset::Custom {
            self.eq_bands_db = preset.gains_db_10();
        }
        self.biquads_dirty = true;
    }

    pub fn set_eq_band(&mut self, band: usize, gain_db: f32) {
        if band < EQ_BANDS {
            self.equalizer = EqualizerPreset::Custom;
            self.eq_bands_db[band] = gain_db.clamp(-12.0, 12.0);
            self.biquads_dirty = true;
        }
    }

    pub fn reset_eq(&mut self) {
        self.eq_preamp_db = 0.0;
        self.set_equalizer_preset(EqualizerPreset::Flat);
    }

    fn ensure_biquads(&mut self, sample_rate: u32) {
        if self.biquads_dirty || self.biquad_sample_rate != sample_rate {
            let sr = sample_rate.max(8000) as f32;
            for i in 0..EQ_BANDS {
                self.biquads[i].update_peaking(sr, EQ_FREQUENCIES[i], self.eq_bands_db[i], 1.41);
            }
            self.biquad_sample_rate = sample_rate;
            self.biquads_dirty = false;
        }
    }

    /// Processes interleaved stereo f32 PCM samples in-place:
    /// applies 10-band EQ, preamp, volume boost (up to 200%), and soft-knee limiter.
    pub fn process_pcm_stereo(&mut self, samples: &mut [f32], sample_rate: u32) {
        self.ensure_biquads(sample_rate);
        let apply_eq = self.eq_enabled && self.eq_bands_db.iter().any(|g| g.abs() >= 0.05);
        let preamp_lin = if self.eq_enabled {
            10.0_f32.powf(self.eq_preamp_db / 20.0)
        } else {
            1.0
        };
        let gain = self.effective_gain() * preamp_lin;

        for frame in samples.chunks_exact_mut(2) {
            let mut l = frame[0];
            let mut r = frame[1];

            if apply_eq {
                for b in 0..EQ_BANDS {
                    (l, r) = self.biquads[b].process_stereo(l, r);
                }
            }

            l *= gain;
            r *= gain;

            // Soft-knee saturation when volume > 100% or EQ boosts over 0 dBFS
            if l.abs() > 0.95 {
                l = (l * 1.15).tanh();
            }
            if r.abs() > 0.95 {
                r = (r * 1.15).tanh();
            }

            frame[0] = l;
            frame[1] = r;
        }
    }

    /// Feeds live stereo PCM samples into the real-time FFT analyzer and oscilloscope.
    pub fn feed_pcm_for_spectrum(&mut self, interleaved_stereo: &[f32]) {
        if interleaved_stereo.len() < 2 {
            return;
        }
        self.has_live_pcm = true;

        let frames = interleaved_stereo.len() / 2;
        let step = (frames / 128).max(1);
        let mut wf_idx = 0usize;

        for i in (0..frames).step_by(step) {
            let mono = 0.5 * (interleaved_stereo[i * 2] + interleaved_stereo[i * 2 + 1]);
            self.fft_ring[self.fft_write_pos] = mono.clamp(-1.0, 1.0);
            self.fft_write_pos = (self.fft_write_pos + 1) % FFT_SIZE;

            if wf_idx < WAVEFORM_SAMPLES {
                self.waveform_samples[wf_idx] = mono.clamp(-1.0, 1.0);
                wf_idx += 1;
            }
        }
        // Also ensure FFT ring advances smoothly if chunk is small
        if step > 1 {
            for f in interleaved_stereo.chunks_exact(2).take(FFT_SIZE) {
                let mono = 0.5 * (f[0] + f[1]);
                self.fft_ring[self.fft_write_pos] = mono.clamp(-1.0, 1.0);
                self.fft_write_pos = (self.fft_write_pos + 1) % FFT_SIZE;
            }
        }

        self.compute_fft_spectrum();
    }

    fn compute_fft_spectrum(&mut self) {
        let mut re = [0.0_f32; FFT_SIZE];
        let mut im = [0.0_f32; FFT_SIZE];

        // Hann window over circular buffer
        for i in 0..FFT_SIZE {
            let idx = (self.fft_write_pos + i) % FFT_SIZE;
            let w = 0.5 * (1.0 - (2.0 * core::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos());
            re[i] = self.fft_ring[idx] * w;
        }

        fft_radix2(&mut re, &mut im);

        let half = FFT_SIZE / 2;
        // Map FFT bins logarithmically across 32 visual bands
        for b in 0..SPECTRUM_BANDS {
            let f0 = ((b as f32 / SPECTRUM_BANDS as f32).powf(1.85) * (half - 1) as f32) as usize;
            let f1 = (((b + 1) as f32 / SPECTRUM_BANDS as f32).powf(1.85) * (half - 1) as f32) as usize;
            let bin_start = f0.max(1).min(half - 1);
            let bin_end = f1.max(bin_start + 1).min(half);

            let mut max_mag = 0.0_f32;
            for k in bin_start..bin_end {
                let mag = (re[k] * re[k] + im[k] * im[k]).sqrt() / (FFT_SIZE as f32 * 0.25);
                if mag > max_mag {
                    max_mag = mag;
                }
            }

            // Perceptual dB scaling
            let db = if max_mag > 1e-5 {
                20.0 * max_mag.log10()
            } else {
                -60.0
            };
            let norm = ((db + 52.0) / 52.0).clamp(0.02, 1.0);
            let target = if self.is_muted || self.volume == 0.0 { 0.0 } else { norm };

            self.spectrum_levels[b] = self.spectrum_levels[b] * 0.35 + target * 0.65;
            if self.spectrum_levels[b] > self.spectrum_peaks[b] {
                self.spectrum_peaks[b] = self.spectrum_levels[b];
            } else {
                self.spectrum_peaks[b] = (self.spectrum_peaks[b] - 0.015).max(0.0);
            }
        }
    }

    /// Decays spectrum and waveform smoothly when no live PCM is playing.
    pub fn update_spectrum(&mut self, _time_ms: u64, is_playing: bool) {
        if self.has_live_pcm && is_playing && !self.is_muted && self.volume > 0.0 {
            return;
        }
        for i in 0..SPECTRUM_BANDS {
            self.spectrum_levels[i] *= 0.84;
            if self.spectrum_levels[i] < 0.005 {
                self.spectrum_levels[i] = 0.0;
            }
            self.spectrum_peaks[i] = (self.spectrum_peaks[i] - 0.03).max(0.0);
        }
        for w in &mut self.waveform_samples {
            *w *= 0.8;
            if w.abs() < 0.002 {
                *w = 0.0;
            }
        }
    }

    /// Generates a chunk of stereo 16-bit PCM samples for fallback/NovaOS audio output.
    pub fn generate_pcm_chunk(&self, num_samples: usize, time_ms: u64) -> Vec<i16> {
        let mut pcm = Vec::with_capacity(num_samples * 2);
        let gain = self.effective_gain();
        let t_base = (time_ms as f64) / 1000.0;
        let dt = 1.0 / (self.sample_rate.max(8000) as f64);

        for s in 0..num_samples {
            let t = t_base + (s as f64) * dt;
            let sub = (t * 2.0 * core::f64::consts::PI * 110.0).sin() * 0.4;
            let mid = (t * 2.0 * core::f64::consts::PI * 440.0).sin() * 0.3;
            let high = (t * 2.0 * core::f64::consts::PI * 880.0).sin() * 0.15;

            let sample = (sub + mid + high) * (gain as f64);
            let clipped = sample.tanh() * 32000.0;
            let s16 = clipped as i16;
            pcm.push(s16);
            pcm.push(s16);
        }

        pcm
    }
}

/// In-place pure-Rust Cooley-Tukey radix-2 FFT (power-of-two length).
pub fn fft_radix2(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    if n <= 1 || !n.is_power_of_two() || im.len() != n {
        return;
    }

    // Bit-reversal permutation
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // Butterfly stages
    let mut len = 2usize;
    while len <= n {
        let half = len / 2;
        let ang = -2.0 * core::f32::consts::PI / (len as f32);
        let (wlen_sin, wlen_cos) = ang.sin_cos();

        let mut i = 0usize;
        while i < n {
            let mut wr = 1.0_f32;
            let mut wi = 0.0_f32;
            for k in 0..half {
                let u_re = re[i + k];
                let u_im = im[i + k];
                let v_re = re[i + k + half] * wr - im[i + k + half] * wi;
                let v_im = re[i + k + half] * wi + im[i + k + half] * wr;

                re[i + k] = u_re + v_re;
                im[i + k] = u_im + v_im;
                re[i + k + half] = u_re - v_re;
                im[i + k + half] = u_im - v_im;

                let next_wr = wr * wlen_cos - wi * wlen_sin;
                wi = wr * wlen_sin + wi * wlen_cos;
                wr = next_wr;
            }
            i += len;
        }
        len <<= 1;
    }
}
