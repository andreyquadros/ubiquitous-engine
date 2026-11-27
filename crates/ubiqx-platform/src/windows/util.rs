//! Small Win32 helpers: UTF-16 strings, COM initialisation and the `FileDescription`
//! version resource of an executable.

use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;

use parking_lot::Mutex;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;

/// A NUL-terminated UTF-16 copy of `s`.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The string up to the first NUL of a UTF-16 buffer.
pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

thread_local! {
    static COM_READY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Initialises COM (multithreaded) on the calling thread once. A thread that already
/// initialised COM in another mode keeps it: the calls we make work either way.
pub fn ensure_com() {
    COM_READY.with(|ready| {
        if !ready.get() {
            // SAFETY: plain COM initialisation; the result is informational only.
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            ready.set(true);
        }
    });
}

/// Expands `%VAR%` references (`%ProgramFiles%\X\x.exe`).
pub fn expand_env(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let wide = to_wide(s);
    let mut buf = vec![0u16; 4096];
    // SAFETY: `wide` is NUL-terminated and `buf` is a valid output slice.
    let n = unsafe { ExpandEnvironmentStringsW(PCWSTR(wide.as_ptr()), Some(&mut buf)) };
    if n == 0 || n as usize > buf.len() {
        return s.to_string();
    }
    from_wide(&buf)
}

/// `FileDescription` of an executable's version resource, cached per path (reading it
/// maps the file, which is cheap but not free on every sample).
pub fn file_description(path: &str) -> Option<String> {
    static CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);
    let key = path.to_lowercase();
    {
        let cache = CACHE.lock();
        if let Some(v) = cache.as_ref().and_then(|m| m.get(&key)) {
            return v.clone();
        }
    }
    let value = read_file_description(path);
    let mut cache = CACHE.lock();
    let map = cache.get_or_insert_with(HashMap::new);
    if map.len() > 512 {
        map.clear();
    }
    map.insert(key, value.clone());
    value
}

fn read_file_description(path: &str) -> Option<String> {
    let wide = to_wide(path);
    // SAFETY: `wide` is NUL-terminated; the buffer sizes below come from the API itself.
    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(wide.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(PCWSTR(wide.as_ptr()), None, size, data.as_mut_ptr().cast()).ok()?;

        let mut langs: Vec<(u16, u16)> = Vec::new();
        let mut ptr: *mut c_void = ptr::null_mut();
        let mut len: u32 = 0;
        let query = to_wide("\\VarFileInfo\\Translation");
        if VerQueryValueW(
            data.as_ptr().cast(),
            PCWSTR(query.as_ptr()),
            &mut ptr,
            &mut len,
        )
        .as_bool()
            && !ptr.is_null()
        {
            let pairs = std::slice::from_raw_parts(ptr as *const u16, (len as usize / 2) & !1);
            for pair in pairs.chunks_exact(2) {
                langs.push((pair[0], pair[1]));
            }
        }
        // English (Unicode and Windows-1252) as the usual fallbacks.
        langs.push((0x0409, 0x04B0));
        langs.push((0x0409, 0x04E4));

        for (lang, codepage) in langs {
            let query = to_wide(&format!(
                "\\StringFileInfo\\{lang:04X}{codepage:04X}\\FileDescription"
            ));
            let mut ptr: *mut c_void = ptr::null_mut();
            let mut len: u32 = 0;
            if !VerQueryValueW(
                data.as_ptr().cast(),
                PCWSTR(query.as_ptr()),
                &mut ptr,
                &mut len,
            )
            .as_bool()
                || ptr.is_null()
                || len == 0
            {
                continue;
            }
            let chars = std::slice::from_raw_parts(ptr as *const u16, len as usize);
            let s = from_wide(chars).trim().to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
}
