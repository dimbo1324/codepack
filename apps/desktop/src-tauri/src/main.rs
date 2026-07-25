// Hides the console window that would otherwise appear behind the app on Windows in a
// release build. `debug_assertions` keeps it in debug builds, where `println!` from a
// command is a legitimate debugging tool.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    codepack_desktop::run();
}
