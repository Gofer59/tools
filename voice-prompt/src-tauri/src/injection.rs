use anyhow::{Context, Result};
use enigo::{Direction, Enigo, Keyboard, Settings};

pub fn inject(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    let mut enigo = Enigo::new(&Settings::default()).context("enigo init")?;
    // Release common modifiers held during push-to-talk before typing
    for k in [
        enigo::Key::Control,
        enigo::Key::Alt,
        enigo::Key::Shift,
        enigo::Key::Meta,
    ] {
        let _ = enigo.key(k, Direction::Release);
    }
    enigo.text(text).context("enigo text")?;
    Ok(())
}
