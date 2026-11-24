#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(windows)]
mod win;
#[cfg(windows)]
fn main() {
    if let Err(error) = win::run() {
        eprintln!("Native control startup failed: {error}");
        std::process::exit(1);
    }
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Windows/WebView2 is required");
    std::process::exit(2);
}
