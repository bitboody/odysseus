use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use tauri::{Url, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();

            let fallback_css = include_str!("../assets/tauri-style.css");
            let fallback_html = include_str!("../assets/tauri-404.html")
                .replace(
                    "<link rel=\"stylesheet\" href=\"tauri-style.css\">",
                    &format!("<style>{fallback_css}</style>"),
                );
            let data_uri = format!(
                "data:text/html;charset=utf-8,{}",
                fallback_html.replace('#', "%23")
            );

            // Call the logic directly or via helper to get the initial Url
            let is_online = TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok();
            let target_url_str = if is_online {
                "http://localhost:7000".to_string()
            } else {
                data_uri.clone()
            };
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
fn get_target_url(fallback_url: String) -> String {
    let addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();
    let is_online = TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok();

    if is_online {
        "http://localhost:7000".to_string() 
    } else {
        fallback_url
    }
}
