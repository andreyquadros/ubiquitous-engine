// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `--shield-helper …` (the elevated system layer, an uninstall hook, a removal by hand)
    // does its work and exits here, before Tauri, the window server or the database.
    ubiqx_platform::sysblock::helper::main_if_requested(std::env::args());
    ubiqx_desktop_lib::run()
}
