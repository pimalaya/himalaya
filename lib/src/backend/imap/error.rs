use std::result;
use thiserror::Error;

use crate::{
    account,
    msg::{self, Flags},
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot get envelope of message {0}")]
    GetEnvelopeError(u32),
    #[error("cannot get sender of message {0}")]
    GetSenderError(u32),
    #[error("cannot get imap session")]
    GetSessionError,
    #[error("cannot retrieve message {0}'s uid")]
    GetMsgUidError(u32),
    #[error("cannot find message {0}")]
    FindMsgError(String),
    #[error("cannot parse sort criterion {0}")]
    ParseSortCriterionError(String),

    #[error("cannot decode subject of message {1}")]
    DecodeSubjectError(#[source] rfc2047_decoder::Error, u32),
    #[error("cannot decode sender name of message {1}")]
    DecodeSenderNameError(#[source] rfc2047_decoder::Error, u32),
    #[error("cannot decode sender mailbox of message {1}")]
    DecodeSenderMboxError(#[source] rfc2047_decoder::Error, u32),
    #[error("cannot decode sender host of message {1}")]
    DecodeSenderHostError(#[source] rfc2047_decoder::Error, u32),

    #[error("cannot create tls connector")]
    CreateTlsConnectorError(#[source] native_tls::Error),
    #[error("cannot connect to imap server")]
    ConnectImapServerError(#[source] imap::Error),
    #[error("cannot login to imap server")]
    LoginImapServerError(#[source] imap::Error),
    #[error("cannot search new messages")]
    SearchNewMsgsError(#[source] imap::Error),
    #[error("cannot examine mailbox {1}")]
    ExamineMboxError(#[source] imap::Error, String),
    #[error("cannot start the idle mode")]
    StartIdleModeError(#[source] imap::Error),
    #[error("cannot parse message {1}")]
    ParseMsgError(#[source] mailparse::MailParseError, String),
    #[error("cannot fetch new messages envelope")]
    FetchNewMsgsEnvelopeError(#[source] imap::Error),
    #[error("cannot get uid of message {0}")]
    GetUidError(u32),
    #[error("cannot create mailbox {1}")]
    CreateMboxError(#[source] imap::Error, String),
    #[error("cannot list mailboxes")]
    ListMboxesError(#[source] imap::Error),
    #[error("cannot delete mailbox {1}")]
    DeleteMboxError(#[source] imap::Error, String),
    #[error("cannot select mailbox {1}")]
    SelectMboxError(#[source] imap::Error, String),
    #[error("cannot fetch messages within range {1}")]
    FetchMsgsByRangeError(#[source] imap::Error, String),
    #[error("cannot fetch messages by sequence {1}")]
    FetchMsgsBySeqError(#[source] imap::Error, String),
    #[error("cannot append message to mailbox {1}")]
    AppendMsgError(#[source] imap::Error, String),
    #[error("cannot sort messages in mailbox {1} with query: {2}")]
    SortMsgsError(#[source] imap::Error, String, String),
    #[error("cannot search messages in mailbox {1} with query: {2}")]
    SearchMsgsError(#[source] imap::Error, String, String),
    #[error("cannot expunge mailbox {1}")]
    ExpungeError(#[source] imap::Error, String),
    #[error("cannot add flags {1} to message(s) {2}")]
    AddFlagsError(#[source] imap::Error, Flags, String),
    #[error("cannot set flags {1} to message(s) {2}")]
    SetFlagsError(#[source] imap::Error, Flags, String),
    #[error("cannot delete flags {1} to message(s) {2}")]
    DelFlagsError(#[source] imap::Error, Flags, String),
    #[error("cannot logout from imap server")]
    LogoutError(#[source] imap::Error),

    #[error(transparent)]
    AccountError(#[from] account::AccountError),
    #[error(transparent)]
    MsgError(#[from] msg::Error),
}

pub type Result<T> = result::Result<T, Error>;
