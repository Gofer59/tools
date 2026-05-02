use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Child, Command};

pub fn spawn(text: &str, voice: &str, speed: f32, wrapper: &Path) -> Result<Child> {
    eprintln!("[screen-ocr] Speaking {} chars…", text.len());

    let speed_str = speed.to_string();

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: setsid() is async-signal-safe and has no preconditions.
        let child = unsafe {
            Command::new(wrapper)
                .arg(text)
                .arg(voice)
                .arg(&speed_str)
                .pre_exec(|| {
                    libc::setsid();
                    Ok(())
                })
                .spawn()
                .with_context(|| {
                    format!(
                        "Failed to run TTS wrapper at {:?}. Is voice-speak installed?",
                        wrapper
                    )
                })?
        };
        Ok(child)
    }

    #[cfg(not(target_os = "linux"))]
    Err(anyhow::anyhow!("TTS not available on this platform"))
}

pub fn kill(child: &mut Child) {
    eprintln!("[screen-ocr] Stopping TTS…");

    #[cfg(target_os = "linux")]
    {
        let pid = child.id() as i32;
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
        let _ = child.wait();
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}
