// ===========
// Crates
// =========== */
use std::io;

use serde::Deserialize;

use tui_rs::backend::{Backend, CrosstermBackend};
use tui_rs::terminal::Frame;
use tui_rs::Terminal;

use crossterm::event::Event;
use crossterm::terminal;

use crate::tui::modes::{
    backend_interface::BackendInterface, normal::main::NormalFrame,
    viewer::main::Viewer, writer::main::Writer,
};

use crate::config::model::Config;

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
    WritingMail,
    ViewingMail,
}

// ----------
// Modes
// ----------
pub enum TuiMode {
    Normal,
    Writing,
    Viewing,
}

// ===================
// Structs/Traits
// =================== */
pub struct Tui<'tui> {
    config: &'tui Config,

    // modes
    normal: NormalFrame,
    writer: Writer,
    viewer: Viewer,

    // State variables
    need_redraw: bool,
    run:         bool,
    mode:        TuiMode,
}

impl<'tui> Tui<'tui> {
    pub fn new(config: &'tui Config) -> Tui<'tui> {
        let normal = NormalFrame::new(&config);
        let writer = Writer::new(&config);
        let viewer = Viewer::new(&config);

        Tui {
            normal,
            viewer,
            writer,
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
        // We treat resizeing as a special treat
        if let Event::Resize(_, _) = event {
            self.need_redraw = true;
            return Ok(());
        }

        match self.mode {
            // Normal - Mode
            TuiMode::Normal => {
                if let Some(action) = self.normal.handle_event(event) {
                    match action {
                        BackendActions::Quit => self.run = false,
                        BackendActions::Redraw => self.need_redraw = true,
                        BackendActions::GetAccount => {
                            let account = match self
                                .config
                                .find_account_by_name(None)
                            {
                                Ok(account) => account,
                                Err(_) => return Err(TuiError::ConnectAccount),
                            };

                            self.normal.set_account(&account)?;
                        },
                        BackendActions::WritingMail => {
                            self.mode = TuiMode::Writing;
                            self.need_redraw = true;
                        },
                        BackendActions::ViewingMail => {
                            self.mode = TuiMode::Viewing;
                            self.need_redraw = true;
                        },
                    };
                };
            },
            TuiMode::Writing => {
                if let Some(action) = self.writer.handle_event(event) {
                    match action {
                        BackendActions::Quit => {
                            self.mode = TuiMode::Normal;
                            self.need_redraw = true;
                        },
                        _ => (),
                    }
                }
            },
            TuiMode::Viewing => {
                if let Some(action) = self.viewer.handle_event(event) {
                    match action {
                        BackendActions::Quit => {
                            self.mode = TuiMode::Normal;
                            self.need_redraw = true;
                        },
                        BackendActions::Redraw => self.need_redraw = true,
                        _ => (),
                    }
                }
            },
        };
        Ok(())
    }

    pub fn draw<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        // prepare the given frame
        match self.mode {
            TuiMode::Normal => self.normal.draw(frame),
            TuiMode::Writing => self.writer.draw(frame),
            TuiMode::Viewing => self.viewer.draw(frame),
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
        self.normal.set_account(account)?;

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
                },
            };
        }
        terminal.clear()?;
        return self.cleanup();
    }
}
