use crate::AppState;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn test_hotkey(
    app: AppHandle,
    _state: State<'_, AppState>,
    which: String,
) -> Result<(), String> {
    // Signal that we're armed and waiting for a keypress.
    let _ = app.emit("hotkey-test-armed", serde_json::json!({"which": which}));

    let app2 = app.clone();
    let which2 = which.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();

    std::thread::spawn(move || {
        capture_next_key(app2, which2, stop2);
    });

    // The rdev listener runs in its own thread; we return immediately.
    // The frontend waits for the "hotkey-captured" event.
    Ok(())
}

fn capture_next_key(app: AppHandle, which: String, stop: Arc<AtomicBool>) {
    let captured: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));
    let captured2 = captured.clone();
    let stop2 = stop.clone();
    let app2 = app.clone();
    let which2 = which.clone();

    let _ = rdev::listen(move |ev| {
        if stop2.load(Ordering::SeqCst) {
            return;
        }
        if let rdev::EventType::KeyPress(key) = ev.event_type {
            let name = format!("{:?}", key);
            let mut c = captured2.lock().unwrap();
            if c.is_none() {
                *c = Some(name.clone());
                stop2.store(true, Ordering::SeqCst);
                let _ = app2.emit(
                    "hotkey-captured",
                    serde_json::json!({"captured": name, "which": which2}),
                );
            }
        }
    });
}
