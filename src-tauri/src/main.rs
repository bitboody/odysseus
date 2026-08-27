#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::PathBuf;

fn main() {
    let is_installed = check_installation_state(check_os());
    println!("{}", is_installed);
    odysseus_lib::run(is_installed);
}

fn check_os() -> &'static str {
    return std::env::consts::OS;
}

fn check_installation_state(os: &str) -> bool {
    let home_var = match os {
        "windows" => "USERPROFILE",
        "macos" | "linux" => "HOME",
        _ => return false,
    };

    std::env::var_os(home_var)
        .map(|home| {
            PathBuf::from(home)
                .join("Documents")
                .join("Odysseus Desktop")
                .join("config.json")
                .is_file()
        })
        .unwrap_or(false)
}
