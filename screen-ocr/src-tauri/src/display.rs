#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayServer {
    X11,
    Wayland,
    Windows,
}

pub fn detect() -> DisplayServer {
    #[cfg(target_os = "windows")]
    return DisplayServer::Windows;

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(session) = std::env::var("XDG_SESSION_TYPE") {
            if session.eq_ignore_ascii_case("wayland") {
                return DisplayServer::Wayland;
            }
        }
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            return DisplayServer::Wayland;
        }
        DisplayServer::X11
    }
}
