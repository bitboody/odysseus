#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::Path;

fn main() {
    let is_installed = check_installation_state(check_os());
    println!("{}", is_installed);
    odysseus_lib::run(is_installed);
}

fn check_os() -> &'static str {
    return std::env::consts::OS;
}

fn check_installation_state(os: &str) -> bool {
    let config_file_path = match os {
        "windows" => std::env::var("USERPROFILE")
            .ok()
            .map(|val| format!("{}\\Documents\\Odysseus Desktop\\config.json", val)),
        "macos" | "linux" => std::env::var("HOME")
            .ok()
            .map(|val| format!("{}/Documents/Odysseus Desktop/config.json", val)),
        _ => None,
    };

    if let Some(path_str) = config_file_path {
        // Ensures it exists AND is specifically a file, not just an empty directory
        Path::new(&path_str).is_file()
    } else {
        false
    }
}
