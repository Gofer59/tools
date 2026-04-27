use anyhow::Result;

pub fn read_primary() -> Result<String> {
    #[cfg(target_os = "linux")]
    return linux::read_sel("primary");
    #[cfg(target_os = "windows")]
    return Ok(String::new());
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    anyhow::bail!("not supported");
}

pub fn read_clipboard() -> Result<String> {
    #[cfg(target_os = "linux")]
    return linux::read_sel("clipboard");
    #[cfg(target_os = "windows")]
    {
        let mut cb = arboard::Clipboard::new()?;
        return Ok(cb.get_text().unwrap_or_default().trim().to_string());
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    anyhow::bail!("not supported");
}

#[cfg(target_os = "linux")]
mod linux {
    use anyhow::Result;
    use std::process::Command;

    enum Display { X11, Wayland }

    fn detect() -> Display {
        if std::env::var("XDG_SESSION_TYPE")
            .map(|s| s.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
            || std::env::var("WAYLAND_DISPLAY").is_ok()
        {
            return Display::Wayland;
        }
        Display::X11
    }

    pub fn read_sel(selection: &str) -> Result<String> {
        let disp = detect();
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
}
