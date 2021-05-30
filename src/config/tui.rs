use serde::Deserialize;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::tui::model::TuiAction;

use std::collections::HashMap;

// ==============
// Constants
// ==============

// Here are all special keybindings, which we can handle. Here are all
// special keys listed:
// https://docs.rs/crossterm/0.19.0/crossterm/event/enum.KeyCode.html
// const SPECIALS: [&str; 15] = [
//     "BS", "CR", "Left", "Right", "Up", "Down", "Home", "End", "PageUp",
//     "PageDown", "Tab", "BackTab", "Delete", "Insert", "Esc",
// ];

// ==========
// Enums
// ==========
#[derive(Debug, Deserialize, Clone)]
pub enum KeyType {
    Action(TuiAction),
    Key(HashMap<Event, KeyType>),
}

// ============
// Structs
// ============
#[derive(Debug, Deserialize)]
pub struct TuiConfig {
    pub sidebar: BlockDataConfig,
    pub mail_list: BlockDataConfig,

    /// Key = Action
    /// Value = Keybinding
    pub keybindings: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct BlockDataConfig {
    pub border_type: Option<String>,
    pub borders: Option<String>,
    pub border_color: Option<String>,
}

impl TuiConfig {
    /// This function will go through all keybindings in  TuiConfig.keybindings
    /// and converts them to a HashMap<Event, KeyType> for the TUI.
    ///
    /// In other words:
    /// It will return the final "form" to lookup after a keybinding.
    pub fn parse_keybindings(&self) -> HashMap<Event, KeyType> {
        // Here are all default keybindings stored in the following order:
        //
        //  default_actions = [
        //      (
        //          <action name from config file>,
        //          <action name for the tui>,
        //          <default keybinding>
        //      ),
        //      (
        //          ...
        //      ),
        //      ...
        //
        let default_actions = vec![
            ("quit", TuiAction::Quit, "q"),
            ("cursor_down", TuiAction::CursorDown, "j"),
            ("cursor_up", TuiAction::CursorUp, "k"),
        ];

        // This variable will store all keybindings which got converted into
        // <Event, Action>.
        let mut keybindings: HashMap<Event, KeyType> = HashMap::new();

        // Now iterate through all available actions and look, which one got
        // overridden.
        for action_name in default_actions {
            // Look, if the user set a keybinding to the given action or not.
            let keybinding = match self.keybindings.get(action_name.0) {
                // So the user provided his/her own keybinding => Parse it
                Some(keybinding) => keybinding,
                // Otherwise we're parsing the default keybinding
                None => action_name.2,
            };

            let mut iter = keybinding.chars();

            // This should rather fungate as a pointer which traverses through
            // the keybinding-tree in order to add other nodes or check, which
            // keybinding, was hit next.
            let mut node: &mut HashMap<Event, KeyType> = &mut keybindings;

            for key in iter.clone() {
                let event =
                    TuiConfig::convert_to_event(KeyModifiers::NONE, key);

                // If we reached the end of the keybinding-sequence like
                //
                //            g
                //             \
                //   gnn   =    n
                //     ^         \
                //     |          n <- node
                //    node
                //
                // Than we can apply the action to it.
                if iter.as_str().len() == 1 {
                    node.insert(event, KeyType::Action(action_name.1.clone()));
                } else {

                    // So this if condition looks, if there's already a node for
                    // the next key-hit. If not, create the node. For example if
                    // we have to store this keybinding:
                    //
                    //  gnn
                    // 
                    // but our tree looks only like that currently:
                    //
                    //      g
                    //
                    // Than this if clause would create the first 'n' node:
                    //
                    //      g
                    //       \
                    //        n
                    //
                    if let None = node.get(&event) {
                        node.insert(event, KeyType::Key(HashMap::new()));
                    }

                    // This if clause let us move to the next node. For example:
                    // 1. Before this if clause
                    //
                    //      g <- 'node' points here
                    //       \
                    //        n
                    //
                    // 2. After this if clause
                    //
                    //      g
                    //       \
                    //        n <- 'node' points here now
                    //
                    // We should never reach this panic-else block since we made
                    // sure with the previous if-clause that a node exists. But
                    // just in case, there's this panic.
                    node = if let Some(KeyType::Key(sub_node)) =
                        node.get_mut(&event)
                    {
                        sub_node
                    } else {
                        println!("Couldn't get to the next node of the");
                        println!("Keybinding tree.");
                        panic!("Incomplete Keybinding Tree.");
                    }

                }

                iter.next();
            }
        }

        keybindings
    }

    /// This function converts with the given
    /// [code](https://docs.rs/crossterm/0.19.0/crossterm/event/struct.KeyEvent.html#structfield.code)
    /// and
    /// [modifier](https://docs.rs/crossterm/0.19.0/crossterm/event/struct.KeyEvent.html#structfield.modifiers)
    /// its
    /// [KeyEvent](https://docs.rs/crossterm/0.19.0/crossterm/event/struct.KeyEvent.html)
    /// .
    ///
    /// It's just like an alias.
    ///
    /// # Example
    /// ```
    /// # use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    /// // this
    /// let key_event = TuiConfig::convert_to_event(KeyModifiers::NONE, 'c');
    ///
    /// // is the same as this
    /// let key_event2 = Event::Key(KeyEvent {
    ///     modifiers: KeyModifiers::NONE,
    ///     code: KeyCode::Char('c'),
    ///     });
    ///
    /// assert_eq!(key_event, key_event2);
    /// ```
    pub fn convert_to_event(modifiers: KeyModifiers, code: char) -> Event {
        Event::Key(KeyEvent {
            modifiers,
            code: KeyCode::Char(code),
        })
    }

    // // HINT: Not finished yet
    // pub fn get_event_from_keybinding(&self, keybinding: &str) -> Vec<Event> {
    //     let mut events = Vec::new();
    //     let mut iter = keybinding.chars();
    //
    //     let mut special_buffer = String::new();
    //     let mut is_special = false;
    //
    //     // Iterate through the given keybinding and parse it to its
    //     // corresponding event.
    //     //
    //     // Variables:
    //     //  c = character
    //     for c in iter.clone() {
    //         // Did we reach a "special" keybinding?
    //         // Special keybindings are
    //         // keys with a modifier like the Ctrl key and/or the keys in the
    //         // vector of `SPECIALS`.
    //         if c == '<' {
    //             // -----------------------------------
    //             // Collect the special-keybinding
    //             // -----------------------------------
    //             // Collect the special keybinding (if it's really a special
    //             // keybinding)
    //             is_special = true;
    //
    //             special_buffer.extend(iter.take_while(|character| {
    //                 if character.is_none() {
    //                     is_special = false;
    //                 }
    //
    //                 character
    //             }));
    //
    //             // -----------------------
    //             // Check for modifier
    //             // -----------------------
    //             if is_special && special_buffer.len() >= 3 && tmp_c == '>' {
    //                 // Yes it was! Now let's see which kind of
    //
    //                 // Look first, if it has a modifier:
    //             }
    //
    //             // now since we collected the next two chars, look if the
    //             // special buffer looks like this:
    //             //
    //             //  'C-', 'A-' or 'D-'
    //             //
    //             // Because this would mean, that we have a mapping like this:
    //             //
    //             //  <C-...>, <A-...> or <D-...>
    //             //
    //             special_buffer.clear();
    //         }
    //
    //         iter.next();
    //     }
    //
    //     events
    // }

    // pub fn handle_special_keybinding(test_str: &str) -> Result<Event, String> {
    //     let keywords = vec![
    //         "BS", "CR", "Left", "Right", "Up", "Down", "Home", "End", "PageUp",
    //         "PageDown", "Tab", "BackTab", "Delete", "Insert", "Esc",
    //     ];
    //
    //     if test_str.peek() == 'C'
    //         || test_str.peek() == 'A'
    //         || test_str.peek() == 'D'
    //     {}
    // }
}
