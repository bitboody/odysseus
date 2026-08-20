use std::io::{Read, Write}; // <-- Add this line
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tauri::{Manager, Url};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(is_installed: bool) {
    tauri::Builder::default()
        .register_uri_scheme_protocol("odysseus", move |_app, request| {
            let path = request.uri().path();
            let fallback_css = include_str!("../assets/tauri-style.css");

            // 1. We now handle the root path ("/" or "") directly based on installation status
            let html = if path.contains("installer")
                || (!is_installed && (path == "/" || path.is_empty()))
            {
                include_str!("../assets/tauri-installer.html").replace(
                    "</head>",
                    &format!("<style>\n{fallback_css}\n</style>\n</head>"),
                )
            } else if path.contains("loading") || (is_installed && (path == "/" || path.is_empty()))
            {
                include_str!("../assets/tauri-loading.html").replace(
                    "</head>",
                    &format!("<style>\n{fallback_css}\n</style>\n</head>"),
                )
            } else {
                include_str!("../assets/tauri-404.html").replace(
                    "</head>",
                    &format!("<style>\n{fallback_css}\n</style>\n</head>"),
                )
            };

            tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", "text/html; charset=utf-8")
                .header("Access-Control-Allow-Origin", "*")
                .body(html.into_bytes())
                .unwrap()
        })
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let window = app_handle.get_webview_window("main").unwrap();

            // KICKSTART THE ROUTER: Force the webview to hit our custom protocol immediately
            #[cfg(target_os = "windows")]
            let initial_url = if is_installed {
                "http://odysseus.localhost/loading"
            } else {
                "http://odysseus.localhost/installer"
            };

            #[cfg(not(target_os = "windows"))]
            let initial_url = if is_installed {
                "odysseus://localhost/loading"
            } else {
                "odysseus://localhost/installer"
            };

            let _ = window.navigate(Url::parse(initial_url).unwrap());

            // 2. Start the background services
            if is_installed {
                println!("Odysseus is installed. Spinning up background services...");

                std::thread::spawn(move || {
                    run_odysseus();

                    println!("Polling for localhost:7000 to serve HTTP 200...");
                    let addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();

                    loop {
                        // Keep a fast connection timeout, but...
                        if let Ok(mut stream) =
                            TcpStream::connect_timeout(&addr, Duration::from_millis(500))
                        {
                            // 1. Give the server up to 3 seconds to actually generate the HTTP response
                            let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                            
                            let request =
                                "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";

                            if stream.write_all(request.as_bytes()).is_ok() {
                                let mut buffer = [0; 1024];
                                
                                if let Ok(bytes_read) = stream.read(&mut buffer) {
                                    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
                                    
                                    // Extract just the first line (e.g., "HTTP/1.1 200 OK")
                                    let first_line = response.lines().next().unwrap_or("Empty Response");
                                    println!("Pinged localhost:7000. Server replied: {}", first_line);

                                    // 2. Accept 2xx (Success) OR 3xx (Redirects)
                                    if first_line.starts_with("HTTP/1.1 20")
                                        || first_line.starts_with("HTTP/1.0 20")
                                        || first_line.starts_with("HTTP/1.1 30")
                                        || first_line.starts_with("HTTP/1.0 30")
                                    {
                                        println!(
                                            "Server is fully booted and responsive! Navigating to live app..."
                                        );

                                        let thread_window =
                                            app_handle.get_webview_window("main").unwrap();
                                        let _ = thread_window
                                            .navigate(Url::parse("http://localhost:7000").unwrap());

                                        break;
                                    }
                                } else {
                                    println!("TCP connected, but timed out waiting for the HTTP response.");
                                }
                            }
                        } else {
                            println!("Waiting for port 7000 to open...");
                        }

                        thread::sleep(Duration::from_millis(1000));
                    }

                });
            } else {
                println!("Odysseus is not installed. Waiting for user in installer UI...");
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api: _, .. } => {
                println!(
                    "User clicked X on window: {}. Shutting down containers...",
                    window.label()
                );
                close_odysseus();
                println!("Teardown complete. Goodbye!");
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![windows_installation_script])
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

fn ensure_docker_is_running() -> Result<(), String> {
    if run_system_command("docker", &["info"]).is_ok() {
        return Ok(());
    }

    println!("Docker daemon is offline. Attempting to launch Docker Desktop...");

    // OS-specific launch commands (Fire-and-forget using .spawn())
    #[cfg(target_os = "windows")]
    let launch_result = {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());

        // Check standard user-level paths vs system-wide path
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
            std::process::Command::new(&user_path_1).spawn()
        } else if std::path::Path::new(&user_path_2).exists() {
            std::process::Command::new(&user_path_2).spawn()
        } else {
            // Fallback to the traditional admin installation path
            std::process::Command::new(system_path).spawn()
        }
    };

    #[cfg(target_os = "macos")]
    let launch_result = Command::new("open").arg("-a").arg("Docker").spawn();

    #[cfg(target_os = "linux")]
    let launch_result = Command::new("sudo")
        .args(&["systemctl", "start", "docker"])
        .spawn();

    if let Err(e) = launch_result {
        return Err(format!("Could not launch Docker automatically: {}", e));
    }

    // Poll until the engine is responsive
    let max_retries = 30; // 60 seconds total wait time
    for attempt in 1..=max_retries {
        if run_system_command("docker", &["info"]).is_ok() {
            println!("Docker daemon is now online and ready!");
            return Ok(());
        }
        println!(
            "Waiting for Docker engine to initialize... (Attempt {}/{})",
            attempt, max_retries
        );
        thread::sleep(Duration::from_secs(2));
    }

    Err("Docker daemon took too long to start. Please check Docker Desktop manually.".to_string())
}

fn run_odysseus() {
    // Guarantee Docker is awake before doing anything else
    if let Err(e) = ensure_docker_is_running() {
        println!("Error: {}", e);
        return; // Bail out safely without crashing the UI
    }

    match run_system_command("docker", &["ps"]) {
        Ok(output) => {
            if output.contains("odysseus") {
                println!("Odysseus is already running.");
            } else {
                println!("Starting Odysseus...");
                let compose_file = get_compose_file_path();

                match run_system_command("docker", &["compose", "-f", &compose_file, "up", "-d"]) {
                    Ok(_) => println!("Odysseus started successfully."),
                    Err(e) => println!("Failed to start Odysseus: {}", e),
                }
            }
        }
        Err(_) => println!("Unexpected error checking docker ps."),
    }
}

fn close_odysseus() {
    // If Docker is offline, the containers are dead. Exit immediately.
    if run_system_command("docker", &["info"]).is_err() {
        println!("Docker is offline. Assuming Odysseus is already stopped.");
        return;
    }

    // If Docker is online, safely stop the containers
    match run_system_command("docker", &["ps"]) {
        Ok(output) => {
            if !output.contains("odysseus") {
                println!("Odysseus is not running.");
            } else {
                println!("Closing Odysseus...");
                let compose_file = get_compose_file_path();

                match run_system_command("docker", &["compose", "-f", &compose_file, "stop"]) {
                    Ok(_) => println!("Odysseus stopped successfully."),
                    Err(e) => println!("Failed to stop Odysseus: {}", e),
                }
            }
        }
        Err(_) => println!("Unexpected error checking docker ps."),
    }
}

// Helper Functions
fn get_compose_file_path() -> String {
    // 1. Get the correct Home Directory based on the OS
    #[cfg(target_os = "windows")]
    let base_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());

    #[cfg(not(target_os = "windows"))]
    let base_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

    // 2. Use PathBuf to safely construct the path with the correct OS slashes
    let mut path = PathBuf::from(base_dir);
    path.push("Documents");
    path.push("Odysseus");
    path.push("docker-compose.yml");

    // 3. Convert back to a String for the command runner
    path.to_string_lossy().to_string()
}

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
