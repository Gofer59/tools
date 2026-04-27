use anyhow::Result;

/// Read currently selected text.
/// Linux: tries PRIMARY selection first (selected but not copied), falls back to CLIPBOARD.
/// Windows: reads CLIPBOARD via arboard.
pub fn read_selection() -> Result<String> {
    #[cfg(target_os = "linux")]
    {
        return linux::read();
    }
    #[cfg(target_os = "windows")]
    {
        return windows::read();
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        anyhow::bail!("clipboard not supported on this platform");
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use anyhow::Result;
    use std::process::Command;

    enum Display {
        X11,
        Wayland,
    }

    fn detect() -> Display {
        if std::env::var("XDG_SESSION_TYPE")
            .map(|s| s.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
        {
            return Display::Wayland;
        }
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            return Display::Wayland;
        }
        Display::X11
    }

    fn read_one(disp: &Display, selection: &str) -> Result<String> {
        let out = match disp {
            Display::X11 => Command::new("xclip")
                .args(["-selection", selection, "-o"])
                .output()?,
            Display::Wayland => {
                let mut cmd = Command::new("wl-paste");
                if selection == "primary" {
                    cmd.arg("--primary");
                }
                cmd.arg("--no-newline").output()?
            }
        };
        if !out.status.success() {
            return Ok(String::new());
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    pub fn read() -> Result<String> {
        let disp = detect();
        let primary = read_one(&disp, "primary")?;
        if !primary.is_empty() {
            return Ok(primary);
        }
        read_one(&disp, "clipboard")
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use anyhow::Result;

    pub fn read() -> Result<String> {
        let mut cb = arboard::Clipboard::new()?;
        Ok(cb.get_text()?.trim().to_string())
    }
}
