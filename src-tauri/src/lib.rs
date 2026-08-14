mod harness;

use tauri::webview::NewWindowResponse;
use tauri::{RunEvent, WebviewUrl, WebviewWindowBuilder};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Build the window first so even a spawn failure can land on the
            // error page. The initial URL is a bundled asset; the supervisor
            // navigates it to the harness once the port is ready.
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("DeepSeek Harness")
                .inner_size(1280.0, 800.0)
                .min_inner_size(800.0, 600.0)
                .resizable(true)
                .decorations(true)
                .on_navigation(|url| {
                    let scheme = url.scheme();
                    let host = url.host_str().unwrap_or("");
                    // Allow our own pages (app origin / error data URL / blank)
                    // and the harness itself on loopback; everything else goes
                    // to the system browser and is blocked here.
                    let is_ours = scheme == "tauri"
                        || host == "tauri.localhost"
                        || scheme == "data"
                        || scheme == "about"
                        || matches!(host, "127.0.0.1" | "localhost" | "::1");
                    if is_ours {
                        true
                    } else {
                        let _ = open::that_detached(url.as_str());
                        false
                    }
                })
                .on_new_window(|url, _features| {
                    // Covers target="_blank" popups, which on_navigation does
                    // not intercept on Windows.
                    let _ = open::that_detached(url.as_str());
                    NewWindowResponse::Deny
                })
                .build()?;

            let app_handle = app.handle().clone();
            let port = harness::find_free_port();
            match harness::spawn(port) {
                Ok(child) => {
                    harness::record_pid(child.id());
                    std::thread::spawn(move || harness::supervise(port, child, app_handle));
                }
                Err(e) => {
                    eprintln!("{e}");
                    let _ = win.navigate(harness::error_url());
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the tauri application")
        .run(|_app_handle, event| match event {
            RunEvent::ExitRequested { .. } | RunEvent::Exit => harness::kill_tree(),
            _ => {}
        });
}
