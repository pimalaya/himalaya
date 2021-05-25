// ===========
// Crates
// =========== */
use std::io;
// use std::time::Duration;

use crate::config::model::{Account, Config};
use crate::imap::model::ImapConnector;

use tui_rs::backend::{Backend, CrosstermBackend};
use tui_rs::layout::{Constraint, Direction, Layout};
use tui_rs::terminal::Frame;
use tui_rs::Terminal;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;

use super::mail_list::MailList;
use super::sidebar::Sidebar;
use super::keybindings::Keybindings;

// =====================
// Tui return types
// ===================== */
pub enum TuiError {
    TerminalPreparation(io::Error),
    RawMode(crossterm::ErrorKind),
    DefaultAccount,
    EventKey,
    Draw,
    AddingAccount,
    ConnectAccount,
}

impl From<io::Error> for TuiError {
    fn from(error: io::Error) -> Self {
        Self::TerminalPreparation(error)
    }
}

impl From<crossterm::ErrorKind> for TuiError {
    fn from(error: crossterm::ErrorKind) -> Self {
        Self::RawMode(error)
    }
}

// ===================
// Structs/Traits
// =================== */
/// This struct is the backend of the Tui.
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
pub struct Tui<'tui> {
    sidebar: Sidebar,
    maillist: MailList,
    tui_accounts: Vec<ImapConnector<'tui>>,
    keybindings: Keybindings,

    // State variables
    need_redraw: bool,
    run: bool,
}

impl<'tui> Tui<'tui> {
    /// Creates a new Tui struct which already sets the appropriate constraints
    /// and places the frames correctly. It'll give the sidebar and the
    /// maillist a default value. The result can be seen
    /// [here](struct.Tui.html).
    /// TODO: Add tabs (accounts)
    pub fn new() -> Tui<'tui> {
        // Create the two desired main-frames
        let sidebar = Sidebar::new(String::from("Sidebar"));
        let maillist = MailList::new(String::from("Mails"));

        Tui {
            sidebar,
            maillist,
            keybindings: Keybindings::new(),
            tui_accounts: Vec::new(),
            need_redraw: true,
            run: true,
        }
    }

    pub fn add_account(
        &mut self,
        account_name: &str,
        config: &'tui Config,
    ) -> Result<(), TuiError> {
        let account = match config.find_account_by_name(Some(account_name)) {
            Ok(account) => account,
            Err(_) => return Err(TuiError::AddingAccount),
        };

        let imap_conn = match ImapConnector::new(&account) {
            Ok(connection) => connection,
            Err(_) => return Err(TuiError::ConnectAccount),
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

    pub fn cleanup(&mut self) -> Result<(), TuiError> {
        // logout all accounts
        for account in &mut self.tui_accounts {
            account.logout();
        }

        // cleanup the account list
        self.tui_accounts.clear();

        // We don't need the raw mode anymore
        terminal::disable_raw_mode()?;

        Ok(())
    }

    /// Use this function to draw the whole Tui with the sidebar, mails and
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
    ///     let tui = Tui::new(frame);
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

        // since we draw the Tui now, we don't need to draw the Tui again
        // immediately
        self.need_redraw = false;
    }

    pub fn eval_events(&mut self, event: Event) {
        let keybindings = self.keybindings;
        match event {
            keybindings.quit => self.run = false,
            keybindings.moveDown => self.maillist.move_selection(1),
            keybindings.moveUp => self.maillist.move_selection(-1),
            _ => (),
        }

        self.need_redraw = true;
    }

    pub fn run(&mut self, config: &'tui Config) -> Result<(), TuiError> {
        // ----------------
        // Preparation
        // ---------------- */
        // Prepare the terminal
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // load the default account
        if let Ok(Account {
            name: Some(name), ..
        }) = config.find_account_by_name(None)
        {
            self.add_account(&name, &config)?;
            self.set_account(0);
        } else {
            return Err(TuiError::DefaultAccount);
        }

        // cleanup the terminal first
        terminal.clear()?;

        // set the terminal into raw mode
        terminal::enable_raw_mode()?;

        // ------------------
        // Main Tui loop
        // ------------------ */
        while self.run {
            // Redraw if needed
            if self.need_redraw {
                if let Err(_) = terminal.draw(|frame| {
                    self.draw(frame);
                }) {
                    terminal.clear()?;
                    self.cleanup()?;
                    return Err(TuiError::Draw);
                };
            }

            // Catch any pressed keys. We're blocking here because nothing else
            // has to be down (no redraw or somehting like that)
            // HINT: If we need to do something in parallel, use add poll.
            match crossterm::event::read() {
                Ok(event) => self.eval_events(event),
                Err(_) => {
                    terminal.clear()?;
                    self.cleanup()?;
                    return Err(TuiError::EventKey);
                }
            };
        }
        terminal.clear()?;
        return self.cleanup();
    }
}
