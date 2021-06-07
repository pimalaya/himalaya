use crate::config::model::Account;
use crate::imap::model::ImapConnector;
use crate::mbox::model::Mboxes;
use crate::msg::model::{Msg, Msgs};
use crate::tui::model::TuiError;
use crate::tui::modes::state_wrappers::TableStateWrapper;

// ============
// Structs
// ============
pub struct NormalData {
    pub msgs:   Vec<MailEntry>,
    pub mboxes: Vec<String>,

    pub display_sidebar: bool,

    pub sidebar_state:   TableStateWrapper,
    pub mail_list_state: TableStateWrapper,
}

impl NormalData {
    pub fn new(account: &Account) -> Result<Self, TuiError> {
        let mut msgs = Vec::new();
        let mut mboxes = Vec::new();

        let mut imap_conn = match ImapConnector::new(&account) {
            Ok(connection) => connection,
            Err(_) => return Err(TuiError::ConnectAccount),
        };

        // Mailboxes
        let imap_mbox_names = match imap_conn.list_mboxes() {
            Ok(names) => names,
            Err(_) => return Err(TuiError::GetMailboxes),
        };

        let imap_mboxes = Mboxes::from(&imap_mbox_names).0;
        for mbox in &imap_mboxes {
            mboxes.push(mbox.name.clone());
        }

        // msgs/mails
        let imap_msgs = match imap_conn.msgs(&imap_mboxes[0].name) {
            Ok(imap_msgs) => imap_msgs,
            Err(_) => return Err(TuiError::GetMails),
        };

        let imap_msgs = match imap_msgs {
            Some(ref fetches) => Msgs::from(fetches).0,
            None => Msgs::new().0,
        };

        for msg in imap_msgs {
            msgs.push(MailEntry::new(&msg));
        }

        assert!(mboxes.len() > 0);

        // -----------
        // States
        // -----------
        let sidebar_state = TableStateWrapper::new(Some(mboxes.len()));
        let mail_list_state = TableStateWrapper::new(Some(msgs.len()));

        imap_conn.logout();

        Ok(Self {
            mboxes,
            msgs,
            display_sidebar: true,
            sidebar_state,
            mail_list_state,
        })
    }

    pub fn get_current_mail_uid(&self) -> u32 {
        self.msgs[self.mail_list_state.get_selected_index()].uid
    }

    pub fn get_current_mailbox_name(&self) -> String {
        self.mboxes[self.sidebar_state.get_selected_index()].clone()
    }

    pub fn move_sidebar_cursor(&mut self, offset: i32) {
        self.sidebar_state.move_cursor(offset);
    }

    pub fn move_mail_list_cursor(&mut self, offset: i32) {
        self.mail_list_state.move_cursor(offset);
    }

    pub fn set_sidebar_cursor(&mut self, index: Option<usize>) {
        self.sidebar_state.set_cursor(index);
    }

    pub fn set_mail_list_cursor(&mut self, index: Option<usize>) {
        self.mail_list_state.set_cursor(index);
    }
}

impl Default for NormalData {
    fn default() -> NormalData {
        NormalData {
            msgs:            Vec::new(),
            mboxes:          Vec::new(),
            display_sidebar: true,
            sidebar_state:   TableStateWrapper::new(None),
            mail_list_state: TableStateWrapper::new(None),
        }
    }
}

// ---------
// MailEntry
// ---------
pub struct MailEntry {
    pub uid:     u32,
    pub subject: String,
    pub sender:  String,
    pub date:    String,
    pub flags:   String,
}

impl MailEntry {
    // Copy the data from the mail into our representation
    pub fn new<'new>(msg: &Msg<'new>) -> Self {
        Self {
            uid:     msg.uid,
            subject: msg.subject.clone(),
            sender:  msg.sender.clone(),
            date:    msg.date.clone(),
            flags:   msg.flags.to_string(),
        }
    }
}
