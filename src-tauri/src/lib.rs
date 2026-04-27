mod auth;
mod calendars;
mod commands;
mod error;
mod models;

use commands::auth_commands::{
    auth_debug_clients, auth_icloud_save, auth_refresh, auth_revoke, auth_start, auth_status,
};
use commands::calendar_commands::{
    calendars_fetch, event_create, event_delete, event_update, events_fetch,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_oauth::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            auth_start,
            auth_refresh,
            auth_revoke,
            auth_status,
            auth_icloud_save,
            auth_debug_clients,
            calendars_fetch,
            events_fetch,
            event_create,
            event_update,
            event_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
