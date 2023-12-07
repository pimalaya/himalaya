use clap::Parser;
use email::flag::{Flag, Flags};
use log::debug;

/// The ids and/or flags arguments parser
#[derive(Debug, Parser)]
pub struct IdsAndFlagsArgs {
    /// The list of ids and/or flags
    ///
    /// Every argument that can be parsed as an integer is considered
    /// an id, otherwise it is considered as a flag.
    #[arg(value_name = "ID-OR-FLAG", required = true)]
    pub ids_and_flags: Vec<IdOrFlag>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum IdOrFlag {
    Id(String),
    Flag(Flag),
}

impl From<&str> for IdOrFlag {
    fn from(value: &str) -> Self {
        value
            .parse::<usize>()
            .map(|_| Self::Id(value.to_owned()))
            .unwrap_or_else(|err| {
                let flag = Flag::from(value);
                debug!("cannot parse {value} as usize, parsing it as flag {flag}");
                debug!("{err:?}");
                Self::Flag(flag)
            })
    }
}

pub fn to_tuple<'a>(ids_and_flags: &'a [IdOrFlag]) -> (Vec<&'a str>, Flags) {
    ids_and_flags.iter().fold(
        (Vec::default(), Flags::default()),
        |(mut ids, mut flags), arg| {
            match arg {
                IdOrFlag::Id(id) => {
                    ids.push(id.as_str());
                }
                IdOrFlag::Flag(flag) => {
                    flags.insert(flag.to_owned());
                }
            };
            (ids, flags)
        },
    )
}
