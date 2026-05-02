use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

use crate::display::DisplayServer;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[allow(dead_code)]
pub fn load(path: &Path) -> Result<Region> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("No saved region at {:?}", path))?;
    let region: Region = serde_json::from_str(&contents)
        .context("Failed to parse saved region JSON")?;
    Ok(region)
}

#[allow(dead_code)]
pub fn save(path: &Path, r: &Region) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create directory {:?}", parent))?;
    }
    std::fs::write(path, serde_json::to_string_pretty(r)?)?;
    Ok(())
}

pub fn select(display: DisplayServer) -> Result<Region> {
    match display {
        DisplayServer::Windows => {
            Err(anyhow::anyhow!("region selection not supported on Windows yet"))
        }
        DisplayServer::X11 => {
            // slop --format outputs space-separated: "W H X Y"
            let output = Command::new("slop")
                .args(["--format=%w %h %x %y"])
                .output()
                .context(
                    "Failed to run slop. Install with:\n  \
                     sudo apt install slop          # Debian/Ubuntu/Mint\n  \
                     sudo pacman -S slop            # Arch/SteamOS",
                )?;

            if !output.status.success() {
                anyhow::bail!("slop exited with non-zero status (user cancelled selection?)");
            }

            let text = String::from_utf8(output.stdout)
                .context("slop output was not valid UTF-8")?
                .trim()
                .to_owned();

            // Parse "W H X Y"
            let parts: Vec<i32> = text
                .split_whitespace()
                .map(|s| s.parse::<i32>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse slop geometry: {:?}", text))?;

            if parts.len() != 4 {
                anyhow::bail!("Unexpected slop output (expected 4 values): {:?}", text);
            }

            Ok(Region {
                w: parts[0] as u32,
                h: parts[1] as u32,
                x: parts[2],
                y: parts[3],
            })
        }
        DisplayServer::Wayland => {
            let output = Command::new("slurp")
                .output()
                .context(
                    "Failed to run slurp. Install with:\n  \
                     sudo apt install slurp          # Debian/Ubuntu\n  \
                     sudo pacman -S slurp            # Arch/SteamOS",
                )?;

            if !output.status.success() {
                anyhow::bail!("slurp exited with non-zero status (user cancelled selection?)");
            }

            let text = String::from_utf8(output.stdout)
                .context("slurp output was not valid UTF-8")?
                .trim()
                .to_owned();

            // Parse "X,Y WxH"
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() != 2 {
                anyhow::bail!("Unexpected slurp geometry: {:?}", text);
            }

            let xy: Vec<i32> = parts[0]
                .split(',')
                .map(|v| v.parse::<i32>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse slurp X,Y: {:?}", parts[0]))?;

            let wh: Vec<u32> = parts[1]
                .split('x')
                .map(|v| v.parse::<u32>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse slurp WxH: {:?}", parts[1]))?;

            if xy.len() != 2 || wh.len() != 2 {
                anyhow::bail!("Unexpected slurp geometry format: {:?}", text);
            }

            Ok(Region {
                x: xy[0],
                y: xy[1],
                w: wh[0],
                h: wh[1],
            })
        }
    }
}
