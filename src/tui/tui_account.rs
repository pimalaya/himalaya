use crate::imap::model::ImapConnector;
use crate::mbox::model::Mbox;
use crate::msg::model::{Msg};

pub struct TuiAccount<'account> {
    imap_conn: ImapConnector<'account>,
    messages: Vec<Msg<'account>>,
    mailboxes: Vec<Mbox<'account>>,
}

// impl<'account> TuiAccount<'account> {
//     pub fn new(imap_conn: ImapConnector<'account>) -> Result<TuiAccount, i32> {
//         // TODO: Read the mail information of the imap connection (after getting
//         // mailbox)
//         let mailboxes = match imap_conn.list_mboxes() {
//             Ok(names) => vec![*names],
//             Err(_) => return Err(-1),
//         };
//
//         // Get the messages from the first mailbox
//         let messages = match imap_conn.list_msgs(mailboxes[0][0].name(), &1, &1) {
//             Ok(messages) => messages,
//             Err(_) => return Err(-2),
//         };
//
//         let messages = match messages {
//             Some(ref fetches) => Msgs::from(fetches).0,
//             None => Msgs::new(),
//         };
//
//         Ok(TuiAccount {
//             imap_conn,
//             messages,
//             mailboxes[0],
//         })
//     }
// }
