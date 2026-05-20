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

use clap::ValueEnum;
use io_maildir::flag::Flag;

#[derive(Clone, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FlagArg {
    Passed,
    Replied,
    Seen,
    Trashed,
    Draft,
    Flagged,
}

impl From<FlagArg> for Flag {
    fn from(flag: FlagArg) -> Self {
        match flag {
            FlagArg::Passed => Flag::Passed,
            FlagArg::Replied => Flag::Replied,
            FlagArg::Seen => Flag::Seen,
            FlagArg::Trashed => Flag::Trashed,
            FlagArg::Draft => Flag::Draft,
            FlagArg::Flagged => Flag::Flagged,
        }
    }
}
