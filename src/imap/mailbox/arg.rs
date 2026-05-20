// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use clap::Parser;

const INBOX: &str = "INBOX";

/// The optional mailbox name argument parser.
#[derive(Debug, Parser)]
pub struct MailboxNameOptionalArg {
    /// The name of the mailbox.
    #[arg(name = "mailbox_name", value_name = "MAILBOX", default_value = INBOX)]
    pub inner: String,
}

impl Default for MailboxNameOptionalArg {
    fn default() -> Self {
        Self {
            inner: INBOX.into(),
        }
    }
}

/// The optional mailbox name flag parser.
#[derive(Debug, Parser)]
pub struct MailboxNameOptionalFlag {
    /// The name of the mailbox.
    #[arg(long = "mailbox", short = 'm')]
    #[arg(name = "mailbox_name", value_name = "NAME", default_value = INBOX)]
    pub inner: String,
}

impl Default for MailboxNameOptionalFlag {
    fn default() -> Self {
        Self {
            inner: INBOX.into(),
        }
    }
}

#[derive(Debug, Parser)]
pub struct MailboxNoSelectFlag {
    /// Do not select the given mailbox before performing the current
    /// action.
    ///
    /// This argument is useful when stateful IMAP sessions are used,
    /// for example with Sirup CLI:
    ///
    /// https://github.com/pimalaya/sirup
    #[arg(long = "no-select", default_value_t)]
    #[arg(name = "mailbox_no_select")]
    pub inner: bool,
}

/// The required mailbox name argument parser.
#[derive(Debug, Parser)]
pub struct MailboxNameArg {
    /// The name of the mailbox.
    #[arg(name = "mailbox_name", value_name = "MAILBOX")]
    pub inner: String,
}

/// The target mailbox name argument parser.
#[derive(Debug, Clone, Parser)]
pub struct TargetMailboxNameArg {
    /// The name of the target mailbox.
    #[arg(name = "target_mailbox_name", value_name = "TARGET")]
    pub inner: String,
}
