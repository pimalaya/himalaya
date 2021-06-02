// ===========
// Crates
// =========== */
use std::io;

use serde::Deserialize;

use crate::config::model::Config;

use tui_rs::backend::{Backend, CrosstermBackend};
use tui_rs::terminal::Frame;
use tui_rs::Terminal;

use crossterm::event::Event;
use crossterm::terminal;

use crate::tui::modes::normal::main::NormalFrame;

use crate::tui::modes::backend_interface::BackendInterface;

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
pub enum BackendActions {
    Quit,
    Redraw,
    GetAccount,
}

// ----------
// Modes
// ----------
pub enum TuiMode {
    Normal,
}

// ===================
// Structs/Traits
// =================== */
/// TODO: Docu
pub struct Tui<'tui> {
    config: &'tui Config,

    // modes
    normal_mode: NormalFrame,

    // State variables
    need_redraw: bool,
    run: bool,
    mode: TuiMode,
}

impl<'tui> Tui<'tui> {
    pub fn new(config: &'tui Config) -> Tui<'tui> {
        let normal_mode = NormalFrame::new(&config);

        Tui {
            normal_mode,
            config: config,
            need_redraw: true,
            run: true,
            mode: TuiMode::Normal,
        }
    }

    pub fn cleanup(&mut self) -> Result<(), TuiError> {
        // We don't need the raw mode anymore
        terminal::disable_raw_mode()?;

        Ok(())
    }

    pub fn handle_event(&mut self, event: Event) -> Result<(), TuiError> {
        // Look if it's intern
        match event {
            Event::Resize(_, _) => self.need_redraw = true,
            _ => match self.mode {
                TuiMode::Normal => match self.normal_mode.handle_event(event) {
                    Some(BackendActions::Quit) => self.run = false,
                    Some(BackendActions::Redraw) => self.need_redraw = true,
                    Some(BackendActions::GetAccount) => {
                        let account =
                            match self.config.find_account_by_name(None) {
                                Ok(account) => account,
                                Err(_) => return Err(TuiError::ConnectAccount),
                            };

                        self.normal_mode.set_account(&account)?;
                    }
                    None => (),
                },
            },
        }

        Ok(())
    }

    pub fn draw<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        // prepare the given frame
        match self.mode {
            TuiMode::Normal => self.normal_mode.draw(frame),
        };

        self.need_redraw = false;
    }

    pub fn run(mut self) -> Result<(), TuiError> {
        // ----------------
        // Preparation
        // ---------------- */
        // Prepare the terminal
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // cleanup the terminal first
        terminal.clear()?;

        // set the terminal into raw mode
        terminal::enable_raw_mode()?;

        // TEMPORARY:
        // get the default account
        let account = match self.config.find_account_by_name(None) {
            Ok(account) => account,
            Err(_) => return Err(TuiError::UnknownAccount),
        };
        self.normal_mode.set_account(account)?;

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
                Ok(event) => self.handle_event(event)?,
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
