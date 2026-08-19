use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use tauri::{Manager, Url};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(os_name: String, is_installed: bool) {
    tauri::Builder::default()
        .setup(move |app| {

            println!("{}", os_name);
            println!("{}", is_installed);

            let fallback_css = include_str!("../assets/tauri-style.css");
            let fallback_html = include_str!("../assets/tauri-404.html").replace(
                "<link rel=\"stylesheet\" href=\"tauri-style.css\">",
                &format!("<style>{fallback_css}</style>"),
            );
            let fallback_data_uri = format!(
                "data:text/html;charset=utf-8,{}",
                fallback_html.replace('#', "%23")
            );

            let mut installer_data_uri= String::new();
            if !is_installed {
                let installer_html= include_str!("../assets/tauri-installer.html").replace(
                    "<link rel=\"stylesheet\" href=\"tauri-style.css\">",
                    &format!("<style>{fallback_css}</style>"),
                );
                installer_data_uri = format!(
                    "data:text/html;charset=utf-8,{}",
                    installer_html.replace('#', "%23")
                );
            }

            let target_url_str = get_target_url(fallback_data_uri, installer_data_uri, is_installed);
            let target_url = Url::parse(&target_url_str).unwrap();

            // Grab the window created by tauri.conf.json
            let window = app.get_webview_window("main").unwrap();

            // Navigate it to the correct URL
            window.navigate(target_url);

            println!("Target URL: {}", target_url_str);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
