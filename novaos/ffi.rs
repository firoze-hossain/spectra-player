//! ffi.rs - FFI bindings to NovaOS ring-3 system calls (VFS, Audio Mixer, RTC, Console).

#![allow(dead_code)]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NovaAudioOpen {
    pub channels: u32,
    pub rate: u32,
    pub flags: u32,
    pub volume: u32,
    pub handle: u32,
    pub capacity_frames: u32,
    pub period_frames: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NovaAudioWrite {
    pub handle: u32,
    pub data: u32,
    pub frames: u32,
    pub accepted: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NovaAudioClose {
    pub handle: u32,
    pub flags: u32,
}

extern "C" {
    pub fn sys_write(s: *const u8) -> i32;
    pub fn sys_exit(code: i32) -> !;
    pub fn sys_yield() -> i32;
    pub fn sys_open(filename: *const u8) -> i32;
    pub fn sys_read(handle: i32, buf: *mut u8, max_len: i32) -> i32;
    pub fn sys_close(handle: i32);
    pub fn sys_list_files(buf: *mut u8, buf_size: i32) -> i32;
    pub fn sys_audio_open(req: *mut NovaAudioOpen) -> i32;
    pub fn sys_audio_write(req: *mut NovaAudioWrite) -> i32;
    pub fn sys_audio_close(req: *const NovaAudioClose) -> i32;
}
