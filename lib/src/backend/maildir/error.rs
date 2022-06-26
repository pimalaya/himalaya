use std::{io, path};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MaildirError {
    #[error("cannot find maildir sender")]
    FindSenderError,
    #[error("cannot read maildir directory {0}")]
    ReadDirError(path::PathBuf),
    #[error("cannot parse maildir subdirectory {0}")]
    ParseSubdirError(path::PathBuf),
    #[error("cannot get maildir envelopes at page {0}")]
    GetEnvelopesOutOfBoundsError(usize),
    #[error("cannot search maildir envelopes: feature not implemented")]
    SearchEnvelopesUnimplementedError,
    #[error("cannot get maildir message {0}")]
    GetMsgError(String),
    #[error("cannot decode maildir entry")]
    DecodeEntryError(#[source] io::Error),
    #[error("cannot parse maildir message")]
    ParseMsgError(#[source] maildir::MailEntryError),
    #[error("cannot decode header {0}")]
    DecodeHeaderError(#[source] rfc2047_decoder::Error, String),
    #[error("cannot parse maildir message header {0}")]
    ParseHeaderError(#[source] mailparse::MailParseError, String),
    #[error("cannot create maildir subdirectory {1}")]
    CreateSubdirError(#[source] io::Error, String),
    #[error("cannot decode maildir subdirectory")]
    DecodeSubdirError(#[source] io::Error),
    #[error("cannot delete subdirectories at {1}")]
    DeleteAllDirError(#[source] io::Error, path::PathBuf),
    #[error("cannot get current directory")]
    GetCurrentDirError(#[source] io::Error),
    #[error("cannot store maildir message with flags")]
    StoreWithFlagsError(#[source] maildir::MaildirError),
    #[error("cannot copy maildir message")]
    CopyMsgError(#[source] io::Error),
    #[error("cannot move maildir message")]
    MoveMsgError(#[source] io::Error),
    #[error("cannot delete maildir message")]
    DelMsgError(#[source] io::Error),
    #[error("cannot add maildir flags")]
    AddFlagsError(#[source] io::Error),
    #[error("cannot set maildir flags")]
    SetFlagsError(#[source] io::Error),
    #[error("cannot remove maildir flags")]
    DelFlagsError(#[source] io::Error),
}
