use super::mail_list::MailList;
use super::sidebar::Sidebar;

use crossterm::event::Event;

use crate::config::model::{Account, Config};
use crate::config::tui::TuiConfig;
use crate::imap::model::ImapConnector;

use crate::tui::model::{BackendActions, TuiError};
use crate::tui::modes::{
    backend_interface::BackendInterface, keybinding_manager::KeybindingManager,
};

use std::collections::HashMap;

use tui_rs::backend::Backend;
use tui_rs::layout::{Constraint, Direction, Layout};
use tui_rs::terminal::Frame;

// ==========
// Enums
// ==========
#[derive(Clone)]
pub enum NormalActions {
    Quit,
    CursorDown,
    CursorUp,
    SetAccount,
}

// ============
// Structs
// ============
pub struct NormalFrame {
    sidebar: Sidebar,
    maillist: MailList,

    keybinding_manager: KeybindingManager<NormalActions>,
}

impl NormalFrame {
    pub fn new(config: &Config) -> Self {
        let sidebar =
            Sidebar::new(String::from("Sidebar"), &config.tui.sidebar);

        let maillist =
            MailList::new(String::from("Mails"), &config.tui.mail_list);

        // ----------------
        // Keybindings
        // ----------------
        let default_keybindings = vec![
            ("quit", NormalActions::Quit, "q"),
            ("cursor_down", NormalActions::CursorDown, "j"),
            ("cursor_up", NormalActions::CursorUp, "k"),
        ];

        let keybindings = if let Some(user_keybindings) =
            config.tui.keybindings.get("normal")
        {
            TuiConfig::parse_keybindings(
                &default_keybindings,
                &user_keybindings,
            )
        } else {
            TuiConfig::parse_keybindings(&default_keybindings, &HashMap::new())
        };

        let keybinding_manager = KeybindingManager::new(keybindings);

        Self {
            sidebar,
            maillist,
            keybinding_manager,
        }
    }

    pub fn set_account(&mut self, account: &Account) -> Result<(), TuiError> {
        // ----------------
        // Get account
        // ----------------
        // Get the account first according to the name
        // let account = match &config.find_account_by_name(name) {
        //     Ok(account) => account,
        //     Err(_) => return Err(TuiError::UnknownAccount),
        // };

        // ----------------------
        // Create connection
        // ----------------------
        let mut imap_conn = match ImapConnector::new(&account) {
            Ok(connection) => connection,
            Err(_) => return Err(TuiError::ConnectAccount),
        };

        // ----------------
        // Refresh TUI
        // ----------------
        // Fill the frames with the information of the mail account
        if let Err(_) = self.sidebar.set_mailboxes(&mut imap_conn) {
            imap_conn.logout();
            return Err(TuiError::Sidebar);
        }

        if let Err(_) = self
            .maillist
            .set_mails(&mut imap_conn, &self.sidebar.mailboxes()[0][0])
        {
            imap_conn.logout();
            return Err(TuiError::MailList);
        }

        // logout
        imap_conn.logout();
        Ok(())
    }
}

impl BackendInterface for NormalFrame {
    fn handle_event(&mut self, event: Event) -> Option<BackendActions> {
        if let Some(action) = self.keybinding_manager.eval_event(event) {
            match action {
                NormalActions::Quit => Some(BackendActions::Quit),
                NormalActions::CursorUp => {
                    self.maillist.move_selection(-1);
                    Some(BackendActions::Redraw)
                }
                NormalActions::SetAccount => Some(BackendActions::GetAccount),
                NormalActions::CursorDown => {
                    self.maillist.move_selection(1);
                    Some(BackendActions::Redraw)
                }
            }
        } else {
            None
        }
    }

    fn draw<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        // Create the two frames for the sidebar and the mails:
        //  - One on the left (sidebar)
        //  - One on the right (mail listing)
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(
                [
                    // For the sidebar (will be the second block => Index 0)
                    Constraint::Percentage(25),
                    // For the mails (will be the second block => Index 1)
                    Constraint::Percentage(75),
                ]
                .as_ref(),
            )
            // Use the given frame size to create the two blocks
            .split(frame.size());

        // Display the sidebar
        frame.render_stateful_widget(
            self.sidebar.widget(),
            layout[0],
            &mut self.sidebar.state,
        );

        // Display the mails
        frame.render_stateful_widget(
            self.maillist.widget(),
            layout[1],
            &mut self.maillist.state,
        );
    }
}
