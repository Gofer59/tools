use anyhow::Result;
use arboard::Clipboard;

pub fn copy(text: &str) -> Result<()> {
    let mut cb = Clipboard::new()?;
    cb.set_text(text.to_owned())?;
    Ok(())
}
