use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::time::Duration;
use tauri::{Manager, Url};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(os_name: String, is_installed: bool) {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![windows_installation_script])
        // 1. Create a secure custom protocol to serve your injected HTML strings
        .register_uri_scheme_protocol("odysseus", move |_app, request| {
            let path = request.uri().path();
            let fallback_css = include_str!("../assets/tauri-style.css");

            // Serve the installer or 404 based on the virtual path
            let html = if path.contains("installer") {
                include_str!("../assets/tauri-installer.html").replace(
                    "</head>",
                    &format!("<style>\n{fallback_css}\n</style>\n</head>"),
                )
            } else {
                include_str!("../assets/tauri-404.html").replace(
                    "</head>",
                    &format!("<style>\n{fallback_css}\n</style>\n</head>"),
                )
            };

            // Return it as a highly compliant HTTP response
            tauri::http::Response::builder()
                .status(200) // Explicitly say "200 OK"
                .header("Content-Type", "text/html; charset=utf-8")
                .header("Access-Control-Allow-Origin", "*") // Prevent cross-origin blocks
                .body(html.into_bytes())
                .unwrap()
        })
        .setup(move |app| {
            println!("OS: {}", os_name);
            println!("Is Installed: {}", is_installed);

            // 2. Format the URL based on the Operating System
            // Windows WebView2 blocks top-level navigation to custom protocols.
            // Tauri bypasses this by intercepting "http://<scheme>.localhost" for us.
            #[cfg(target_os = "windows")]
            let (fallback_url, installer_url) = (
                "http://odysseus.localhost/404".to_string(),
                "http://odysseus.localhost/installer".to_string(),
            );

            #[cfg(not(target_os = "windows"))]
            let (fallback_url, installer_url) = (
                "odysseus://localhost/404".to_string(),
                "odysseus://localhost/installer".to_string(),
            );

            let target_url_str = get_target_url(fallback_url, installer_url, is_installed);
            let target_url = Url::parse(&target_url_str).unwrap();

            // Grab the window created by tauri.conf.json
            let window = app.get_webview_window("main").unwrap();

            // Navigate it to the correct URL
            let _ = window.navigate(target_url);

            println!("Target URL: {}", target_url_str);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Check if the current user has administrative privileges
#[cfg(target_os = "windows")]
fn is_admin() -> bool {
    is_elevated::is_elevated()
}

#[cfg(target_family = "unix")]
fn is_admin() -> bool {
    // Ensure `libc` is added to your Cargo.toml dependencies
    unsafe { libc::geteuid() == 0 }
}

// OS Specific Installation Scripts
#[tauri::command]
fn windows_installation_script() -> (String, bool) {
    let is_admin = is_admin();
    if !is_admin {
        return (
            "Administrative privileges are required to run the installer.".to_string(),
            false,
        );
    }

    // 1. Check for Git (If missing, attempt silent installation via winget)
    match run_system_command("git", &["--version"]) {
        Ok(output) => println!("Found Git: {}", output.trim()),
        Err(_) => {
            println!("Git not found. Attempting to install via winget...");

            // Run winget to silently install Git
            let install_result = run_system_command(
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
            );

            match install_result {
                Ok(_) => println!("Git installed successfully."),
                Err(e) => {
                    return (
                        format!("Git is not installed, and automated installation via winget failed: {e}"),
                        false,
                    );
                }
            }
        }
    }

    // 2. Check for Docker AND check if the daemon is actually running
    match run_system_command("docker", &["info"]) {
        Ok(_) => println!("Docker CLI found and Engine is running."),
        Err(_) => {
            return (
                "Docker is either not installed or the Docker Engine is not running. Please open Docker Desktop, wait for it to fully initialize, and try again.".to_string(),
                false,
            );
        }
    }

    // 3. Clone Repository
    let target_dir = format!(
        "{}\\Documents\\Odysseus",
        std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string())
    );

    match run_system_command(
        "git",
        &[
            "clone",
            "https://github.com/odysseus-dev/odysseus.git",
            &target_dir,
        ],
    ) {
        Ok(output) => println!("Git repository cloned successfully: {}", output.trim()),
        Err(e) => {
            if std::path::Path::new(&target_dir).exists() {
                println!("Directory already exists, skipping clone and proceeding.");
            } else {
                return (format!("Failed to clone repository: {e}"), false);
            }
        }
    }

    // Copy .env.example to .env natively using Rust
    if !std::path::Path::new(".env").exists() {
        match std::fs::copy(".env.example", ".env") {
            Ok(_) => println!("Successfully created .env file."),
            Err(e) => println!("Warning: Could not copy .env.example: {}", e),
        }
    }

    // Build the Docker images WITH the optional arguments
    println!("Building Odysseus with optional extras (this will take a while)...");
    match run_system_command(
        "docker",
        &["compose", "build", "--build-arg", "INSTALL_OPTIONAL=true"],
    ) {
        Ok(_) => println!("Build successful!"),
        Err(_) => {
            return (
                "Failed to build Odysseus Docker images. Please check if Docker Desktop is running.".to_string(),
                false,
            );
        }
    }

    // Start the containers in the background
    match run_system_command(
        "docker",
        &[
            "compose",
            "-f",
            &format!("{}\\docker-compose.yml", target_dir),
            "up",
            "-d",
            "--build",
        ],
    ) {
        Ok(output) => println!("Docker compose executed successfully: {}", output.trim()),
        Err(e) => {
            return (format!("Docker compose failed to execute: {e}"), false);
        }
    }

    println!("Windows installation script executed successfully.");
    (
        "Windows installation script executed successfully.".to_string(),
        true,
    )
}

// Helper Functions
fn get_target_url(fallback_url: String, installer_url: String, is_installed: bool) -> String {
    if !is_installed {
        return installer_url;
    }

    let addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();
    let is_online = TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok();

    if is_online {
        "http://localhost:7000".to_string()
    } else {
        fallback_url
    }
}

fn run_system_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute command '{cmd}': {e}"))?;

    if output.status.success() {
        // Convert stdout bytes to a UTF-8 String
        String::from_utf8(output.stdout)
            .map_err(|e| format!("Command output was not valid UTF-8: {e}"))
    } else {
        // If the command returned a non-zero exit code, capture stderr
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!(
            "Command failed with exit code {:?}: {}",
            output.status.code(),
            error_message
        ))
    }
}
