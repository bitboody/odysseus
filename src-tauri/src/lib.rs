use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tauri::{Manager, Url};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(is_installed: bool) {
    tauri::Builder::default()
        .register_uri_scheme_protocol("odysseus", move |_app, request| {
            let path = request.uri().path();
            let fallback_css = include_str!("../assets/tauri-style.css");

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

            if is_installed {
                println!("Odysseus is installed. Spinning up background services...");

                std::thread::spawn(move || {
                    run_odysseus();

                    println!("Polling for localhost:7000 to serve HTTP 200...");
                    let addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();

                    loop {
                        if let Ok(mut stream) =
                            TcpStream::connect_timeout(&addr, Duration::from_millis(500))
                        {
                            let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                            
                            let request =
                                "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";

                            if stream.write_all(request.as_bytes()).is_ok() {
                                let mut buffer = [0; 1024];
                                
                                if let Ok(bytes_read) = stream.read(&mut buffer) {
                                    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
                                    
                                    let first_line = response.lines().next().unwrap_or("Empty Response");
                                    println!("Pinged localhost:7000. Server replied: {}", first_line);

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
        // 3. Point the handler to the module namespace
        .invoke_handler(tauri::generate_handler![commands::windows_installation_script])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod commands {
    use super::*; // Pulls our helper functions into this module's scope

    #[tauri::command]
    pub async fn windows_installation_script() -> (String, bool) {
        let is_admin = is_admin();
        if !is_admin {
            return (
                "Administrative privileges are required to run the installer.".to_string(),
                false,
            );
        }

        match run_system_command("git", &["--version"]) {
            Ok(output) => println!("Found Git: {}", output.trim()),
            Err(_) => {
                println!("Git not found. Attempting to install via winget...");

                let install_result = run_system_command(
                    "winget",
                    &[
                        "install", "--id", "Git.Git", "-e", "--source", "winget",
                        "--silent", "--accept-package-agreements", "--accept-source-agreements",
                    ],
                );

                match install_result {
                    Ok(_) => println!("Git installed successfully."),
                    Err(e) => {
                        return (
                            format!("Git is not installed, and automated installation failed: {e}"),
                            false,
                        );
                    }
                }
            }
        }

        match run_system_command("docker", &["info"]) {
            Ok(_) => println!("Docker CLI found and Engine is running."),
            Err(_) => {
                return (
                    "Docker is not running. Please open Docker Desktop and try again.".to_string(),
                    false,
                );
            }
        }

        let target_dir = format!(
            "{}\\Documents\\Odysseus",
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string())
        );

        match run_system_command(
            "git",
            &["clone", "https://github.com/odysseus-dev/odysseus.git", &target_dir],
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

        let env_example_path = format!("{}\\.env.example", target_dir);
        let env_path = format!("{}\\.env", target_dir);
        if !std::path::Path::new(&env_path).exists() {
            match std::fs::copy(&env_example_path, &env_path) {
                Ok(_) => println!("Successfully created .env file."),
                Err(e) => println!("Warning: Could not copy .env.example: {}", e),
            }
        }

        println!("Building Odysseus with optional extras (this will take a while)...");
        let build_status = Command::new("docker")
            .current_dir(&target_dir)
            .args(["compose", "build", "--build-arg", "INSTALL_OPTIONAL=true"])
            .stdout(Stdio::inherit()) 
            .stderr(Stdio::inherit()) 
            .status();

        match build_status {
            Ok(status) if status.success() => println!("Build successful!"),
            Ok(status) => return (format!("Docker build failed with status: {}", status), false),
            Err(e) => return (format!("Failed to execute docker build: {}", e), false),
        }

        println!("Starting Odysseus containers...");
        let up_status = Command::new("docker")
            .current_dir(&target_dir)
            .args(["compose", "up", "-d", "--build"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        match up_status {
            Ok(status) if status.success() => println!("Docker compose up executed successfully!"),
            Ok(status) => return (format!("Docker compose up failed with status: {}", status), false),
            Err(e) => return (format!("Failed to execute docker up command: {}", e), false),
        }

        println!("Windows installation script executed successfully.");
        (
            "Windows installation script executed successfully.".to_string(),
            true,
        )
    }
}

// ---------------------------------------------------------
// HELPER FUNCTIONS
// ---------------------------------------------------------
#[cfg(target_os = "windows")]
fn is_admin() -> bool {
    is_elevated::is_elevated()
}

#[cfg(target_family = "unix")]
fn is_admin() -> bool {
    unsafe { libc::geteuid() == 0 }
}

fn ensure_docker_is_running() -> Result<(), String> {
    if run_system_command("docker", &["info"]).is_ok() {
        return Ok(());
    }

    println!("Docker daemon is offline. Attempting to launch Docker Desktop...");

    #[cfg(target_os = "windows")]
    let launch_result = {
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
            std::process::Command::new(&user_path_1).spawn()
        } else if std::path::Path::new(&user_path_2).exists() {
            std::process::Command::new(&user_path_2).spawn()
        } else {
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

    let max_retries = 30; 
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
    if let Err(e) = ensure_docker_is_running() {
        println!("Error: {}", e);
        return; 
    }

    match run_system_command("docker", &["ps"]) {
        Ok(output) => {
            if output.contains("odysseus") {
                println!("Odysseus is already running.");
            } else {
                println!("Starting Odysseus...");
                let target_dir = get_odysseus_dir();

                let result = Command::new("docker")
                    .current_dir(&target_dir)
                    .args(["compose", "up", "-d"])
                    .output();

                match result {
                    Ok(out) if out.status.success() => println!("Odysseus started successfully."),
                    Ok(out) => println!("Failed to start Odysseus: {}", String::from_utf8_lossy(&out.stderr)),
                    Err(e) => println!("Failed to execute docker command: {}", e),
                }
            }
        }
        Err(_) => println!("Unexpected error checking docker ps."),
    }
}

fn close_odysseus() {
    if run_system_command("docker", &["info"]).is_err() {
        println!("Docker is offline. Assuming Odysseus is already stopped.");
        return;
    }

    match run_system_command("docker", &["ps"]) {
        Ok(output) => {
            if !output.contains("odysseus") {
                println!("Odysseus is not running.");
            } else {
                println!("Closing Odysseus...");
                let target_dir = get_odysseus_dir();

                let result = Command::new("docker")
                    .current_dir(&target_dir)
                    .args(["compose", "stop"])
                    .output();

                match result {
                    Ok(out) if out.status.success() => println!("Odysseus stopped successfully."),
                    Ok(out) => println!("Failed to stop Odysseus: {}", String::from_utf8_lossy(&out.stderr)),
                    Err(e) => println!("Failed to execute docker command: {}", e),
                }
            }
        }
        Err(_) => println!("Unexpected error checking docker ps."),
    }
}

fn get_odysseus_dir() -> String {
    #[cfg(target_os = "windows")]
    let base_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());

    #[cfg(not(target_os = "windows"))]
    let base_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

    let mut path = PathBuf::from(base_dir);
    path.push("Documents");
    path.push("Odysseus");

    path.to_string_lossy().to_string()
}

fn run_system_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute command '{cmd}': {e}"))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| format!("Command output was not valid UTF-8: {e}"))
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!(
            "Command failed with exit code {:?}: {}",
            output.status.code(),
            error_message
        ))
    }
}
