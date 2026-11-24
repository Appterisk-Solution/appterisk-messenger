#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(windows)]
mod win;
#[cfg(windows)]
fn main() {
    win::run();
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Windows/WebView2 is required");
    std::process::exit(2);
}
