use super::mail_credits::MailCredits;
use super::attachments::Attachments;

use crate::config::model::Config;
use crate::config::tui::tui::TuiConfig;
use crate::tui::model::BackendActions;
use crate::tui::modes::{
    backend_interface::BackendInterface, keybinding_manager::KeybindingManager,
};

use tui_rs::backend::Backend;
use tui_rs::terminal::Frame;
use tui_rs::layout::{Layout, Direction, Constraint};

use crossterm::event::Event;

// use crate::msg::tpl::model::Tpl;

// ==========
// Enums
// ==========
#[derive(Clone, Debug)]
pub enum WritingMailAction {
    SetBcc,
    SetCc,
    SetInReplyTo,
    SetSubject,
    SetTo,
    Quit,
}

// ============
// Structs
// ============
pub struct WritingMail {
    credits: MailCredits,
    attachments: Attachments,
    // template: Tpl,
    keybinding_manager: KeybindingManager<WritingMailAction>,
}

impl WritingMail {
    pub fn new(config: &Config) -> Self {
        // ----------------
        // Keybindings
        // ----------------
        let keybindings = TuiConfig::parse_keybindings(
            &config.tui.writing_mail.default_keybindings,
            &config.tui.writing_mail.keybindings,
        );

        let keybinding_manager = KeybindingManager::new(keybindings);

        let credits = MailCredits::new(
            String::from("tornax07@gmail.com"),
            &config.tui.writing_mail.mail_credits
        );

        let attachments = Attachments::new(&config.tui.writing_mail.attachments);

        Self {
            // template: Tpl::new(),
            credits,
            attachments,
            keybinding_manager,
        }
    }
}

impl BackendInterface for WritingMail {
    fn handle_event(&mut self, event: Event) -> Option<BackendActions> {
        if let Some(action) = self.keybinding_manager.eval_event(event) {
            match action {
                WritingMailAction::Quit => Some(BackendActions::Quit),
                _ => None,
            }
        } else {
            None
        }
    }

    fn draw<B: Backend>(&mut self, frame: &mut Frame<B>) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    // At the top the mail credits
                    Constraint::Percentage(25),
                    // At the bottom the listing
                    Constraint::Percentage(75),
                ]
                .as_ref(),
            )
            .split(frame.size());

        frame.render_widget(self.credits.widget(), layout[0]);
        frame.render_widget(self.attachments.widget(), layout[1]);

    }
}
