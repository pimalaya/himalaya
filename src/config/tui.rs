use serde::Deserialize;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::tui::model::TuiAction;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// ==========
// Enums
// ==========
#[derive(Debug, Deserialize, Clone)]
pub enum KeyType {
    Action(TuiAction),
    Key(Rc<RefCell<HashMap<Event, KeyType>>>),
}

// Errors
pub enum KeybindingError {
    NodeConflict,
    ConvertError,
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
            (
                "quit",
                TuiAction::Quit,
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('q'),
                }),
            ),
            (
                "cursor_down",
                TuiAction::CursorDown,
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('j'),
                }),
            ),
            (
                "cursor_up",
                TuiAction::CursorUp,
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('k'),
                }),
            ),
        ];

        // This variable will store all keybindings which got converted into
        // <Event, Action>.
        let keybindings: Rc<RefCell<HashMap<Event, KeyType>>> =
            Rc::new(RefCell::new(HashMap::new()));

        // Now iterate through all available actions and look, which one got
        // overridden.
        for action_name in default_actions {
            // Look, if the user set a keybinding to the given action or not.
            if let Some(keybinding) = self.keybindings.get(action_name.0) {
                let mut iter = keybinding.chars();

                // This should rather fungate as a pointer to a node of the
                // keybinding-tree.
                let mut node = Rc::clone(&keybindings);

                for key in iter.clone() {
                    let event = self.convert_to_event(KeyModifiers::NONE, key);

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
                        node.borrow_mut().insert(
                            event,
                            KeyType::Action(action_name.1.clone()),
                        );
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
                    //
                    // HINT: This text below might be a little bit wrong, but I
                    // it's reason is understandable.
                    //
                    // We are cloning "node" here, in order to "promise" the
                    // compiler, that this else-if-clause doesn't change and
                    // nothing bad can happen in the background. So we're using
                    // it's clone to get the Hashtable.
                    else if let Some(KeyType::Key(sub_node)) =
                        node.clone().borrow_mut().get(&event)
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
            // Use the default keybinding to the given action
            else {
            }
        }

        (*keybindings).clone().into_inner()
    }

    pub fn convert_to_event(
        &self,
        modifiers: KeyModifiers,
        code: char,
    ) -> Event {
        Event::Key(KeyEvent {
            modifiers,
            code: KeyCode::Char(code),
        })
    }

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
