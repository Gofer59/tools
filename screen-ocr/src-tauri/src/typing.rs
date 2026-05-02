use anyhow::{Context, Result};
use std::process::Command;

use crate::display::DisplayServer;

pub fn type_text(text: &str, display: DisplayServer) -> Result<()> {
    match display {
        DisplayServer::X11 => {
            let status = Command::new("xdotool")
                .args(["type", "--clearmodifiers", "--delay", "0", "--", text])
                .status()
                .context("Failed to run xdotool. Install with: sudo apt install xdotool")?;

            if !status.success() {
                anyhow::bail!("xdotool exited with non-zero status");
            }
            Ok(())
        }
        DisplayServer::Wayland => {
            let status = Command::new("ydotool")
                .args(["type", "--", text])
                .status()
                .context(
                    "Failed to run ydotool. Install with:\n  \
                     sudo pacman -S ydotool          # Arch/SteamOS\n  \
                     Note: ydotoold daemon must be running.",
                )?;

            if !status.success() {
                anyhow::bail!("ydotool exited with non-zero status");
            }
            Ok(())
        }
        DisplayServer::Windows => {
            #[cfg(target_os = "windows")]
            {
                use enigo::{Enigo, KeyboardControllable};
                let mut enigo = Enigo::new();
                enigo.key_sequence(text);
                Ok(())
            }
            #[cfg(not(target_os = "windows"))]
            Err(anyhow::anyhow!("Windows typing called on non-Windows platform"))
        }
    }
}
