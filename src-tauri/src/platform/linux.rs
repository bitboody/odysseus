use std::process::{Child, Command};

use crate::run_system_command;

// TEMPLATE: covers the most common distro package managers — extend with zypper/apk/etc. as needed.
pub fn install_git() -> Result<String, String> {
    let package_managers: [(&str, &[&str]); 3] = [
        ("apt-get", &["install", "-y", "git"]),
        ("dnf", &["install", "-y", "git"]),
        ("pacman", &["-S", "--noconfirm", "git"]),
    ];

    for (manager, install_args) in package_managers {
        if run_system_command("which", &[manager]).is_ok() {
            let mut args = vec![manager];
            args.extend_from_slice(install_args);
            // Installing packages needs root; this assumes passwordless sudo or an interactive prompt.
            return run_system_command("sudo", &args);
        }
    }

    Err(
        "Git is not installed and no supported package manager (apt-get, dnf, pacman) was found. Install Git manually and retry."
            .to_string(),
    )
}

pub fn launch_docker_desktop() -> std::io::Result<Child> {
    Command::new("sudo")
        .args(["systemctl", "start", "docker"])
        .spawn()
}
