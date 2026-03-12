mod ai;
mod calendar;
mod callback_server;
mod commands;
mod db;
mod email;
pub mod oauth;
mod scheduler;

use log::warn;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let google_client_id = std::env::var("KAIROS_GOOGLE_CLIENT_ID").unwrap_or_default();
    let google_client_secret = std::env::var("KAIROS_GOOGLE_CLIENT_SECRET").unwrap_or_default();
    let microsoft_client_id = std::env::var("KAIROS_MICROSOFT_CLIENT_ID").unwrap_or_default();
    let microsoft_client_secret =
        std::env::var("KAIROS_MICROSOFT_CLIENT_SECRET").unwrap_or_default();

    if google_client_id.is_empty() {
        warn!("KAIROS_GOOGLE_CLIENT_ID is not set — Google OAuth will not work");
    }
    if google_client_secret.is_empty() {
        warn!("KAIROS_GOOGLE_CLIENT_SECRET is not set — Google OAuth will not work");
    }
    if microsoft_client_id.is_empty() {
        warn!("KAIROS_MICROSOFT_CLIENT_ID is not set — Microsoft OAuth will not work");
    }
    if microsoft_client_secret.is_empty() {
        warn!("KAIROS_MICROSOFT_CLIENT_SECRET is not set — Microsoft OAuth will not work");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:kairos.db", db::migrations())
                .build(),
        )
        .manage(commands::OAuthConfig {
            google_client_id,
            google_client_secret,
            microsoft_client_id,
            microsoft_client_secret,
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_auth_url,
            commands::handle_oauth_callback,
            commands::disconnect_account,
            commands::get_valid_token,
        ])
        .setup(|app| {
            oauth::init();
            email::init();
            calendar::init();
            ai::init();
            scheduler::init();

            // Start the localhost OAuth callback server
            callback_server::start(app.handle());

            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .tooltip("Kairos")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
