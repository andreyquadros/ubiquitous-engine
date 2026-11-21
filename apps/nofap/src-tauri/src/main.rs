// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod card;
mod tray;

fn main() {
    // A `--shield-helper` run does its work and exits here, before Tauri, the window server
    // or the database are touched.
    ubiqx_platform::sysblock::helper::main_if_requested(std::env::args());
    app::run()
}
