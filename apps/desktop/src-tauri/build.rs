//! Stamps the build identity into the desktop binary (`UBIQX_BUILD_*`, read by `lib.rs`
//! through `env!`) and runs the Tauri build steps.
//!
//! CI exports the values through `scripts/build-info.sh` before building; a local build asks
//! git for the same numbers, and a checkout without git is a development build
//! (`0` / `0` / `dev` / empty), which never checks for updates automatically.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Feed URL compiled in when `UBIQX_UPDATE_FEED_URL` is not set: the product's rolling
/// GitHub release (kept in sync with `ubiqx_core::update::DEFAULT_FEED_URL`).
const DEFAULT_FEED_URL: &str =
    "https://github.com/andreyquadros/ubiquitous-engine/releases/download/continuous/latest.json";

fn main() {
    let root = workspace_root();
    stamp("UBIQX_BUILD_EPOCH", "0", || {
        git(&root, &["log", "-1", "--format=%ct"])
    });
    stamp("UBIQX_BUILD_NUMBER", "0", || {
        git(&root, &["rev-list", "--count", "HEAD"])
    });
    stamp("UBIQX_BUILD_SHA", "dev", || {
        git(&root, &["rev-parse", "--short=7", "HEAD"])
    });
    stamp("UBIQX_BUILD_BRANCH", "", || {
        git(&root, &["rev-parse", "--abbrev-ref", "HEAD"]).filter(|b| b != "HEAD")
    });
    stamp("UBIQX_UPDATE_FEED_URL", DEFAULT_FEED_URL, || None);

    // A new commit changes the stamp of a local build: watch the files git moves then
    // (asked from git, since `.git` is a file in a worktree).
    if let Some(git_dir) = git(&root, &["rev-parse", "--absolute-git-dir"]).map(PathBuf::from) {
        println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
        if let Some(head) = std::fs::read_to_string(git_dir.join("HEAD"))
            .ok()
            .and_then(|h| h.trim().strip_prefix("ref: ").map(str::to_string))
        {
            println!("cargo:rerun-if-changed={}", git_dir.join(head).display());
        }
    }

    tauri_build::build()
}

/// Emits `cargo:rustc-env=NAME=value`, taking the value from the environment variable of
/// the same name when set (CI), from `fallback()` otherwise, and `default` as a last resort.
fn stamp(name: &str, default: &str, fallback: impl FnOnce() -> Option<String>) {
    println!("cargo:rerun-if-env-changed={name}");
    let value = std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(fallback)
        .unwrap_or_else(|| default.to_string());
    println!("cargo:rustc-env={name}={value}");
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// `apps/desktop/src-tauri` → the workspace root, where `.git` lives.
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or(manifest)
}
