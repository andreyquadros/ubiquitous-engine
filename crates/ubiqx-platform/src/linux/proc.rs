//! `/proc` helpers: the executable behind a pid, its owner, and the processes of this user.

use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use crate::app_id::exe_stem;

/// Path of the executable of `pid` (`/proc/<pid>/exe`), without the ` (deleted)` marker a
/// replaced binary carries. `None` when the process is gone or belongs to another user.
pub fn exe_path(pid: u32) -> Option<PathBuf> {
    let link = std::fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    let s = link.to_string_lossy();
    Some(PathBuf::from(
        s.strip_suffix(" (deleted)").unwrap_or(&s).to_string(),
    ))
}

/// App id of `pid`: the executable's stem.
pub fn exe_stem_of(pid: u32) -> Option<String> {
    let path = exe_path(pid)?;
    let stem = exe_stem(&path.to_string_lossy());
    (!stem.is_empty()).then_some(stem)
}

/// Owner (uid) of `pid`.
pub fn uid_of(pid: u32) -> Option<u32> {
    std::fs::metadata(format!("/proc/{pid}"))
        .ok()
        .map(|m| m.uid())
}

/// Uid of this process.
pub fn my_uid() -> Option<u32> {
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
}

/// Every pid listed in `/proc`.
pub fn pids() -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok())
        .collect()
}

/// Pids of this user's processes whose app id is `stem`, excluding this process.
pub fn pids_of_stem(stem: &str) -> Vec<u32> {
    let stem = stem.trim().to_lowercase();
    let me = std::process::id();
    let uid = my_uid();
    pids()
        .into_iter()
        .filter(|&pid| pid != me)
        .filter(|&pid| uid.is_none() || uid_of(pid) == uid)
        .filter(|&pid| exe_stem_of(pid).as_deref() == Some(stem.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_this_process() {
        let me = std::process::id();
        assert!(exe_path(me).is_some());
        let stem = exe_stem_of(me).unwrap();
        assert!(!stem.is_empty());
        assert_eq!(uid_of(me), my_uid());
        assert!(pids().contains(&me));
        // This process is excluded from its own stem's list.
        assert!(!pids_of_stem(&stem).contains(&me));
    }
}
