use clap;

use crate::{
    config::model::{Account, Config},
    output::model::Output,
};

pub struct App<'a> {
    pub config: &'a Config,
    pub account: &'a Account,
    pub output: &'a Output,
    pub mbox: &'a str,
    pub arg_matches: &'a clap::ArgMatches<'a>,
}

impl<'a> App<'a> {
    pub fn new(
        config: &'a Config,
        account: &'a Account,
        output: &'a Output,
        mbox: &'a str,
        arg_matches: &'a clap::ArgMatches<'a>,
    ) -> Self {
        Self {
            config,
            account,
            output,
            mbox,
            arg_matches,
        }
    }
}
