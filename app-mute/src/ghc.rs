use std::path::Path;
use std::ptr;
use std::{thread, time::Duration};

use windows::Win32::Foundation::{HWND, LPARAM, MAX_PATH};
use windows::Win32::Media::Audio::{
    Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator, eMultimedia, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
};
use windows::core::*;

const TARGET_BIN: &str = "spotify.exe";
const TRIGGER_TITLE: &str = "Advertisement";

struct ScanContext {
    found_trigger: bool,
    target_bin: String,
}

struct AudioController {
    endpoint: IAudioEndpointVolume,
}

impl AudioController {
    unsafe fn new() -> Result<Self> {
        // initialize COM library for this thread
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
        };

        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)? };
        let endpoint: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None) }?;

        Ok(AudioController { endpoint })
    }

    unsafe fn set_mute(&self, mute: bool) -> Result<()> {
        // only set if state is different to avoid flickering
        let current_state = unsafe { self.endpoint.GetMute() }?.as_bool();
        if current_state != mute {
            unsafe { self.endpoint.SetMute(mute, ptr::null()) }?;
            println!("Audio Status: [{}]", if mute { "MUTED" } else { "UNMUTED" });
        }
        Ok(())
    }
}

unsafe fn get_process_path(process_id: u32) -> String {
    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) };

    if let Ok(handle) = process_handle {
        let mut buffer = [0u16; MAX_PATH as usize];
        let mut size = buffer.len() as u32;

        let result = unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
        };

        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(handle);
        }

        if result.is_ok() {
            return String::from_utf16_lossy(&buffer[..size as usize]);
        }
    }
    String::new()
}

unsafe extern "system" fn enum_window_callback(window: HWND, lparam: LPARAM) -> BOOL {
    // restore context from the pointer
    let context = unsafe { &mut *(lparam.0 as *mut ScanContext) };

    let len = unsafe { GetWindowTextLengthW(window) };
    if len > 0 {
        let mut buffer = vec![0u16; (len + 1) as usize];
        let text_len = unsafe { GetWindowTextW(window, &mut buffer) };
        let window_title = String::from_utf16_lossy(&buffer[..text_len as usize]);

        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut process_id));
        }

        let full_path = unsafe { get_process_path(process_id) };

        let bin_name = Path::new(&full_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if bin_name.eq_ignore_ascii_case(&context.target_bin) {
            if window_title == TRIGGER_TITLE {
                context.found_trigger = true;
            }
        }
    }
    BOOL(1)
}

/// Monitor provides the scanning and mute/unmute logic without applying any
/// threading policy. The caller is responsible for creating threads or scheduling
/// the periodic checks.
pub struct Monitor {
    target_bin: String,
    trigger_title: String,
    audio: AudioController,
}

impl Monitor {
    /// Create a new Monitor. Must be called on the thread that will run the
    /// monitoring loop (COM initialization happens inside `AudioController::new`).
    pub fn new() -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        unsafe {
            let audio = AudioController::new().map_err(|e| {
                let msg = format!("app-mute: failed to init audio: {:?}", e);
                Box::<dyn std::error::Error + Send + Sync>::from(msg)
            })?;
            Ok(Self {
                target_bin: TARGET_BIN.to_string(),
                trigger_title: TRIGGER_TITLE.to_string(),
                audio,
            })
        }
    }

    /// Run a single scan+apply iteration. Returns `Ok(true)` if the trigger was
    /// found (i.e., audio was or should be muted).
    pub fn check_and_apply(
        &self,
    ) -> std::result::Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut context = ScanContext {
            found_trigger: false,
            target_bin: self.target_bin.clone(),
        };

        // run the scan
        unsafe {
            let _ = EnumWindows(
                Some(enum_window_callback),
                LPARAM(&mut context as *mut _ as isize),
            );
        }

        unsafe {
            if context.found_trigger {
                let _ = self.audio.set_mute(true);
            } else {
                let _ = self.audio.set_mute(false);
            }
        }

        Ok(context.found_trigger)
    }
}
