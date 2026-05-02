use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::display::DisplayServer;
use crate::region::Region;

pub fn capture(display: DisplayServer, region: &Region, output: &Path) -> Result<()> {
    let path_str = output.to_string_lossy().to_string();

    match display {
        DisplayServer::Windows => {
            Err(anyhow::anyhow!("screen capture not supported on Windows yet"))
        }
        DisplayServer::X11 => {
            let geom = format!("{}x{}+{}+{}", region.w, region.h, region.x, region.y);
            let status = Command::new("maim")
                .args(["-g", &geom, &path_str])
                .status()
                .context(
                    "Failed to run maim. Install with:\n  \
                     sudo apt install maim          # Debian/Ubuntu/Mint\n  \
                     sudo pacman -S maim            # Arch/SteamOS",
                )?;

            if !status.success() {
                anyhow::bail!("maim exited with non-zero status");
            }
            Ok(())
        }
        DisplayServer::Wayland => {
            let geom = format!("{},{} {}x{}", region.x, region.y, region.w, region.h);
            let status = Command::new("grim")
                .args(["-g", &geom, &path_str])
                .status()
                .context(
                    "Failed to run grim. Install with:\n  \
                     sudo apt install grim           # Debian/Ubuntu\n  \
                     sudo pacman -S grim             # Arch/SteamOS",
                )?;

            if !status.success() {
                anyhow::bail!("grim exited with non-zero status");
            }
            Ok(())
        }
    }
}
