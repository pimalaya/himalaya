use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

struct Keybinding(KeyModifiers, char);

impl From<Keybinding> for Event {
    fn from(keybinding: Keybinding) -> Event {
        Event::Key(KeyEvent {
            modifiers: keybinding.0,
            code: KeyCode::Char(keybinding.1),
        })
    }
}

pub struct Keybindings {
    quit: Keybinding,
    moveDown: Keybinding,
    moveUp: Keybinding,
}

impl Keybindings {
    pub fn new() -> Keybindings {
        Keybindings {
            quit: Keybinding(KeyModifiers::NONE, 'q'),
            moveDown: Keybinding(KeyModifiers::NONE, 'j'),
            moveUp: Keybinding(KeyModifiers::NONE, 'k'),
        }
    }
}
