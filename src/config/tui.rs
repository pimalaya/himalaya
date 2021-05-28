use serde::Deserialize;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::tui::model::TuiAction;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    Key(Rc<RefCell<HashMap<Event, KeyType>>>),
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
    pub fn parse_keybindings(&self) -> Rc<RefCell<HashMap<Event, KeyType>>> {
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
        let keybindings: Rc<RefCell<HashMap<Event, KeyType>>> =
            Rc::new(RefCell::new(HashMap::new()));

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
            let mut node: Rc<RefCell<HashMap<Event, KeyType>>> =
                Rc::clone(&keybindings);

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
                    node.borrow_mut()
                        .insert(event, KeyType::Action(action_name.1.clone()));
                }
                // Suppose we have already stored the following keymapping:
                //
                //  gna
                //
                // Now we'd like to add the following keymapping:
                //
                //  gnn
                //
                // So we've to travel to node `n` first, in order to add the
                // second `n` to `gn`.
                // That's the usage of this else-if-clause: It will let the
                // `node` variable point to the first `n` so it'll look like
                // this:
                //
                // 1.
                //      g  <- node
                //       \
                //        n
                //         \
                //          a
                //
                // 2. (after this else-if-clause)
                //
                //      g
                //       \
                //        n <- node
                //         \
                //          a
                else if let Some(KeyType::Key(sub_node)) =
                    (*node).clone().borrow_mut().get(&event)
                    {
                        node = Rc::clone(&sub_node);
                    }
                // Suppose the user wants to have the following keymapping:
                //
                //  gnn
                //
                // But our keybinding tree looks only like that currently:
                //
                //      g
                //
                // We'd have to create the tree to g->n->n.
                // This else clause is creating the missing nodes to our
                // needed path.
                // So it'll do the following (assuming our tree is like
                // above):
                //
                //      g
                //       \
                //        n <- node
                //
                else {
                    let new_node = Rc::new(RefCell::new(HashMap::new()));
                
                    node.borrow_mut()
                        .insert(event, KeyType::Key(Rc::clone(&new_node)));
                
                    node = new_node;
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
