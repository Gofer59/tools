use rdev::{listen, Event, EventType, Key};

fn main() {
    println!("Key detection utility — press any key (Ctrl+C to quit)");
    println!("Look for Key::Unknown(N) values to use in your rdev Config.\n");

    if let Err(e) = listen(|event: Event| {
        let (label, key) = match event.event_type {
            EventType::KeyPress(k) => ("KeyPress  ", k),
            EventType::KeyRelease(k) => ("KeyRelease", k),
            _ => return,
        };

        match key {
            Key::Unknown(code) => {
                println!("{label}  Key::Unknown({code})   ← use Key::Unknown({code}) in Config");
            }
            named => {
                println!("{label}  {named:?}");
            }
        }
    }) {
        eprintln!("rdev listen error: {e:?}");
    }
}
