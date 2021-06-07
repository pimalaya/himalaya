use crossterm::event::Event;

use tui_rs::backend::Backend;
use tui_rs::layout::{Constraint, Direction, Layout, Rect};
use tui_rs::terminal::Frame;

use crate::config::model::Config;
use crate::config::tui::tui::TuiConfig;
use crate::tui::model::BackendActions;
use crate::tui::modes::widgets::{attachments::Attachments, header::Header};
use crate::tui::tabs::account_tab::AccountTab;

use crate::tui::modes::{
    backend_interface::BackendInterface, keybinding_manager::KeybindingManager,
};

// ==========
// Enums
// ==========
#[derive(Clone, Debug)]
pub enum WriterAction {
    SetBcc,
    SetCc,
    SetReplyTo,
    SetTo,
    SetSubject,
    Quit,
}

// ============
// Structs
// ============
pub struct Writer {
    header:      Header,
    attachments: Attachments,

    keybinding_manager: KeybindingManager<WriterAction>,
}
impl Writer {
    pub fn new(config: &Config) -> Self {
        let keybindings = TuiConfig::parse_keybindings(
            &config.tui.writer.default_keybindings,
            &config.tui.writer.keybindings,
        );

        let keybinding_manager = KeybindingManager::new(keybindings);

        Self {
            header: Header::new(&config.tui.writer.header),
            attachments: Attachments::new(&config.tui.writer.attachments),
            keybinding_manager,
        }
    }
}

impl BackendInterface for Writer {
    fn handle_event<'event>(
        &mut self,
        event: Event,
        account: &mut AccountTab<'event>,
    ) -> Option<BackendActions> {
        if let Some(action) = self.keybinding_manager.eval_event(event) {
            match action {
                WriterAction::Quit => Some(BackendActions::Quit),
                _ => None,
            }
        } else {
            None
        }
    }

    fn draw<'draw, B>(
        &mut self,
        frame: &mut Frame<B>,
        free_space: Rect,
        account: &mut AccountTab<'draw>,
    ) where
        B: Backend,
    {
        let layout = Layout::default()
            .margin(1)
            .direction(Direction::Vertical)
            .constraints(
                [Constraint::Percentage(25), Constraint::Percentage(75)]
                    .as_ref(),
            )
            .split(free_space);

        frame.render_widget(self.header.widget(), layout[0]);
        frame.render_widget(self.attachments.widget(), layout[1]);
    }
}
