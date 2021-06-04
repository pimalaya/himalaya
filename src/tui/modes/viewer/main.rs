use crate::config::model::{Account, Config};
use crate::config::tui::tui::TuiConfig;
use crate::imap::model::ImapConnector;
use crate::msg::model::ReadableMsg;
use crate::tui::model::{BackendActions, TuiError};
use crate::tui::modes::{
    backend_interface::BackendInterface, keybinding_manager::KeybindingManager,
};

use tui_rs::backend::Backend;
use tui_rs::layout::{Constraint, Direction, Layout};
use tui_rs::terminal::Frame;

use crossterm::event::Event;

use crate::tui::modes::widgets::{attachments::Attachments, header::Header};

use super::mail_content::MailContent;

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
pub struct Viewer<'viewer> {
    attachments: Attachments,
    header:      Header,
    content:     MailContent<'viewer>,

    show_attachments: bool,

    keybinding_manager: KeybindingManager<ViewerAction>,
}

impl<'viewer> Viewer<'viewer> {
    pub fn new(config: &Config) -> Self {
        let keybindings = TuiConfig::parse_keybindings(
            &config.tui.viewer.default_keybindings,
            &config.tui.viewer.keybindings,
        );

        let keybinding_manager = KeybindingManager::new(keybindings);

        let attachments = Attachments::new(&config.tui.viewer.attachments);
        let content = MailContent::new(&config.tui.viewer.mailcontent);
        let header = Header::new(&config.tui.viewer.header);

        Self {
            attachments,
            header,
            keybinding_manager,
            show_attachments: false,
            content,
        }
    }

    pub fn load_mail(
        &mut self,
        account: &Account,
        mbox: &str,
        uid: &str,
    ) -> Result<(), TuiError> {
        let mut imap_conn = match ImapConnector::new(account) {
            Ok(connection) => connection,
            Err(_) => return Err(TuiError::ConnectAccount),
        };

        let msg = match imap_conn.read_msg(mbox, uid) {
            Ok(msg) => msg,
            Err(_) => {
                b"Couldn't load the selected mail from the imap server..."
                    .to_vec()
            },
        };

        let msg = match ReadableMsg::from_bytes("text/plain", &msg) {
            Ok(readable_msg) => readable_msg,
            Err(_) => ReadableMsg {
                content:        String::from(
                    "Couldn't convert the mail to mime-type[text/plain]",
                ),
                has_attachment: false,
            },
        };

        self.content.set_content(&msg.content);

        imap_conn.logout();

        Ok(())
    }
}

impl<'viewer> BackendInterface for Viewer<'viewer> {
    fn handle_event(&mut self, event: Event) -> Option<BackendActions> {
        if let Some(action) = self.keybinding_manager.eval_event(event) {
            match action {
                ViewerAction::Quit => Some(BackendActions::Quit),
                ViewerAction::ToggleAttachment => {
                    self.show_attachments = !self.show_attachments;
                    Some(BackendActions::Redraw)
                },
                ViewerAction::AddOffset(x, y) => {
                    self.content.add_offset(x, y);
                    Some(BackendActions::Redraw)
                },
                ViewerAction::SubOffset(x, y) => {
                    self.content.sub_offset(x, y);
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
            // ---------------------
            // Widget positions
            // ---------------------
            let layer1 = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [Constraint::Percentage(70), Constraint::Percentage(30)]
                        .as_ref(),
                )
                .split(frame.size());

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

            // frame.render_stateful_widget(
            //     self.content.widget(),
            //     layer2[1],
            //     self.content.get_state(),
            // );

            frame.render_widget(self.content.widget(), layer2[1]);
        } else {
            let layer = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [Constraint::Percentage(25), Constraint::Percentage(75)]
                        .as_ref(),
                )
                .split(frame.size());

            frame.render_widget(self.header.widget(), layer[0]);

            // frame.render_stateful_widget(
            //     self.content.widget(),
            //     layer[1],
            //     self.content.get_state(),
            // );
            frame.render_widget(self.content.widget(), layer[1]);
        }
    }
}
