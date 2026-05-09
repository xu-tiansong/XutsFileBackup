mod commands;
mod db;
mod error;
mod scheduler;
mod search;
mod sync;
mod tags;
mod task_manager;
mod watcher;

use db::DbState;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use watcher::TaskHandles;

/// On startup, start background services for all enabled non-manual tasks.
async fn init_background_tasks(app: tauri::AppHandle) {
    let tasks: Vec<i64> = {
        let db = app.state::<DbState>();
        let conn = match db.0.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut stmt = match conn.prepare(
            "SELECT id FROM tasks WHERE enabled = 1 AND trigger_type != 'manual'",
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        stmt.query_map([], |row| row.get(0))
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    };

    for task_id in tasks {
        if let Err(e) = watcher::start(task_id, app.clone()).await {
            eprintln!("Failed to start background task {}: {}", task_id, e);
        }
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &sep, &quit])?;

    let icon = app
        .default_window_icon()
        .expect("no window icon")
        .clone();

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("XutsFileBackup")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(TaskHandles(Mutex::new(std::collections::HashMap::new())))
        .setup(|app| {
            // Database
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("xuts.db");
            let conn = rusqlite::Connection::open(&db_path)?;
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
            db::migrations::run(&conn)?;
            app.manage(DbState(Mutex::new(conn)));

            // System tray
            setup_tray(app)?;

            // Minimize to tray on close
            let window = app.get_webview_window("main").unwrap();
            let w = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = w.hide();
                }
            });

            // Background sync services
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                init_background_tasks(handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Sync
            commands::sync_task,
            commands::start_task_watcher,
            commands::stop_task_watcher,
            // Tasks
            commands::list_tasks,
            commands::get_task,
            commands::create_task,
            commands::update_task,
            commands::delete_task,
            commands::set_task_enabled,
            // Tags
            commands::create_tag,
            commands::update_tag,
            commands::delete_tag,
            commands::list_tags,
            commands::get_tag_tree,
            // File-Tag associations
            commands::add_file_tag,
            commands::remove_file_tag,
            commands::get_file_tags,
            // Search
            commands::search_files,
            commands::get_task_files,
            commands::get_sync_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
