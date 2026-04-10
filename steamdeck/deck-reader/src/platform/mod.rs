// Platform abstraction layer.
//
// Exposes a cross-platform public API by re-exporting one of two inner
// implementations (linux or windows) selected at compile time via cfg.
//
// Consumers in main.rs should never reach into the inner module directly —
// always call functions through the `platform::` prefix.

#[cfg(unix)]
#[path = "linux.rs"]
mod inner;

#[cfg(windows)]
#[path = "windows.rs"]
mod inner;

pub use inner::*;

use serde::{Deserialize, Serialize};

/// A screen rectangle in physical (device) pixels.
/// Origin is the top-left of the virtual-screen bounding box.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Which selection buffer to read from.
/// On Linux this maps to X11 PRIMARY vs CLIPBOARD (or Wayland primary vs clipboard).
/// On Windows there is only one system clipboard, so both variants behave the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Primary,
    Clipboard,
}

/// Underlying stream type for the TTS daemon IPC endpoint.
/// Unix socket on Linux, TCP stream on Windows (phase 2).
/// Currently unused outside main.rs's TtsState; exposed for subtask 1.6.
#[allow(dead_code)]
#[cfg(unix)]
pub type DaemonStream = std::os::unix::net::UnixStream;
#[allow(dead_code)]
#[cfg(windows)]
pub type DaemonStream = std::net::TcpStream;

/// True on platforms where the persistent TTS daemon is implemented.
/// Windows MVP uses fallback-only, so this is false there.
/// Read by subtask 1.6 once the daemon code path is cfg-gated.
#[allow(dead_code)]
#[cfg(unix)]
pub const DAEMON_SUPPORTED: bool = true;
#[allow(dead_code)]
#[cfg(windows)]
pub const DAEMON_SUPPORTED: bool = false;
