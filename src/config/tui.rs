use serde::Deserialize;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::tui::model::TuiAction;

use std::collections::HashMap;

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
/// This struct stores the possible values which the user can set.
/// It's mainly a representation for the looking of each frame, like the sidebar
/// and the mail_list frame. So if you want to change the look of these frames,
/// than you're actually setting the values into this struct.
///
/// # Example
/// This is an example for the sidebar:
///
/// ```toml
/// [tui]
/// [tui.sidebar]
/// border_type = "Rounded"
/// borders = "ALL"
/// border_color = "Yellow"
/// ```
///
/// So after reading the config file, these values are stored here in this
/// struct into their appropriate attribute name.
#[derive(Debug, Deserialize)]
pub struct BlockDataConfig {
    
    /// So this variable stores the border type which the user wants to see. All
    /// possible options can be seen here:
    /// [here](https://docs.rs/tui/0.15.0/tui/widgets/enum.BorderType.html).
    pub border_type: Option<String>,

    /// Which borders of the square frame should be displayed? Default: `ALL`.
    /// For more information, take a look into their
    /// [docs](https://docs.rs/tui/0.15.0/tui/widgets/struct.Borders.html).
    pub borders: Option<String>,

    /// This stores the color of the border which can be one of [these
    /// variants](https://docs.rs/tui/0.15.0/tui/style/enum.Color.html#variants).
    pub border_color: Option<String>,
}

/// All config sections below the `[tui]` part in your
/// [config.toml](https://github.com/soywod/himalaya/wiki/Configuration:config-file)
/// file will
/// be stored in this struct.
/// 
/// # Example
/// ```toml
/// # other settings...
///
/// [tui]
/// # Everything which goes here, will be stored in the struct
/// ```
#[derive(Debug, Deserialize)]
pub struct TuiConfig {
    pub sidebar: BlockDataConfig,
    /// As explained in the [`BlockDataConfig` doc](struct.BlockDataConfig.html)
    /// each frame can be customized by this struct. For more information, take
    /// a short look into the doc of
    /// [`BlockDataConfig`](struct.BlockDataConfig.html).
    pub mail_list: BlockDataConfig,

    /// This attribute stores the loaded keybindings where:
    ///     Key = Action
    ///     Value = Keybinding
    ///
    /// # Example
    /// If this is in your
    /// [config.toml](https://github.com/soywod/himalaya/wiki/Configuration:config-file)
    /// file:
    ///
    /// ```toml
    /// [tui]
    /// [tui.keybindings]
    /// quit = "ddq"
    /// ```
    /// 
    /// Then this section will be stored in this variable as follows:
    ///
    /// ```no_run
    /// TuiConfig.keybindings = {
    ///     "quit" = "ddq",
    /// }
    pub keybindings: HashMap<String, String>,
}

/// The implementation of the struct are rather used for parsing the
/// keybindings.
impl TuiConfig {
    /// This function will go through all keybindings in  TuiConfig.keybindings
    /// and converts them to a HashMap<Event, KeyType> for the TUI.
    ///
    /// # Example Structure
    /// If this is in your config file:
    ///
    /// ```toml
    /// [tui]
    /// [tui.keybindings]
    /// quit = "d"
    /// ```
    ///
    /// Then this function will return the a
    /// [HashMap](https://doc.rust-lang.org/std/collections/hash_map/struct.HashMap.html)
    /// which will look as follows:
    ///
    /// ```no_run
    /// HashMap = {
    ///      Event::Key(KeyEvent{
    ///         code : 'd'
    ///         KeyModifiers: KeyModifiers::NONE,
    ///     })
    ///     :
    ///     KeyType::Action(Quit),
    /// }
    /// ```
    ///
    /// Click [here](https://docs.rs/crossterm/0.19.0/crossterm/event/enum.Event.html)
    /// to get more information about this `Event`.
    ///
    /// # Datastructure
    /// The HashMap which is gonna be returned by this function is not just a
    /// HashMap, it's rather a keybinding-tree-datastructure. Let's use this as
    /// an example for better understanding:
    ///
    /// ```text
    ///     User <- ptr
    ///     /  \
    ///    j    g
    ///          \
    ///           n
    ///          / \
    ///         a   n
    /// ```
    ///
    /// So `User` Represents the keyboard of the current user. `ptr` points to
    /// our current node, where we currently are. So at this moment we are in
    /// the root of the tree. If the user presses `g`, the tree would look as
    /// follows:
    ///
    /// ```text
    ///     User
    ///     /  \
    ///    j    g <- ptr
    ///          \
    ///           n
    ///          / \
    ///         a   n
    /// ```
    ///
    /// `ptr` moved one node down to `g`. Now if he/she presses `nn` than the
    /// tree would look like the following:
    ///
    /// ```text
    ///     User
    ///     /  \
    ///    j    g
    ///          \
    ///           n
    ///          / \
    ///         a   n <- ptr
    /// ```
    ///
    /// We reached a leaf of the tree. Each leaf has an Action, which can be
    /// executed by the TUI, for example when scrolling up.
    /// 
    /// # Minimal example
    /// So this example shows you, how to work with the output of the function.
    ///
    /// ```rust
    /// # use himalaya::config::tui::TuiConfig;
    /// # fn main() {
    /// // Consider that "TuiConfig" and "BlockData" don't have a "new()"
    /// // because the "toml" crate automatically fills in the data.
    /// let mut tui_config = TuiConfig {
    ///     sidebar: BlockDataConfig {
    ///         border_type: None,
    ///         borders: None,
    ///         border_color: None,
    ///     },
    ///     mail_list: BlockDataConfig {
    ///         border_type: None,
    ///         borders: None,
    ///         border_color: None,
    ///     },
    ///
    ///     // this is the interesting attribute
    ///     keybindings: HashMap::new(),
    /// };
    ///
    /// // Suppose our config.toml file looks as follows:
    /// // ```
    /// // [tui]
    /// // [tui.keybindings]
    /// // quit = "qq"
    /// // ```
    /// // This would be the same as adding this:
    /// tui_config.keybindings.insert("quit", "qq");
    ///
    /// // Now convert the keybindings into their appropriate events
    /// let mut keybindings = tui_config.parse_keybindings();
    /// // this will be our pointer for the tree
    /// let mut keybinding_ptr = &mut keybindings;
    ///
    /// // now catch some events
    /// match crossterm::event::read() {
    ///     Ok(event) =>
    ///         keybinding_ptr = match keybinding_ptr.get_mut(&event) {
    ///             // Well, there's no node with this event => go back to top
    ///             None => &mut keybindings,
    ///             // We reached a subnode => point to it
    ///             Some(tui::KeyType::Key(sub_node)) => sub_node,
    ///             // We reached a leaf! Do what you need with the action
    ///             // and go back to the top
    ///             Some(tui::KeyType::Action(action)) => {
    ///                 // do something
    ///                 // ...
    ///                 // point back to the top
    ///                 &mut keybindings
    ///             }
    ///         };
    ///     // error handling...
    /// }
    ///
    /// # }
    /// ```
    ///
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

        // This variable will store all keybindings which will get converted into
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

            // This should rather fungate as a pointer which traverses through
            // the keybinding-tree in order to add other nodes or check where to
            // go next.
            let mut node: &mut HashMap<Event, KeyType> = &mut keybindings;

            // Parse each keypress into the given event
            let iter = TuiConfig::parse_keys(keybinding);

            // We are iterating through all events, except the last one, because
            // the last key will bind the action.
            // This loop just makes sure that the "path" for the keybinding
            // exists. 
            // In other words, if we'd have this keybinding: 'gnn', than:
            //  1. Split it up to 'gn' and 'n'
            //  2. Create the path in the keybinding tree for "gn" (this loop):
            //          
            //      g
            //       \
            //        n
            //
            // 3. Add the last keybinding with the given action in the end after
            //    the loop:
            //
            //      g
            //       \
            //        n
            //         \
            //          n
            //
            for event in &iter[..iter.len() - 1] {
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
                // Then this if clause would create the first 'n' node:
                //
                //      g
                //       \
                //        n
                //
                if let None = node.get(&event) {
                    node.insert(*event, KeyType::Key(HashMap::new()));
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
                node =
                    if let Some(KeyType::Key(sub_node)) = node.get_mut(&event) {
                        sub_node
                    } else {
                        println!("Couldn't get to the next node of the");
                        println!("Keybinding tree.");
                        panic!("Incomplete Keybinding Tree.");
                    }
            }

            // So we created the path to our keybinding, now we just need to add
            // the last key with the given action.
            node.insert(
                *iter.last().unwrap(),
                KeyType::Action(action_name.1.clone()),
            );
        }

        keybindings
    }

    /// This function converts the given
    /// [code](https://docs.rs/crossterm/0.19.0/crossterm/event/struct.KeyEvent.html#structfield.code)
    /// and
    /// [modifier](https://docs.rs/crossterm/0.19.0/crossterm/event/struct.KeyEvent.html#structfield.modifiers)
    /// , to their corresponding
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
    #[inline(always)]
    pub fn convert_to_event(modifiers: KeyModifiers, code: KeyCode) -> Event {
        Event::Key(KeyEvent { modifiers, code })
    }

    /// This function parses a given keybinding into its corresponding events.
    /// So for example if the user has a keymapping like `gnn`, this function
    /// will convert each character into the event and returns them in a vector.
    ///
    /// # Example
    /// ```rust
    /// # use himalaya::config::tui::TuiConfig;
    /// # use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    /// # fn main() {
    /// let keybinding = String::from("<C-i>aj")
    /// let converted_keybinding = TuiConfig::parse_keys(&keybinding);
    ///
    /// assert_eq!(converted_keybinding,
    ///     vec![
    ///         Event::Key(KeyEvent { 
    ///             modifiers: KeyModifiers::CONTROL,
    ///             code: KeyCode::Char('i'),
    ///         }),
    ///         Event::Key(KeyEvent { 
    ///             modifiers: KeyModifiers::NONE,
    ///             code: KeyCode::Char('a'),
    ///         }),
    ///         Event::Key(KeyEvent { 
    ///             modifiers: KeyModifiers::NONE,
    ///             code: KeyCode::Char('j'),
    ///         }),
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn parse_keys(keybinding: &str) -> Vec<Event> {
        let mut events = Vec::new();
        let mut iter = keybinding.chars();
        // iter_c = iteration character
        let mut iter_c = iter.next();

        while iter_c.is_some() {
            // since we know that "character" has to be at least 'somehting', we
            // can unwrap it
            let character = iter_c.unwrap();

            if character == '<' {
                // Get the modifier key, if it exist like these:
                //  <C-l> or <S-h>
                let modifier = match &iter.as_str()[..2] {
                    "C-" => KeyModifiers::CONTROL,
                    "A-" => KeyModifiers::ALT,
                    "S-" => KeyModifiers::SHIFT,
                    _ => KeyModifiers::NONE,
                };

                let code = {
                    // "unparsed" is a representatin of the rest of the
                    // keybinding which has not been parsed yet. It's used for
                    // the other functions below in order to find a "pattern" in
                    // the rest of the keybinding.
                    let unparsed = &iter.as_str();

                    // Look, if the keybinding is just like:
                    //  <Home> or <Up>
                    if modifier == KeyModifiers::NONE {
                        TuiConfig::get_special_key(&unparsed[1..])
                    }
                    // Look if the keybinding looks like this:
                    //  <C-l> or <S-h>
                    else if iter.as_str().chars().nth(3) == Some('>') {
                        KeyCode::Char(unparsed.chars().nth(2).unwrap())
                    }
                    // Otherwise it may be a combination of a modifier + special
                    // key like these:
                    //  <C-Home> or <C-Left>
                    else {
                        TuiConfig::get_special_key(&unparsed[3..])
                    }
                };

                // Now let's make sure, that we REALLY found a special
                // keybinding, by looking, if "code" is Null. If it's "Null"
                // than the keymapping would look like this:
                //  <C---- or <Cowefl
                // So we have been "tricked" and we just need to add the key...
                if code == KeyCode::Null {
                    // events.push(TuiConfig::convert_to_event(
                    //     KeyModifiers::NONE,
                    //     KeyCode::Char(character),
                    // ));
                    events.push(TuiConfig::convert_to_event(
                        KeyModifiers::NONE,
                        KeyCode::Char(character),
                    ));
                }
                // otherwise we just need to add our special keybinding and
                // bring the iterator to the position after the closing ">".
                else {
                    // events.push(TuiConfig::convert_to_event(modifier, code));
                    events.push(TuiConfig::convert_to_event(modifier, code));

                    while iter.next() != Some('>') {}
                }
            }
            // So in this case, the current character is just a normal character
            // like:
            //  "gnn" or "asg"
            // So we just need to add the character code to it
            else {
                events.push(TuiConfig::convert_to_event(
                    KeyModifiers::NONE,
                    KeyCode::Char(character),
                ));
            }

            iter_c = iter.next();
        }

        events
    }

    /// This function just looks, if the given `key` matches one of the [special
    /// keycodes](https://docs.rs/crossterm/0.19.0/crossterm/event/enum.KeyCode.html)
    /// including the end '>' tag. Take a short look into its source code and
    /// you'll understand, what I mean :)
    pub fn get_special_key(key: &str) -> KeyCode {
        match key {
            "BS>" => KeyCode::Backspace,
            "CR>" => KeyCode::Enter,
            "Left>" => KeyCode::Left,
            "Right>" => KeyCode::Right,
            "Up>" => KeyCode::Up,
            "Down>" => KeyCode::Down,
            "Home>" => KeyCode::Home,
            "End>" => KeyCode::End,
            "PageUp>" => KeyCode::PageUp,
            "PageDown>" => KeyCode::PageDown,
            "Tab>" => KeyCode::Tab,
            "BackTab>" => KeyCode::BackTab,
            "Delete>" => KeyCode::Delete,
            "Insert>" => KeyCode::Insert,
            "Esc>" => KeyCode::Esc,
            _ => KeyCode::Null,
        }
    }
}
