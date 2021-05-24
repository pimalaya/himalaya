use std::io;

use crate::config::model::Config;
use crate::imap::model::ImapConnector;

use tui_rs::backend::{Backend, CrosstermBackend};
use tui_rs::layout::{Constraint, Direction, Layout};
use tui_rs::terminal::Frame;
use tui_rs::Terminal;

use super::mail_frame::MailFrame;
use super::mail_list::MailList;
use super::sidebar::Sidebar;

// ===================
// Structs/Traits
// =================== */
/// This struct is the backend of the TUI.
///
///     --- Tab 1 ---
///     |           |
///     -  Sidebar  -- Mail Listing -------------------
///     |           |                                 |
///     |           |                                 |
///     |           |                                 |
///     |           |                                 |
///     |           |                                 |
///     |           |                                 |
///     -----------------------------------------------
///
pub struct TUI<'tui> {
    sidebar: Sidebar,
    maillist: MailList<'tui>,
    tui_accounts: Vec<ImapConnector<'tui>>,
}

impl<'tui> TUI<'tui> {
    /// Creates a new TUI struct which already sets the appropriate constraints
    /// and places the frames correctly. It'll give the sidebar and the
    /// maillist a default value. The result can be seen
    /// [here](struct.TUI.html).
    /// TODO: Add tabs (accounts)
    pub fn new() -> TUI<'tui> {
        // Create the two desired main-frames
        let sidebar = Sidebar::new(String::from("Sidebar"));
        let maillist = MailList::new(String::from("Mails"));

        TUI {
            sidebar,
            maillist,
            tui_accounts: Vec::new(),
        }
    }

    pub fn add_account(&mut self, account_name: &str, config: &'tui Config) -> Result<(), i32> {
        let account = match config.find_account_by_name(Some(account_name)) {
            Ok(account) => account,
            Err(_) => return Err(-1),
        };

        let imap_conn = match ImapConnector::new(&account) {
            Ok(connection) => connection,
            Err(_) => return Err(-2),
        };

        self.tui_accounts.push(imap_conn);

        Ok(())
    }

    pub fn set_account(&mut self, index: usize) {
        if index < self.tui_accounts.len() {
            // Set the mailboxes
            let mut imap_conn = &mut self.tui_accounts[index];
            if let Err(err) = self.sidebar.set_mailboxes(&mut imap_conn) {
                println!("{}", err);
            };

            // set the mails
            if let Err(err) = self
                .maillist
                .set_mails(&mut imap_conn, &self.sidebar.mailboxes()[0][0])
            {
                println!("{}", err);
            };
        }
    }

    pub fn cleanup(&mut self) {
        // logout all accounts
        for account in &mut self.tui_accounts {
            account.logout();
        }

        // cleanup the account list
        self.tui_accounts.clear();
    }

    /// Use this function to draw the whole TUI with the sidebar, mails and
    /// accounts.
    ///
    /// # Example:
    /// ```rust
    /// let stdout = io::stdout();
    /// let backend = CrosstermBackend::new(stdout);
    /// let mut terminal = Terminal::new(backend)?;
    ///
    /// // clear the terminal
    /// terminal.clear()?;
    ///
    /// // Draw the user interface
    /// terminal.draw(|frame| {
    ///     let tui = TUI::new(frame);
    ///     tui.draw(frame);
    /// })?;
    /// ```
    pub fn draw<B>(&mut self, frame: &mut Frame<B>)
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

        frame.render_widget(self.sidebar.widget(), layout[0]);
        frame.render_widget(self.maillist.widget(), layout[1]);
    }
}

// ==============
// Functions
// ============== */
/// Start the tui by preparing
/// Return:
///     -1 => Preparation gone wrong
///     -2 => Couldn't create TUI
pub fn run_tui(config: &Config) -> Result<(), String> {
    println!("Starting tui");

    // Prepare the terminal and the account connection as well
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(err) => return Err(err.to_string()),
    };

    // clear the terminal
    if let Err(_) = terminal.clear() {
        return Err(String::from(
            "An error appeared, when trying to clear the terminal.",
        ));
    };

    // get the default account
    let default_account = match config.find_account_by_name(None) {
        Ok(account) => account,
        Err(_) => {
            return Err(String::from("Couldn't get default account"));
        }
    };

    let default_account = match &default_account.name {
        Some(name) => name,
        None => return Err(String::from("Couldn't find default account")),
    };

    let mut tui = TUI::new();

    // select the default account first
    if let Err(code) = tui.add_account(&default_account, &config) {
        if code == -1 {
            println!("Bruh");
        } else if code == -2 {
            println!(" LOL ");
        }

        return Err(String::from("Couldn't load the default account."));
    }
    tui.set_account(0);

    // Draw the user interface
    if let Err(_) = terminal.draw(|frame| {
        tui.draw(frame);
        tui.cleanup();
    }) {
        tui.cleanup();
        return Err(String::from("Couldn't draw the TUI"));
    };

    Ok(())
}
