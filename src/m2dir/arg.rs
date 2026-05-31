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

const INBOX: &str = "Inbox";

#[derive(Debug, Parser)]
pub struct M2dirNameArg {
    /// Name of the m2dir folder, relative to the m2store root.
    #[arg(name = "m2dir_name", value_name = "NAME")]
    pub inner: String,
}

#[derive(Debug, Parser)]
pub struct M2dirNameFlag {
    /// Name of the m2dir folder, relative to the m2store root.
    #[arg(name = "m2dir_source_name", long = "m2dir", short = 'm')]
    #[arg(value_name = "NAME", default_value = INBOX)]
    pub inner: String,
}

#[derive(Debug, Parser)]
pub struct MessageIdArg {
    /// Identifier of the message.
    #[arg(name = "message_id", value_name = "ID")]
    pub inner: String,
}

#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// Identifier(s) of message(s).
    #[arg(name = "message_ids", value_name = "ID")]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
}
