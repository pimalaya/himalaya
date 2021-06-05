use crossterm::event::Event;

use crate::config::model::{Account, Config};
use crate::config::tui::tui::TuiConfig;
use crate::imap::model::ImapConnector;
use crate::tui::modes::state_wrappers::TableWrapperFuncs;

use crate::tui::model::{BackendActions, TuiError};
use crate::tui::modes::{
    backend_interface::BackendInterface, keybinding_manager::KeybindingManager,
};

use tui_rs::backend::Backend;
use tui_rs::layout::{Constraint, Direction, Layout};
use tui_rs::terminal::Frame;

// The widgets
use super::mail_list::MailList;
use super::sidebar::Sidebar;
use super::widgets::mail_entry::MailEntry;

// ==========
// Enums
// ==========
#[derive(Clone, Debug)]
pub enum NormalAction {
    Quit,
    CursorOffset(i32),
    CursorAbsolut(Option<usize>),
    SetAccount,
    WritingMail,
    ViewingMail,
    ToggleSidebar,
}

// ============
// Structs
// ============
pub struct NormalFrame {
    sidebar:  Sidebar,
    maillist: MailList,

    display_sidebar: bool,

    keybinding_manager: KeybindingManager<NormalAction>,
}

impl NormalFrame {
    pub fn new(config: &Config) -> Self {
        let sidebar =
            Sidebar::new(String::from("Sidebar"), &config.tui.normal.sidebar);

        let maillist =
            MailList::new(String::from("Mails"), &config.tui.normal.mail_list);

        // ----------------
        // Keybindings
        // ----------------
        let keybindings = TuiConfig::parse_keybindings(
            &config.tui.normal.default_keybindings,
            &config.tui.normal.keybindings,
        );

        let keybinding_manager = KeybindingManager::new(keybindings);

        Self {
            sidebar,
            maillist,
            display_sidebar: true,
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

    pub fn get_current_mail(&self) -> (String, String) {
        (
            self.maillist.get_current_mail().get_uid(),
            self.sidebar.get_current_mailbox(),
        )
    }
}

impl BackendInterface for NormalFrame {
    fn handle_event(&mut self, event: Event) -> Option<BackendActions> {
        if let Some(action) = self.keybinding_manager.eval_event(event) {
            match action {
                NormalAction::Quit => Some(BackendActions::Quit),
                NormalAction::CursorOffset(offset) => {
                    self.maillist.move_cursor(offset);
                    Some(BackendActions::Redraw)
                },
                NormalAction::CursorAbsolut(index) => {
                    self.maillist.set_cursor(index);
                    Some(BackendActions::Redraw)
                },
                NormalAction::SetAccount => Some(BackendActions::GetAccount),
                NormalAction::ToggleSidebar => {
                    self.display_sidebar = !self.display_sidebar;
                    Some(BackendActions::Redraw)
                },
                NormalAction::WritingMail => Some(BackendActions::WritingMail),
                NormalAction::ViewingMail => Some(BackendActions::ViewingMail),
            }
        } else {
            None
        }
    }

    fn draw<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        if self.display_sidebar {
            // Create the two frames for the sidebar and the mails:
            //  - One on the left (sidebar)
            //  - One on the right (mail listing)
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(
                    [
                        // For the sidebar (will be the second block => Index
                        // 0)
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
                self.sidebar.get_state(),
                // &mut self.sidebar.state,
            );
            // Display the mails
            frame.render_stateful_widget(
                self.maillist.widget(),
                layout[1],
                self.maillist.get_state(),
            );
        } else {
            let layout = Layout::default()
                .margin(1)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(frame.size());

            frame.render_stateful_widget(
                self.maillist.widget(),
                layout[0],
                self.maillist.get_state(),
            );
        }
    }
}
