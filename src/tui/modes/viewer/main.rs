use crate::config::model::Config;
use crate::config::tui::tui::TuiConfig;
use crate::tui::model::BackendActions;
use crate::tui::modes::{
    backend_interface::BackendInterface, keybinding_manager::KeybindingManager,
};
use crate::tui::tabs::account_tab::AccountTab;
use crate::tui::modes::widgets::{
    attachments::Attachments,
    header::Header,
};

use tui_rs::backend::Backend;
use tui_rs::terminal::Frame;
use tui_rs::layout::{Constraint, Direction, Layout, Rect};

use crossterm::event::Event;

use super::mail_content::ContentWidget;

// ==========
// Enums
// ==========
#[derive(Clone, Debug)]
pub enum ViewerAction {
    Quit,
    AddOffset(u16, u16),
    SubOffset(u16, u16),
    ToggleAttachment,
}

// ===========
// Struct
// ===========
pub struct Viewer {
    content: ContentWidget,
    attachments: Attachments,
    header: Header,
    keybinding_manager: KeybindingManager<ViewerAction>,
}

impl Viewer {
    pub fn new(config: &Config) -> Self {
        let keybindings = TuiConfig::parse_keybindings(
            &config.tui.viewer.default_keybindings,
            &config.tui.viewer.keybindings,
        );

        let keybinding_manager = KeybindingManager::new(keybindings);

        Self {
            keybinding_manager,
            attachments: Attachments::new(&config.tui.viewer.attachments),
            header: Header::new(&config.tui.viewer.header),
            content: ContentWidget::new(&config.tui.viewer.mailcontent),
        }
    }
}

impl BackendInterface for Viewer {
    fn handle_event<'event>(
        &mut self,
        event: Event,
        account: &mut AccountTab<'event>,
    ) -> Option<BackendActions> {

        if let Some(action) = self.keybinding_manager.eval_event(event) {
            match action {
                ViewerAction::Quit => Some(BackendActions::Quit),
                ViewerAction::ToggleAttachment => {
                    account.viewer.show_attachments = !account.viewer.show_attachments;
                    Some(BackendActions::Redraw)
                },
                ViewerAction::AddOffset(x, y) => {
                    account.viewer.content.add_offset(x, y);
                    Some(BackendActions::Redraw)
                },
                ViewerAction::SubOffset(x, y) => {
                    account.viewer.content.sub_offset(x, y);
                    Some(BackendActions::Redraw)
                },
            }
        } else {
            None
        }
    }

    fn draw<'draw, B>(&mut self, frame: &mut Frame<B>, free_space: Rect, account: &mut AccountTab<'draw>)
    where
        B: Backend,
    {

        let account = &mut account.viewer;

        if account.show_attachments {
            // ---------------------
            // Widget positions
            // ---------------------
            let layer1 = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [Constraint::Percentage(70), Constraint::Percentage(30)]
                        .as_ref(),
                )
                .split(free_space);

            let layer2 = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [Constraint::Percentage(25), Constraint::Percentage(75)]
                        .as_ref(),
                )
                .split(layer1[0]);

            // --------------
            // Rendering
            // --------------
            frame.render_widget(self.attachments.widget(), layer1[1]);
            frame.render_widget(self.header.widget(), layer2[0]);

            frame.render_widget(self.content.widget(&account.content), layer2[1]);
        } else {
            let layer = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [Constraint::Percentage(25), Constraint::Percentage(75)]
                        .as_ref(),
                )
                .split(free_space);

            frame.render_widget(self.header.widget(), layer[0]);

            frame.render_widget(self.content.widget(&account.content), layer[1]);
        }
    }
}
