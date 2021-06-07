// ===========
// Crates
// =========== */
use std::io;

use serde::Deserialize;

use tui_rs::backend::{Backend, CrosstermBackend};
use tui_rs::layout::{Constraint, Direction, Layout};
use tui_rs::style::{Color, Style};
use tui_rs::terminal::Frame;
use tui_rs::text::{Span, Spans};
use tui_rs::widgets::{Block, Tabs};
use tui_rs::Terminal;

use crossterm::event::Event;
use crossterm::terminal;

use crate::tui::modes::{
    backend_interface::BackendInterface, normal::main::NormalFrame,
    viewer::main::Viewer, writer::main::Writer,
};

use crate::config::model::Config;
use crate::tui::modes::block_data::BlockData;
use crate::tui::tabs::account_tab::AccountTab;

// ================
// Tui - Enums
// ================
// -----------
// Errors
// -----------
#[derive(Debug)]
pub enum TuiError {
    ConnectAccount,
    Draw,
    EventKey,
    MailList,
    RawMode(crossterm::ErrorKind),
    Sidebar,
    TerminalPreparation(io::Error),
    UnknownAccount,
    GetMailboxes,
    GetMails,
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

    // Accounts
    accounts:      Vec<AccountTab<'tui>>,
    account_index: usize,
    account_block: BlockData,

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
        // Create all modes
        let normal = NormalFrame::new(&config);
        let writer = Writer::new(&config);
        let viewer = Viewer::new(&config);

        // Load all accounts and prepare the normal-mode for them, since this
        // will be their default view, when starting
        let accounts: Vec<AccountTab<'tui>> = config
            .accounts
            .values()
            .map(|account| {
                AccountTab::new(account.clone(), TuiMode::Normal).unwrap()
            })
            .collect();

        Tui {
            normal,
            viewer,
            accounts,
            account_index: 0,
            account_block: BlockData::new(
                String::from("Accounts"),
                &config.tui.account_block,
            ),
            writer,
            config: config,
            need_redraw: true,     // draw the TUI
            run: true,             // let the event loop run
            mode: TuiMode::Normal, // default when startup: Normal
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
                if let Some(action) = self
                    .normal
                    .handle_event(event, &mut self.accounts[self.account_index])
                {
                    match action {
                        BackendActions::Quit => self.run = false,
                        BackendActions::Redraw => self.need_redraw = true,
                        BackendActions::GetAccount => (),
                        BackendActions::WritingMail => {
                            self.mode = TuiMode::Writing;
                            self.need_redraw = true;
                        },
                        BackendActions::ViewingMail => {
                            self.mode = TuiMode::Viewing;
                            self.need_redraw = true;

                            let uid = self.accounts[self.account_index]
                                .normal
                                .get_current_mail_uid();

                            let mbox = self.accounts[self.account_index]
                                .normal
                                .get_current_mailbox_name();

                            let account_tab =
                                &mut self.accounts[self.account_index];
                            account_tab.viewer.set_content(
                                &account_tab.account,
                                &mbox,
                                &uid,
                            )?;
                        },
                    };
                };
            },
            TuiMode::Writing => {
                if let Some(action) = self
                    .writer
                    .handle_event(event, &mut self.accounts[self.account_index])
                {
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
                if let Some(action) = self
                    .viewer
                    .handle_event(event, &mut self.accounts[self.account_index])
                {
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
        // preserve some spaces for the tab with the users
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [Constraint::Length(3), Constraint::Length(10)]
                    .as_ref(),
            )
            .margin(1)
            .split(frame.size());

        frame.render_widget(self.get_user_tabs(), layout[0]);

        // prepare the given frame
        match self.mode {
            TuiMode::Normal => self.normal.draw(
                frame,
                layout[1],
                &mut self.accounts[self.account_index],
            ),
            // TuiMode::Writing => self.writer.draw(frame),
            TuiMode::Viewing => self.viewer.draw(
                frame,
                layout[1],
                &mut self.accounts[self.account_index],
            ),
            _ => (),
        };

        self.need_redraw = false;
    }

    pub fn get_user_tabs(&mut self) -> Tabs {
        let account_tabs = self
            .accounts
            .iter()
            .map(|account_tab| {
                let account_name = match &account_tab.account.name {
                    Some(name) => name.clone(),
                    None => self.config.name.clone(),
                };
                Spans::from(Span::raw(account_name))
            })
            .collect();

        let block = Block::from(self.account_block.clone());

        Tabs::new(account_tabs)
            .block(block)
            .highlight_style(Style::default().fg(Color::Green))
            .select(self.account_index)
            .divider("|")
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
            // has to be done (no redraw or somehting like that)
            // HINT: If we need to do something in parallel, use poll.
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
