use crate::tui::modes::{
    backend_interface::BackendInterface, keybinding_manager::KeybindingManager,
};
use crate::config::tui::tui::TuiConfig;
use crate::config::model::Config;
use crate::tui::model::BackendActions;

use tui_rs::backend::Backend;
use tui_rs::terminal::Frame;
use tui_rs::layout::{Layout, Direction, Constraint};
use tui_rs::widgets::{Block, Borders};
// use tui_rs::widgets::Paragraph;

use crossterm::event::Event;

use crate::tui::modes::widgets::{attachments::Attachments, header::Header};

// ==========
// Enums
// ==========
#[derive(Clone, Debug)]
pub enum ViewerAction {
    Quit,
    ToggleAttachment,
}

// ===========
// Struct
// ===========
pub struct Viewer {
    attachments: Attachments,
    header:      Header,
    content:     Vec<String>,

    show_attachments: bool,

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
            attachments: Attachments::new(&config.tui.viewer.attachments),
            header: Header::new(&config.tui.viewer.header),
            keybinding_manager,
            show_attachments: true,
            content: Vec::new(),
        }
    }
}

impl BackendInterface for Viewer {
    fn handle_event(&mut self, event: Event) -> Option<BackendActions> {
        if let Some(action) = self.keybinding_manager.eval_event(event) {
            match action {
                ViewerAction::Quit => Some(BackendActions::Quit),
                ViewerAction::ToggleAttachment => 
                {
                    self.show_attachments = !self.show_attachments;
                    Some(BackendActions::Redraw)
                },
            }
        } else {
            None
        }
    }

    fn draw<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        if self.show_attachments {
            let layer1 = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ].as_ref())
                .split(frame.size());

            let layer2 = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(75),
                ].as_ref())
                .split(layer1[0]);

            frame.render_widget(
                self.attachments.widget(),
                layer1[1]
            );

            frame.render_widget(
                self.header.widget(),
                layer2[0]
            );

            frame.render_widget(
                Block::default().title("Content").borders(Borders::ALL),
                layer2[1]
            );
        }
        else {
            let layer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(75),
                ].as_ref())
                .split(frame.size());

            frame.render_widget(
                self.header.widget(),
                layer[0]
            );

            frame.render_widget(
                Block::default().title("Content").borders(Borders::ALL),
                layer[1]
            );
        }
    }
}
