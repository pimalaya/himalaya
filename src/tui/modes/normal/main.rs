use crossterm::event::Event;

use crate::config::model::Config;
use crate::config::tui::tui::TuiConfig;
use crate::tui::tabs::account_tab::AccountTab;

use crate::tui::model::BackendActions;
use crate::tui::modes::{
    backend_interface::BackendInterface, keybinding_manager::KeybindingManager,
};

use tui_rs::backend::Backend;
use tui_rs::layout::{Constraint, Direction, Layout, Rect};
use tui_rs::terminal::Frame;

// The widgets
use super::mail_list::MailList;
use super::sidebar::Sidebar;

// ==========
// Enums
// ==========
#[derive(Clone, Debug)]
pub enum NormalAction {
    Quit,
    CursorOffset(i32),
    CursorAbsolut(Option<usize>),
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
            keybinding_manager,
        }
    }

}

impl BackendInterface for NormalFrame {
    fn handle_event<'event>(
        &mut self,
        event: Event,
        account: &mut AccountTab<'event>,
    ) -> Option<BackendActions> {

        let mut account = &mut account.normal;

        if let Some(action) = self.keybinding_manager.eval_event(event) {
            match action {
                NormalAction::Quit => Some(BackendActions::Quit),
                NormalAction::CursorOffset(offset) => {
                    account.move_mail_list_cursor(offset);
                    Some(BackendActions::Redraw)
                },
                NormalAction::CursorAbsolut(index) => {
                    account.set_mail_list_cursor(index);
                    Some(BackendActions::Redraw)
                },
                NormalAction::ToggleSidebar => {
                    account.display_sidebar = !account.display_sidebar;
                    Some(BackendActions::Redraw)
                },
                NormalAction::WritingMail => Some(BackendActions::WritingMail),
                NormalAction::ViewingMail => Some(BackendActions::ViewingMail),
            }
        } else {
            None
        }
    }

    fn draw<'draw, B>(&mut self, frame: &mut Frame<B>, free_space: Rect, account: &mut AccountTab<'draw>)
    where
        B: Backend,
    {
        let account = &mut account.normal;
        
        if account.display_sidebar {
            // Create the two frames for the sidebar and the mails:
            //  - One on the left (sidebar)
            //  - One on the right (mail listing)
            let layout = Layout::default()
                .direction(Direction::Horizontal)
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
                .split(free_space);

            // Display the sidebar
            frame.render_stateful_widget(
                self.sidebar.widget(&account.mboxes),
                layout[0],
                account.sidebar_state.get_state(),
            );
            // Display the mails
            frame.render_stateful_widget(
                self.maillist.widget(&account.msgs),
                layout[1],
                account.mail_list_state.get_state(),
            );
        } else {
            let layout = Layout::default()
                .margin(1)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(frame.size());

            frame.render_stateful_widget(
                self.maillist.widget(&account.msgs),
                layout[0],
                account.mail_list_state.get_state(),
            );
        }
    }
}
