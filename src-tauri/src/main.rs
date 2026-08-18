#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::Path;

fn main() {
    println!("{}", check_installation_state(check_os()));
    odysseus_lib::run();
}

fn check_os() -> &'static str {
    return std::env::consts::OS;
}

fn check_installation_state(os: &str) -> bool {
    let config_file_path = match os {
        "windows" => std::env::var("USERPROFILE")
            .ok()
            .map(|val| format!("{}\\Documents\\Odysseus\\config.json", val)),
        "macos" | "linux" => std::env::var("HOME")
            .ok()
            .map(|val| format!("{}/Documents/Odysseus/config.json", val)),
        _ => None,
    };

    if let Some(path_str) = config_file_path {
        // Ensures it exists AND is specifically a file, not just an empty directory
        Path::new(&path_str).is_file()
    } else {
        false
    }
}
