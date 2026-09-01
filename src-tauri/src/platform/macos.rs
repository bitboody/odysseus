use std::process::{Child, Command};

use crate::run_system_command;

// TEMPLATE: only exercised via Homebrew so far — verify on a real macOS box before shipping.
pub fn install_git() -> Result<String, String> {
    if run_system_command("brew", &["--version"]).is_ok() {
        return run_system_command("brew", &["install", "git"]);
    }

    // xcode-select --install opens a non-blocking GUI installer, so it can't be driven headlessly here.
    Err(
        "Git is not installed and Homebrew was not found. Install Homebrew (https://brew.sh) or run `xcode-select --install`, then retry."
            .to_string(),
    )
}

pub fn launch_docker_desktop() -> std::io::Result<Child> {
    Command::new("open").arg("-a").arg("Docker").spawn()
}
