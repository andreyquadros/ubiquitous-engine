//! Installed applications: Start Menu shortcuts (resolved through `IShellLinkW`) plus the
//! registry's `App Paths`. The app id is the executable stem, the name the shortcut's name
//! or the executable's `FileDescription`. Cached for a minute.

use std::path::{Path, PathBuf};

use ubiqx_core::ports::AppCatalog;
use ubiqx_core::{CoreResult, InstalledApp};
use windows::core::{Interface, PCWSTR, PWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Com::{
    CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, REG_EXPAND_SZ, REG_SZ, REG_VALUE_TYPE,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

use super::util::{ensure_com, expand_env, file_description, from_wide, to_wide};
use crate::app_id::exe_stem;
use crate::catalog::{looks_like_uninstaller, sort_apps, CachedScan};

/// How deep below each Start Menu root the scan looks (`Programs\Vendor\App.lnk` is 2).
const MAX_DEPTH: usize = 3;

const APP_PATHS: &str = r"Software\Microsoft\Windows\CurrentVersion\App Paths";

/// The `Programs` folders of the shared and the user's Start Menu.
fn start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["ProgramData", "APPDATA"] {
        if let Some(base) = std::env::var_os(var).filter(|v| !v.is_empty()) {
            roots.push(PathBuf::from(base).join(r"Microsoft\Windows\Start Menu\Programs"));
        }
    }
    roots
}

fn collect_links(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth > 1 {
                collect_links(&path, depth - 1, out);
            }
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
        {
            out.push(path);
        }
    }
}

/// The target path of a `.lnk` file, with environment variables expanded.
pub fn resolve_shortcut(path: &Path) -> Option<String> {
    ensure_com();
    let wide = to_wide(&path.to_string_lossy());
    // SAFETY: COM calls on interfaces created here; the buffer is a valid out-slice.
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist: IPersistFile = link.cast().ok()?;
        persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;
        let mut buf = [0u16; 1024];
        link.GetPath(&mut buf, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
            .ok()?;
        let raw = from_wide(&buf);
        if raw.trim().is_empty() {
            return None;
        }
        Some(expand_env(raw.trim()))
    }
}

fn is_exe(path: &str) -> bool {
    path.to_lowercase().ends_with(".exe")
}

/// Subkeys of `App Paths` under `root` as `(key name, default value)` pairs.
fn app_paths(root: HKEY) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let sub = to_wide(APP_PATHS);
    let mut key = HKEY::default();
    // SAFETY: registry handles opened here are closed before returning; buffers carry
    // their lengths.
    unsafe {
        if RegOpenKeyExW(root, PCWSTR(sub.as_ptr()), None, KEY_READ, &mut key) != ERROR_SUCCESS {
            return out;
        }
        let mut index = 0u32;
        loop {
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            let status = RegEnumKeyExW(
                key,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut len,
                None,
                None,
                None,
                None,
            );
            if status != ERROR_SUCCESS {
                break;
            }
            index += 1;
            let name = from_wide(&name[..len as usize]);
            let subkey_name = to_wide(&name);
            let mut subkey = HKEY::default();
            if RegOpenKeyExW(
                key,
                PCWSTR(subkey_name.as_ptr()),
                None,
                KEY_READ,
                &mut subkey,
            ) != ERROR_SUCCESS
            {
                continue;
            }
            let mut kind = REG_VALUE_TYPE::default();
            let mut data = vec![0u8; 2048];
            let mut size = data.len() as u32;
            let status = RegQueryValueExW(
                subkey,
                PCWSTR::null(),
                None,
                Some(&mut kind),
                Some(data.as_mut_ptr()),
                Some(&mut size),
            );
            let _ = RegCloseKey(subkey);
            if status != ERROR_SUCCESS || (kind != REG_SZ && kind != REG_EXPAND_SZ) {
                continue;
            }
            let chars: Vec<u16> = data[..size as usize]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let value = from_wide(&chars).trim().trim_matches('"').to_string();
            if !value.is_empty() {
                out.push((name, expand_env(&value)));
            }
        }
        let _ = RegCloseKey(key);
    }
    out
}

/// Scans now: Start Menu shortcuts first (their names are the ones users know), then the
/// registry; one entry per executable stem, sorted by name.
pub fn scan() -> Vec<InstalledApp> {
    let mut apps: Vec<InstalledApp> = Vec::new();
    let mut push = |name: String, target: String| {
        if !is_exe(&target) || looks_like_uninstaller(&name, &target) {
            return;
        }
        let stem = exe_stem(&target);
        if stem.is_empty() || apps.iter().any(|a| a.bundle_id == stem) {
            return;
        }
        let name = if name.trim().is_empty() {
            file_description(&target).unwrap_or_else(|| stem.clone())
        } else {
            name.trim().to_string()
        };
        apps.push(InstalledApp {
            name,
            bundle_id: stem,
            path: target,
        });
    };
    let mut links = Vec::new();
    for root in start_menu_roots() {
        collect_links(&root, MAX_DEPTH, &mut links);
    }
    for link in links {
        let Some(target) = resolve_shortcut(&link) else {
            continue;
        };
        let name = link
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        push(name, target);
    }
    for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        for (_, target) in app_paths(root) {
            if Path::new(&target).is_file() {
                push(String::new(), target);
            }
        }
    }
    sort_apps(&mut apps);
    apps
}

/// [`AppCatalog`] over the Start Menu and `App Paths`, cached for a minute.
#[derive(Debug, Default)]
pub struct WindowsAppCatalog {
    cache: CachedScan,
}

impl AppCatalog for WindowsAppCatalog {
    fn installed_apps(&self) -> CoreResult<Vec<InstalledApp>> {
        Ok(self.cache.get_or_scan(scan))
    }
}
