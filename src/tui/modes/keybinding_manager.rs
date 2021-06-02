use crate::config::tui::KeyType;
use crossterm::event::Event;

use std::collections::HashMap;

// ============
// Structs
// ============
pub struct KeybindingManager<ModeAction> {
    keybindings: HashMap<Event, KeyType<ModeAction>>,
    event_buffer: Vec<Event>,
}

impl<ModeAction: Clone> KeybindingManager<ModeAction> {
    pub fn new(keybindings: HashMap<Event, KeyType<ModeAction>>) -> Self {
        Self {
            keybindings,
            event_buffer: Vec::new(),
        }
    }

    // TODO: here
    pub fn eval_event(&mut self, event: Event) -> Option<ModeAction> {

        // get the starting node
        let node = if self.event_buffer.is_empty() {
            if let Some(node) = self.keybindings.get(&event) {
                node
            } else {
                return None;
            }
        } else {
            let mut node = self.keybindings.get(&self.event_buffer[0]).unwrap();

            for eve in &self.event_buffer[1..] {
                if let KeyType::Key(hashmap) = node { 
                    node = hashmap.get(&eve).unwrap();
                }
            }

            if let KeyType::Key(hashmap) = node {
                if let Some(node) = hashmap.get(&event) {
                    node
                }
                else {
                    self.event_buffer.clear();
                    return None;
                }
            }
            else {
                panic!("This shouldn't have happened...");
            }
        };

        // Look which kind of node we reached
        match node {
            KeyType::Action(action) => {
                self.event_buffer.clear();
                return Some((*action).clone());
            },
            KeyType::Key(_) => self.event_buffer.push(event),
        }

        None

    }
}
