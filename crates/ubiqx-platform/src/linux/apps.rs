//! Installed applications from `.desktop` entries (XDG, system, user and Flatpak exports).
//! The app id is the executable's stem taken from `Exec`; `Name` is the human name.
//! Cached for a minute.

use std::path::{Path, PathBuf};

use ubiqx_core::ports::AppCatalog;
use ubiqx_core::{CoreResult, InstalledApp};

use crate::app_id::exe_stem;
use crate::catalog::{looks_like_uninstaller, sort_apps, CachedScan};

/// The folders scanned, in order (the first entry per executable wins).
pub fn roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(home) = &home {
        roots.push(home.join(".local/share/applications"));
    }
    roots.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    if let Some(home) = &home {
        roots.push(home.join(".local/share/flatpak/exports/share/applications"));
    }
    roots
}

/// What a `.desktop` file says, once parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    pub name: String,
    /// The executable's stem derived from `Exec`.
    pub app_id: String,
    /// The executable as written in `Exec` (a path or a bare command).
    pub exec: String,
}

/// Parses the `[Desktop Entry]` group. `None` for entries that are not applications, are
/// hidden (`NoDisplay`, `Hidden`) or have no usable `Exec`.
pub fn parse_desktop_entry(text: &str) -> Option<DesktopEntry> {
    let mut in_entry = false;
    let mut name: Option<String> = None;
    let mut exec: Option<String> = None;
    let mut kind: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            if in_entry {
                break;
            }
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "Name" => name = Some(value.to_string()),
            "Exec" => exec = Some(value.to_string()),
            "Type" => kind = Some(value.to_string()),
            "NoDisplay" | "Hidden" if value.eq_ignore_ascii_case("true") => return None,
            _ => {}
        }
    }
    if kind.as_deref().is_some_and(|k| k != "Application") {
        return None;
    }
    let exec = exec.filter(|e| !e.is_empty())?;
    let command = executable_of_exec(&exec)?;
    let app_id = exe_stem(&command);
    if app_id.is_empty() {
        return None;
    }
    let name = name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| app_id.clone());
    if looks_like_uninstaller(&name, &command) {
        return None;
    }
    Some(DesktopEntry {
        name,
        app_id,
        exec: command,
    })
}

/// The executable an `Exec` line runs: the first word, skipping `env VAR=x` prefixes and
/// looking through `flatpak run --command=<exe>` (whose sandboxed process is what the
/// window will belong to). Quotes are removed; field codes (`%U`) never reach here.
pub fn executable_of_exec(exec: &str) -> Option<String> {
    let words: Vec<String> = split_exec(exec);
    let mut iter = words.iter().peekable();
    let mut first = iter.next()?.clone();
    if exe_stem(&first) == "env" {
        // `env FOO=bar prog args`: skip the assignments.
        loop {
            let next = iter.next()?.clone();
            if next.contains('=') {
                continue;
            }
            first = next;
            break;
        }
    }
    if exe_stem(&first) == "flatpak" {
        if let Some(cmd) = words
            .iter()
            .find_map(|w| w.strip_prefix("--command="))
            .filter(|c| !c.is_empty())
        {
            return Some(cmd.to_string());
        }
        // `flatpak run org.mozilla.firefox`: the app id's last label is usually the binary.
        let app = words
            .iter()
            .rev()
            .find(|w| !w.starts_with('-') && w.contains('.'))?;
        return app.rsplit('.').next().map(|s| s.to_lowercase());
    }
    if first == "sh" || first == "bash" || first == "/bin/sh" || first == "/bin/bash" {
        // `sh -c "cd x && exec prog"`: not worth guessing.
        return None;
    }
    Some(first)
}

/// Splits an `Exec` value into words, honouring double quotes and backslash escapes.
fn split_exec(exec: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => quoted = !quoted,
            '\\' => {
                if let Some(n) = chars.next() {
                    current.push(n);
                }
            }
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn scan_dir(dir: &Path, out: &mut Vec<InstalledApp>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "desktop"))
        .collect();
    files.sort();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Some(entry) = parse_desktop_entry(&text) else {
            continue;
        };
        if out.iter().any(|a| a.bundle_id == entry.app_id) {
            continue;
        }
        out.push(InstalledApp {
            name: entry.name,
            bundle_id: entry.app_id,
            path: file.to_string_lossy().to_string(),
        });
    }
}

/// Scans the roots now: one entry per executable stem, sorted by name.
pub fn scan() -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    for root in roots() {
        scan_dir(&root, &mut apps);
    }
    sort_apps(&mut apps);
    apps
}

/// [`AppCatalog`] over the `.desktop` folders, cached for a minute.
#[derive(Debug, Default)]
pub struct LinuxAppCatalog {
    cache: CachedScan,
}

impl LinuxAppCatalog {
    /// The `.desktop` name of the application whose executable stem is `app_id`.
    pub fn name_of(&self, app_id: &str) -> Option<String> {
        self.cache.with(scan, |apps| {
            apps.iter()
                .find(|a| a.bundle_id == app_id)
                .map(|a| a.name.clone())
        })
    }
}

impl AppCatalog for LinuxAppCatalog {
    fn installed_apps(&self) -> CoreResult<Vec<InstalledApp>> {
        Ok(self.cache.get_or_scan(scan))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_desktop_entry() {
        let entry = parse_desktop_entry(
            "[Desktop Entry]\nVersion=1.0\nName=Firefox Web Browser\nName[pt_BR]=Navegador Firefox\nExec=firefox %u\nType=Application\n\n[Desktop Action new-window]\nName=Open a New Window\nExec=firefox --new-window\n",
        )
        .unwrap();
        assert_eq!(entry.name, "Firefox Web Browser");
        assert_eq!(entry.app_id, "firefox");
        assert_eq!(entry.exec, "firefox");

        let entry = parse_desktop_entry(
            "[Desktop Entry]\nName=Visual Studio Code\nExec=/usr/share/code/code --unity-launch %F\nType=Application\n",
        )
        .unwrap();
        assert_eq!(entry.app_id, "code");
        assert_eq!(entry.exec, "/usr/share/code/code");
    }

    #[test]
    fn skips_hidden_and_non_applications() {
        assert!(parse_desktop_entry(
            "[Desktop Entry]\nName=X\nExec=x\nType=Application\nNoDisplay=true\n"
        )
        .is_none());
        assert!(parse_desktop_entry(
            "[Desktop Entry]\nName=X\nExec=x\nType=Application\nHidden=true\n"
        )
        .is_none());
        assert!(
            parse_desktop_entry("[Desktop Entry]\nName=Link\nType=Link\nURL=https://x\n").is_none()
        );
        assert!(parse_desktop_entry("[Desktop Entry]\nName=No exec\nType=Application\n").is_none());
        assert!(parse_desktop_entry("[Other]\nName=X\nExec=x\n").is_none());
        assert!(parse_desktop_entry(
            "[Desktop Entry]\nName=Uninstall Thing\nExec=/opt/thing/uninstall\nType=Application\n"
        )
        .is_none());
    }

    #[test]
    fn executables_from_exec_lines() {
        assert_eq!(executable_of_exec("firefox %u").as_deref(), Some("firefox"));
        assert_eq!(
            executable_of_exec("env GDK_BACKEND=x11 /opt/app/bin/app --flag").as_deref(),
            Some("/opt/app/bin/app")
        );
        assert_eq!(
            executable_of_exec(
                "/usr/bin/flatpak run --branch=stable --arch=x86_64 --command=firefox --file-forwarding org.mozilla.firefox @@u %u @@"
            )
            .as_deref(),
            Some("firefox")
        );
        assert_eq!(
            executable_of_exec("flatpak run com.spotify.Client").as_deref(),
            Some("client")
        );
        assert_eq!(
            executable_of_exec("\"/opt/My App/my app\" --x").as_deref(),
            Some("/opt/My App/my app")
        );
        assert_eq!(executable_of_exec("sh -c \"cd x && exec y\""), None);
        assert_eq!(executable_of_exec(""), None);
        assert_eq!(
            exe_stem(&executable_of_exec("/usr/lib/x/Some\\ App %F").unwrap()),
            "some app"
        );
    }

    #[test]
    fn scans_a_folder_and_dedupes_by_executable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("b-code.desktop"),
            "[Desktop Entry]\nName=Code\nExec=/usr/bin/code %F\nType=Application\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("a-code-oss.desktop"),
            "[Desktop Entry]\nName=Code - OSS\nExec=/usr/bin/code\nType=Application\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("zed.desktop"),
            "[Desktop Entry]\nName=Alacritty\nExec=alacritty\nType=Application\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("notes.txt"), "ignored").unwrap();
        let mut apps = Vec::new();
        scan_dir(dir.path(), &mut apps);
        sort_apps(&mut apps);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].name, "Alacritty");
        assert_eq!(apps[0].bundle_id, "alacritty");
        // Files are read in name order: `a-code-oss` wins the `code` stem.
        assert_eq!(apps[1].name, "Code - OSS");
        assert!(apps[1].path.ends_with("a-code-oss.desktop"));
    }

    #[test]
    fn catalog_is_cached_and_answers_names() {
        let catalog = LinuxAppCatalog::default();
        let apps = catalog.installed_apps().unwrap();
        // Whatever this host has, names must be non-empty and sorted.
        for a in &apps {
            assert!(!a.name.is_empty());
            assert!(!a.bundle_id.is_empty());
        }
        let names: Vec<String> = apps.iter().map(|a| a.name.to_lowercase()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert_eq!(catalog.name_of("definitely-not-installed-xyz"), None);
    }
}
