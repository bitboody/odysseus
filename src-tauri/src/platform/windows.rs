use std::process::{Child, Command};

use crate::run_system_command;

pub fn install_git() -> Result<String, String> {
    run_system_command(
        "winget",
        &[
            "install",
            "--id",
            "Git.Git",
            "-e",
            "--source",
            "winget",
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ],
    )
}

pub fn launch_docker_desktop() -> std::io::Result<Child> {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());

    let user_path_1 = format!(
        "{}\\Programs\\DockerDesktop\\Docker Desktop.exe",
        local_app_data
    );
    let user_path_2 = format!(
        "{}\\Programs\\Docker\\Docker\\Docker Desktop.exe",
        local_app_data
    );
    let system_path = "C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe";

    if std::path::Path::new(&user_path_1).exists() {
        Command::new(&user_path_1).spawn()
    } else if std::path::Path::new(&user_path_2).exists() {
        Command::new(&user_path_2).spawn()
    } else {
        Command::new(system_path).spawn()
    }
}
