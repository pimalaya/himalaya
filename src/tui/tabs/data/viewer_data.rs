use crate::msg::model::{Attachment, ReadableMsg};
use crate::tui::modes::state_wrappers::TableStateWrapper;
use crate::tui::model::TuiError;
use crate::imap::model::ImapConnector;
use crate::config::model::Account;

use super::shared_widgets::header::Header;

use tui_rs::text::{Span, Spans};

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

// ===========
// Struct
// ===========
pub struct ViewerData<'viewer> {
    pub show_attachments: bool,
    pub content:          MailContent<'viewer>,
    pub header:           Header,

    // Placeholder
    pub attachments:   Vec<Attachment>,
    attachments_state: TableStateWrapper,
}

impl<'viewer> Default for ViewerData<'viewer> {
    fn default() -> Self {
        Self {
            show_attachments:  false,
            content:           MailContent::default(),
            header:            Header::default(),
            attachments:       Vec::new(),
            attachments_state: TableStateWrapper::new(None),
        }
    }
}

impl<'viewer> ViewerData<'viewer> {
    pub fn set_content(
        &mut self,
        account: &Account,
        mbox: &str,
        uid: &u32,
    ) -> Result<(), TuiError> {

        let uid = uid.to_string();

        let mut imap_conn = match ImapConnector::new(&account) {
            Ok(connection) => connection,
            Err(_) => return Err(TuiError::ConnectAccount),
        };

        let msg = match imap_conn.read_msg(mbox, &uid) {
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
                    "Couldn't convert the mail to mime-type [text/plain]",
                ),
                has_attachment: false,
            },
        };

        self.content.set_content(&msg.content);

        imap_conn.logout();

        Ok(())
    }
}

// ----------------
// MailContent
// ----------------
pub struct MailContent<'content> {
    pub x_offset: u16,
    pub y_offset: u16,
    pub content:  Vec<Spans<'content>>,
    parser:       SyntaxSet,
    theme:        ThemeSet,
}

impl<'content> MailContent<'content> {
    pub fn new() -> Self {
        let parser = SyntaxSet::load_defaults_newlines();
        let theme = ThemeSet::load_defaults();

        Self {
            x_offset: 0,
            y_offset: 0,
            content: Vec::new(),
            parser,
            theme,
        }
    }

    pub fn set_content(&mut self, new_content: &str) {
        // Since we load a new mail, we should reset the offset
        self.x_offset = 0;
        self.y_offset = 0;
        self.content.clear();

        let syntax = self.parser.find_syntax_by_extension("md").unwrap();
        let mut highlighter = HighlightLines::new(
            syntax,
            &self.theme.themes["Solarized (light)"],
        );

        for line in LinesWithEndings::from(new_content) {
            let mut converted_line: Vec<Span> = Vec::new();

            let ranges: Vec<(Style, &str)> =
                highlighter.highlight(line, &self.parser);

            for piece in ranges {
                let red = piece.0.foreground.r;
                let green = piece.0.foreground.g;
                let blue = piece.0.foreground.b;
                let text_part = piece.1.trim().to_string();

                converted_line.push(Span::styled(
                    text_part,
                    tui_rs::style::Style::default()
                        .fg(tui_rs::style::Color::Rgb(red, green, blue)),
                ));
            }

            self.content.push(Spans::from(converted_line));
        }
    }

    pub fn add_offset(&mut self, x: u16, y: u16) {
        self.x_offset = self.x_offset.saturating_add(x);
        self.y_offset = self.y_offset.saturating_add(y);
    }

    pub fn sub_offset(&mut self, x: u16, y: u16) {
        self.x_offset = self.x_offset.saturating_sub(x);
        self.y_offset = self.y_offset.saturating_sub(y);
    }
}

impl<'content> Default for MailContent<'content> {
    fn default() -> Self {
        let parser = SyntaxSet::load_defaults_newlines();
        let theme = ThemeSet::load_defaults();

        Self {
            x_offset: 0,
            y_offset: 0,
            content: Vec::new(),
            parser,
            theme,
        }
    }
}
