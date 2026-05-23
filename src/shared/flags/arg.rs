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

use std::fmt;

use clap::{Parser, ValueEnum};

/// Shared CLI flag argument for the cross-protocol `flags` and
/// `messages add` commands. The variant set is the strict
/// least-common-denominator across IMAP, JMAP and Maildir; backend
/// extras (`\Deleted`, Maildir `Trashed`/`Passed`, JMAP custom
/// keywords) live on the protocol-specific commands.
#[derive(Clone, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FlagArg {
    Seen,
    Answered,
    Flagged,
    Draft,
}

impl fmt::Display for FlagArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Seen => "seen",
            Self::Answered => "answered",
            Self::Flagged => "flagged",
            Self::Draft => "draft",
        };
        f.write_str(name)
    }
}

#[cfg(feature = "imap")]
impl FlagArg {
    pub fn imap(&self) -> io_imap::types::flag::Flag<'static> {
        use io_imap::types::flag::Flag;

        match self {
            Self::Seen => Flag::Seen,
            Self::Answered => Flag::Answered,
            Self::Flagged => Flag::Flagged,
            Self::Draft => Flag::Draft,
        }
    }
}

#[cfg(feature = "jmap")]
impl FlagArg {
    pub fn jmap(&self) -> &'static str {
        match self {
            Self::Seen => "$seen",
            Self::Answered => "$answered",
            Self::Flagged => "$flagged",
            Self::Draft => "$draft",
        }
    }
}

#[cfg(feature = "maildir")]
impl From<&FlagArg> for io_maildir::flag::Flag {
    fn from(flag: &FlagArg) -> Self {
        use io_maildir::flag::Flag;

        match flag {
            FlagArg::Seen => Flag::Seen,
            FlagArg::Answered => Flag::Replied,
            FlagArg::Flagged => Flag::Flagged,
            FlagArg::Draft => Flag::Draft,
        }
    }
}

impl From<&FlagArg> for io_email::flag::Flag {
    fn from(flag: &FlagArg) -> Self {
        use io_email::flag::{Flag, IanaFlag};

        let iana = match flag {
            FlagArg::Seen => IanaFlag::Seen,
            FlagArg::Answered => IanaFlag::Answered,
            FlagArg::Flagged => IanaFlag::Flagged,
            FlagArg::Draft => IanaFlag::Draft,
        };

        Flag::from_iana(iana)
    }
}

#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// Message Identifier(s).
    #[arg(name = "message_ids", value_name = "MESSAGE-IDS")]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct FlagsArg {
    /// Flag(s) to add on message(s).
    #[arg(name = "flags", value_name = "FLAG")]
    #[arg(long = "flag", short, required = true, num_args = 1..)]
    pub inner: Vec<FlagArg>,
}
