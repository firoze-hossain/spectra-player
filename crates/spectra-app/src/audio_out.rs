//! Cross-platform real-time audio output backend:
//! - Linux: dynamic `dlopen` of PulseAudio (`libpulse-simple.so.0` -> PipeWire/Pulse) with ALSA (`libasound.so.2`) fallback (zero `-dev` headers required)
//! - macOS & Windows: native `cpal` (CoreAudio / WASAPI)

use spectra_core::codec::media_backend::TARGET_AUDIO_RATE;
use spectra_ui::SpectraPlayerView;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct AudioOutputHandle {
    stop_flag: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
    pub backend_name: String,
}

impl Drop for AudioOutputHandle {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Release);
    }
}

pub fn start_audio_output(view: Arc<Mutex<SpectraPlayerView>>) -> Option<AudioOutputHandle> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_flag);

    #[cfg(target_os = "linux")]
    {
        let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
        let handle = thread::spawn(move || {
            if run_pulse_simple(Arc::clone(&view), Arc::clone(&stop_clone), &tx).is_ok() {
                return;
            }
            if run_alsa(view, stop_clone, &tx).is_ok() {
                return;
            }
            let _ = tx.send(None);
        });
        let name = rx.recv_timeout(Duration::from_millis(800)).ok().flatten();
        if let Some(backend_name) = name {
            return Some(AudioOutputHandle {
                stop_flag,
                _thread: Some(handle),
                backend_name,
            });
        }
        stop_flag.store(true, Ordering::Release);
        None
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
        let handle = thread::spawn(move || {
            run_cpal(view, stop_clone, &tx);
        });
        let name = rx.recv_timeout(Duration::from_millis(1200)).ok().flatten();
        if let Some(backend_name) = name {
            return Some(AudioOutputHandle {
                stop_flag,
                _thread: Some(handle),
                backend_name,
            });
        }
        stop_flag.store(true, Ordering::Release);
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (view, stop_clone);
        None
    }
}

#[cfg(target_os = "linux")]
fn run_pulse_simple(
    view: Arc<Mutex<SpectraPlayerView>>,
    stop: Arc<AtomicBool>,
    ready_tx: &std::sync::mpsc::Sender<Option<String>>,
) -> Result<(), ()> {
    use std::ffi::{c_char, c_int, c_void, CString};

    #[repr(C)]
    struct PaSampleSpec {
        format: i32,
        rate: u32,
        channels: u8,
    }

    type PaSimpleNew = unsafe extern "C" fn(
        server: *const c_char,
        name: *const c_char,
        dir: c_int,
        dev: *const c_char,
        stream_name: *const c_char,
        ss: *const PaSampleSpec,
        map: *const c_void,
        attr: *const c_void,
        error: *mut c_int,
    ) -> *mut c_void;
    type PaSimpleWrite = unsafe extern "C" fn(
        s: *mut c_void,
        data: *const c_void,
        bytes: usize,
        error: *mut c_int,
    ) -> c_int;
    type PaSimpleFree = unsafe extern "C" fn(s: *mut c_void);

    let lib = unsafe { libloading::Library::new("libpulse-simple.so.0") }.map_err(|_| ())?;
    let pa_new: libloading::Symbol<PaSimpleNew> = unsafe { lib.get(b"pa_simple_new\0") }.map_err(|_| ())?;
    let pa_write: libloading::Symbol<PaSimpleWrite> = unsafe { lib.get(b"pa_simple_write\0") }.map_err(|_| ())?;
    let pa_free: libloading::Symbol<PaSimpleFree> = unsafe { lib.get(b"pa_simple_free\0") }.map_err(|_| ())?;

    let app_name = CString::new("Spectra Media Studio").unwrap();
    let stream_name = CString::new("Playback").unwrap();
    let ss = PaSampleSpec {
        format: 5, // PA_SAMPLE_FLOAT32LE
        rate: TARGET_AUDIO_RATE,
        channels: 2,
    };
    let mut err: c_int = 0;
    let handle = unsafe {
        pa_new(
            std::ptr::null(),
            app_name.as_ptr(),
            1, // PA_STREAM_PLAYBACK
            std::ptr::null(),
            stream_name.as_ptr(),
            &ss,
            std::ptr::null(),
            std::ptr::null(),
            &mut err,
        )
    };
    if handle.is_null() {
        return Err(());
    }

    let _ = ready_tx.send(Some("PulseAudio / PipeWire".to_string()));

    let mut pcm = vec![0.0_f32; 512 * 2];
    while !stop.load(Ordering::Acquire) {
        let mut active = false;
        if let Ok(mut v) = view.lock() {
            active = v.engine.session.is_some()
                && v.engine.state == spectra_core::types::PlaybackState::Playing;
            if active {
                v.engine.fill_audio_output(&mut pcm);
            } else {
                pcm.fill(0.0);
            }
        }
        if !active {
            thread::sleep(Duration::from_millis(12));
            continue;
        }
        let bytes = pcm.len() * std::mem::size_of::<f32>();
        let rc = unsafe { pa_write(handle, pcm.as_ptr() as *const c_void, bytes, &mut err) };
        if rc < 0 {
            break;
        }
    }

    unsafe { pa_free(handle) };
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_alsa(
    view: Arc<Mutex<SpectraPlayerView>>,
    stop: Arc<AtomicBool>,
    ready_tx: &std::sync::mpsc::Sender<Option<String>>,
) -> Result<(), ()> {
    use std::ffi::{c_char, c_int, c_uint, c_void, CString};

    type SndPcmOpen = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
    type SndPcmSetParams =
        unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
    type SndPcmWritei = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> isize;
    type SndPcmPrepare = unsafe extern "C" fn(*mut c_void) -> c_int;
    type SndPcmClose = unsafe extern "C" fn(*mut c_void) -> c_int;

    let lib = unsafe { libloading::Library::new("libasound.so.2") }.map_err(|_| ())?;
    let pcm_open: libloading::Symbol<SndPcmOpen> = unsafe { lib.get(b"snd_pcm_open\0") }.map_err(|_| ())?;
    let pcm_params: libloading::Symbol<SndPcmSetParams> =
        unsafe { lib.get(b"snd_pcm_set_params\0") }.map_err(|_| ())?;
    let pcm_writei: libloading::Symbol<SndPcmWritei> =
        unsafe { lib.get(b"snd_pcm_writei\0") }.map_err(|_| ())?;
    let pcm_prepare: libloading::Symbol<SndPcmPrepare> =
        unsafe { lib.get(b"snd_pcm_prepare\0") }.map_err(|_| ())?;
    let pcm_close: libloading::Symbol<SndPcmClose> =
        unsafe { lib.get(b"snd_pcm_close\0") }.map_err(|_| ())?;

    let dev = CString::new("default").unwrap();
    let mut pcm_ptr: *mut c_void = std::ptr::null_mut();
    if unsafe { pcm_open(&mut pcm_ptr, dev.as_ptr(), 0, 0) } < 0 || pcm_ptr.is_null() {
        return Err(());
    }
    // SND_PCM_FORMAT_S16_LE = 2, SND_PCM_ACCESS_RW_INTERLEAVED = 3
    if unsafe { pcm_params(pcm_ptr, 2, 3, 2, TARGET_AUDIO_RATE, 1, 50_000) } < 0 {
        unsafe { pcm_close(pcm_ptr) };
        return Err(());
    }

    let _ = ready_tx.send(Some("ALSA PCM".to_string()));

    let mut fbuf = vec![0.0_f32; 512 * 2];
    let mut ibuf = vec![0_i16; 512 * 2];
    while !stop.load(Ordering::Acquire) {
        let mut active = false;
        if let Ok(mut v) = view.lock() {
            active = v.engine.session.is_some()
                && v.engine.state == spectra_core::types::PlaybackState::Playing;
            if active {
                v.engine.fill_audio_output(&mut fbuf);
            } else {
                fbuf.fill(0.0);
            }
        }
        if !active {
            thread::sleep(Duration::from_millis(12));
            continue;
        }
        for (i, &s) in fbuf.iter().enumerate() {
            ibuf[i] = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        }
        let written = unsafe { pcm_writei(pcm_ptr, ibuf.as_ptr() as *const c_void, 512) };
        if written < 0 {
            unsafe { pcm_prepare(pcm_ptr) };
        }
    }

    unsafe { pcm_close(pcm_ptr) };
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn run_cpal(
    view: Arc<Mutex<SpectraPlayerView>>,
    stop: Arc<AtomicBool>,
    ready_tx: &std::sync::mpsc::Sender<Option<String>>,
) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else {
        let _ = ready_tx.send(None);
        return;
    };
    let Ok(supported) = device.default_output_config() else {
        let _ = ready_tx.send(None);
        return;
    };
    let channels = supported.channels().max(1) as usize;
    let dev_rate = supported.sample_rate().0;
    let config = cpal::StreamConfig {
        channels: supported.channels(),
        sample_rate: supported.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };

    let view_cb = Arc::clone(&view);
    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _| {
            let out_frames = data.len() / channels;
            let src_frames = ((out_frames as f64 * TARGET_AUDIO_RATE as f64) / dev_rate as f64).ceil() as usize;
            let mut tmp = vec![0.0_f32; src_frames.max(1) * 2];
            if let Ok(mut v) = view_cb.try_lock() {
                v.engine.fill_audio_output(&mut tmp);
            }
            for f in 0..out_frames {
                let sf = ((f as f64 * TARGET_AUDIO_RATE as f64) / dev_rate as f64) as usize;
                let si = sf.min(src_frames.saturating_sub(1)) * 2;
                let l = tmp[si];
                let r = tmp[si + 1];
                for c in 0..channels {
                    data[f * channels + c] = if c == 0 { l } else { r };
                }
            }
        },
        |_err| {},
        None,
    );

    match stream {
        Ok(st) if st.play().is_ok() => {
            let backend = if cfg!(target_os = "macos") {
                "CoreAudio (cpal)"
            } else {
                "WASAPI (cpal)"
            };
            let _ = ready_tx.send(Some(backend.to_string()));
            while !stop.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(25));
            }
        }
        _ => {
            let _ = ready_tx.send(None);
        }
    }
}
