use std::path::Path;
use std::ptr;

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

pub struct AudioController {
    pub endpoint: IAudioEndpointVolume,
}

impl AudioController {
    pub unsafe fn new() -> Result<Self> {
        // initialize COM library for this thread
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;

            Ok(AudioController { endpoint })
        }
    }

    /// If `set_mute` fails, it is likely due to a change in the audio endpoint (e.g. audio device change)
    unsafe fn set_mute(&mut self, mute: bool) -> Result<()> {
        unsafe {
            let current_state = match self.endpoint.GetMute() {
                Ok(m) => m.as_bool(),
                Err(e) => {
                    eprintln!("{e:?}");
                    self.reset_device()?;
                    self.endpoint.GetMute()?.as_bool()
                }
            };
            if current_state != mute {
                self.endpoint.SetMute(mute, ptr::null())?;
                println!("Audio Status: [{}]", if mute { "MUTED" } else { "UNMUTED" });
            }
        }
        Ok(())
    }

    unsafe fn reset_device(&mut self) -> Result<()> {
        unsafe {
            let endpoint = self.get_endpoint()?;
            self.endpoint = endpoint;
        }
        Ok(())
    }

    unsafe fn get_endpoint(&self) -> Result<IAudioEndpointVolume> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;

            Ok(endpoint)
        }
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

pub unsafe fn scan(audio_controller: &mut AudioController) -> Result<()> {
    let mut context = ScanContext {
        found_trigger: false,
        target_bin: TARGET_BIN.to_string(),
    };

    unsafe {
        EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut context as *mut _ as isize),
        )?;

        if context.found_trigger {
            audio_controller.set_mute(true)?;
        } else {
            audio_controller.set_mute(false)?;
        };
    }

    Ok(())
}
