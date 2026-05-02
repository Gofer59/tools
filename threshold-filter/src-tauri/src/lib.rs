pub mod commands;
pub mod config;
pub mod overlay;

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use tauri::{Manager, Emitter};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

pub struct AppState {
    pub config: Arc<RwLock<config::Config>>,
    pub overlay_child: Arc<Mutex<Option<std::process::Child>>>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let dir = handle.path().app_local_data_dir().expect("app_local_data_dir");
                let cfg = config::load_or_default(&dir);
                let auto_start = cfg.auto_start_overlay;

                let state = AppState {
                    config: Arc::new(RwLock::new(cfg)),
                    overlay_child: Arc::new(Mutex::new(None)),
                };
                handle.manage(state);

                if auto_start {
                    let exe = std::env::current_exe().expect("current_exe");
                    match std::process::Command::new(&exe).arg("--daemon").spawn() {
                        Ok(child) => {
                            if let Some(state) = handle.try_state::<AppState>() {
                                *state.overlay_child.lock().await = Some(child);
                            }
                            let _ = handle.emit(
                                "overlay-state",
                                serde_json::json!({"running": true}),
                            );
                        }
                        Err(e) => {
                            eprintln!("[threshold-filter] auto-start overlay failed: {e}");
                        }
                    }
                }
            });

            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, ev| match ev.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        // Best-effort: kill overlay child before exit.
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(mut guard) = state.overlay_child.try_lock() {
                                if let Some(mut child) = guard.take() {
                                    let _ = child.kill();
                                }
                            }
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // Minimize to tray on close.
            if let Some(w) = app.get_webview_window("main") {
                let w_clone = w.clone();
                w.on_window_event(move |ev| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
                        api.prevent_close();
                        let _ = w_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::update_config,
            commands::overlay::start_overlay,
            commands::overlay::stop_overlay,
            commands::overlay::is_overlay_running,
            commands::hotkey::test_hotkey,
        ])
        .run(tauri::generate_context!())
        .expect("error while running threshold-filter");
}
