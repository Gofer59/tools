use tauri::{AppHandle, Manager};
use tempfile::NamedTempFile;

use crate::AppState;
use crate::capture;
use crate::clipboard;
use crate::ocr;
use crate::region;
use crate::tts;
use crate::typing;

pub enum Mode {
    Quick,
    Select,
}

pub async fn run(app: AppHandle, mode: Mode) {
    let state = match app.try_state::<AppState>() {
        Some(s) => s,
        None => {
            eprintln!("[screen-ocr] pipeline: no AppState");
            return;
        }
    };

    let (display, ocr_language, delivery_mode, tts_voice, tts_speed) = {
        let cfg = state.config.read().await;
        (
            state.display,
            cfg.ocr_language.clone(),
            cfg.delivery_mode.clone(),
            cfg.tts_voice.clone(),
            cfg.tts_speed,
        )
    };

    // Determine region: for Quick, use last_region or fall back to interactive;
    // for Select, always do interactive.
    let region_result = match mode {
        Mode::Quick => {
            let last = *state.last_region.lock().await;
            match last {
                Some(r) => {
                    eprintln!(
                        "[screen-ocr] Quick capture: {}x{}+{}+{}",
                        r.w, r.h, r.x, r.y
                    );
                    Ok(r)
                }
                None => {
                    eprintln!("[screen-ocr] No saved region — select one now…");
                    let disp = display;
                    tauri::async_runtime::spawn_blocking(move || region::select(disp))
                        .await
                        .unwrap_or_else(|e| Err(anyhow::anyhow!("join error: {e}")))
                }
            }
        }
        Mode::Select => {
            eprintln!("[screen-ocr] Select a screen region…");
            let disp = display;
            tauri::async_runtime::spawn_blocking(move || region::select(disp))
                .await
                .unwrap_or_else(|e| Err(anyhow::anyhow!("join error: {e}")))
        }
    };

    let selected_region = match region_result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[screen-ocr] Region selection error: {e}");
            return;
        }
    };

    // Save the region for future Quick captures.
    *state.last_region.lock().await = Some(selected_region);
    eprintln!(
        "[screen-ocr] Region: {}x{}+{}+{}",
        selected_region.w, selected_region.h, selected_region.x, selected_region.y
    );

    // Capture to temp PNG.
    let tmp = match NamedTempFile::with_suffix(".png") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[screen-ocr] Could not create temp file: {e}");
            return;
        }
    };

    if let Err(e) = capture::capture(display, &selected_region, tmp.path()) {
        eprintln!("[screen-ocr] Capture error: {e}");
        return;
    }

    // OCR.
    let text = match ocr::extract(&app, tmp.path(), &ocr_language) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[screen-ocr] OCR error: {e}");
            return;
        }
    };
    // tmp drops here → temp file deleted.

    if text.is_empty() {
        eprintln!("[screen-ocr] No text extracted.");
        return;
    }

    eprintln!("[screen-ocr] Extracted {} chars.", text.len());

    // Deliver.
    match delivery_mode.as_str() {
        "clipboard" => {
            if let Err(e) = clipboard::copy(&text) {
                eprintln!("[screen-ocr] Clipboard error: {e}");
            } else {
                eprintln!("[screen-ocr] Copied to clipboard.");
            }
        }
        "type" => {
            if let Err(e) = typing::type_text(&text, display) {
                eprintln!("[screen-ocr] Type error: {e}");
            } else {
                eprintln!("[screen-ocr] Typed at cursor.");
            }
        }
        "both" => {
            if let Err(e) = clipboard::copy(&text) {
                eprintln!("[screen-ocr] Clipboard error: {e}");
            }
            if let Err(e) = typing::type_text(&text, display) {
                eprintln!("[screen-ocr] Type error: {e}");
            }
        }
        other => {
            eprintln!("[screen-ocr] Unknown delivery_mode: {other}, defaulting to clipboard");
            if let Err(e) = clipboard::copy(&text) {
                eprintln!("[screen-ocr] Clipboard error: {e}");
            }
        }
    }

    // TTS (non-blocking).
    let wrapper = crate::paths::wrapper_script(&app);
    // Use tts_speak_wrapper.sh from voice-speak install, falling back to home dir.
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let tts_wrapper = home.join(".local").join("bin").join("tts_speak_wrapper.sh");

    let _ = wrapper; // wrapper_script gives OCR script; TTS uses tts_speak_wrapper
    {
        let mut tts_guard = state.tts_child.lock().await;
        // Kill previous TTS if still running.
        if let Some(ref mut child) = *tts_guard {
            match child.try_wait() {
                Ok(Some(_)) => {}
                _ => tts::kill(child),
            }
            *tts_guard = None;
        }
        match tts::spawn(&text, &tts_voice, tts_speed, &tts_wrapper) {
            Ok(child) => *tts_guard = Some(child),
            Err(e) => eprintln!("[screen-ocr] TTS error (continuing without speech): {e}"),
        }
    }

    eprintln!("[screen-ocr] Ready.");
}
