#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::Path;

fn main() {
    odysseus_lib::run();
}

fn check_installation_state(os: &str) -> bool {
    let config_file_path = match os {
        "windows" => {
            std::env::var("USERPROFILE").ok().map(|val| format!("{}\\Documents\\Odysseus\\config.json", val))
        }
        "macos" | "linux" => {
            std::env::var("HOME").ok().map(|val| format!("{}/Documents/Odysseus/config.json", val))
        }
        _ => None,
    };

    if let Some(path_str) = config_file_path {
        // Ensures it exists AND is specifically a file, not just an empty directory
        Path::new(&path_str).is_file()
    } else {
        false
    }
}
