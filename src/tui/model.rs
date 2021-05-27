// ===========
// Crates
// =========== */
use std::collections::HashMap;
use std::io;
// use std::time::Duration;

use serde::Deserialize;

use crate::config::model::Config;
use crate::imap::model::ImapConnector;
use crate::config::tui;

use tui_rs::backend::{Backend, CrosstermBackend};
use tui_rs::layout::{Constraint, Direction, Layout};
use tui_rs::terminal::Frame;
use tui_rs::Terminal;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;

use super::mail_list::MailList;
use super::sidebar::Sidebar;
// use super::keybindings::Keybindings;

// ================
// Tui - Enums
// ================
// -----------
// Errors
// -----------
pub enum TuiError {
    ConnectAccount,
    Draw,
    EventKey,
    MailList,
    RawMode(crossterm::ErrorKind),
    Sidebar,
    TerminalPreparation(io::Error),
    UnknownAccount,
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

// ------------
// Actions
// ------------
#[derive(Debug, Deserialize, Clone)]
pub enum TuiAction {
    Quit,
    CursorDown,
    CursorUp,
}

// ===================
// Structs/Traits
// =================== */
/// This struct is the backend of the Tui.
///
/// --- Tab 1 ---
/// |           |
/// -  Sidebar  -- Mail Listing -------------------
/// |           |                                 |
/// |           |                                 |
/// |           |                                 |
/// |           |                                 |
/// |           |                                 |
/// |           |                                 |
/// -----------------------------------------------
///
pub struct Tui<'tui> {
    sidebar: Sidebar,
    maillist: MailList,
    connections: Vec<ImapConnector<'tui>>,
    config: &'tui Config,
    keybindings: HashMap<Event, tui::KeyType>,

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
    /// HINT: Think about adding all accounts immediately or storing the configs
    /// in the struct => Take ownership
    pub fn new(config: &'tui Config) -> Tui<'tui> {

        // -----------------
        // TUI - Frames
        // -----------------
        let sidebar =
            Sidebar::new(String::from("Sidebar"), &config.tui.sidebar);
        let maillist =
            MailList::new(String::from("Mails"), &config.tui.mail_list);

        Tui {
            sidebar,
            maillist,
            connections: Vec::new(),
            config: config,
            keybindings: HashMap::new(),
            need_redraw: true,
            run: true,
        }
    }

    pub fn set_account(&mut self, name: Option<&str>) -> Result<(), TuiError> {
        // ----------------
        // Get account
        // ----------------
        // Get the account first according to the name
        let account = match self.config.find_account_by_name(name) {
            Ok(account) => account,
            Err(_) => return Err(TuiError::UnknownAccount),
        };

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

    pub fn cleanup(&mut self) -> Result<(), TuiError> {
        // logout all accounts
        for account in &mut self.connections {
            account.logout();
        }

        // We don't need the raw mode anymore
        terminal::disable_raw_mode()?;

        Ok(())
    }

    /// Use this function to draw the whole Tui with the sidebar, mails and
    /// accounts.
    ///
    /// # Example:
    /// ```no_run
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

    // pub fn do_action(&mut self, action: TuiAction) {
    //     match action {
    //         TuiAction::Quit => self.run = false,
    //         TuiAction::CursorDown => self.maillist.move_selection(1),
    //         TuiAction::CursorUp => self.mailllist.move_selection(-1),
    //         _ => (),
    //     };
    //
    //     self.need_redraw = true;
    // }

    pub fn eval_events(&mut self, event: Event) {
        match event {
            Event::Key(KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Char('q'),
            }) => self.run = false,
            Event::Key(KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Char('j'),
            }) => self.maillist.move_selection(1),
            Event::Key(KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Char('k'),
            }) => self.maillist.move_selection(-1),
            _ => (),
        };

        self.need_redraw = true;
    }

    pub fn run(&mut self) -> Result<(), TuiError> {
        // ----------------
        // Preparation
        // ---------------- */
        // Prepare the terminal
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // load the default account
        if let Err(err) = self.set_account(None) {
            return Err(err);
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
