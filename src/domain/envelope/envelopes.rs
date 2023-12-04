use anyhow::Result;
use email::account::config::AccountConfig;
use serde::Serialize;
use std::ops;

use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
    Envelope, IdMapper, Mailbox,
};

/// Represents the list of envelopes.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelopes(Vec<Envelope>);

impl Envelopes {
    pub fn from_backend(
        config: &AccountConfig,
        id_mapper: &IdMapper,
        envelopes: email::envelope::Envelopes,
    ) -> Result<Envelopes> {
        let envelopes = envelopes
            .iter()
            .map(|envelope| {
                Ok(Envelope {
                    id: id_mapper.get_or_create_alias(&envelope.id)?,
                    flags: envelope.flags.clone().into(),
                    subject: envelope.subject.clone(),
                    from: Mailbox {
                        name: envelope.from.name.clone(),
                        addr: envelope.from.addr.clone(),
                    },
                    date: envelope.format_date(config),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Envelopes(envelopes))
    }
}

impl ops::Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PrintTable for Envelopes {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use email::account::config::AccountConfig;
    use std::env;

    use crate::{Envelopes, IdMapper};

    #[test]
    fn default_datetime_fmt() {
        let config = AccountConfig::default();
        let id_mapper = IdMapper::Dummy;

        let envelopes = email::envelope::Envelopes::from_iter([email::envelope::Envelope {
            date: DateTime::parse_from_rfc3339("2023-06-15T09:42:00+04:00").unwrap(),
            ..Default::default()
        }]);
        let envelopes = Envelopes::from_backend(&config, &id_mapper, envelopes).unwrap();

        let expected_date = "2023-06-15 09:42+04:00";
        let date = &envelopes.first().unwrap().date;

        assert_eq!(date, expected_date);
    }

    #[test]
    fn custom_datetime_fmt() {
        let id_mapper = IdMapper::Dummy;
        let config = AccountConfig {
            email_listing_datetime_fmt: Some("%d/%m/%Y %Hh%M".into()),
            ..AccountConfig::default()
        };

        let envelopes = email::envelope::Envelopes::from_iter([email::envelope::Envelope {
            date: DateTime::parse_from_rfc3339("2023-06-15T09:42:00+04:00").unwrap(),
            ..Default::default()
        }]);
        let envelopes = Envelopes::from_backend(&config, &id_mapper, envelopes).unwrap();

        let expected_date = "15/06/2023 09h42";
        let date = &envelopes.first().unwrap().date;

        assert_eq!(date, expected_date);
    }

    #[test]
    fn custom_datetime_fmt_with_local_tz() {
        env::set_var("TZ", "UTC");

        let id_mapper = IdMapper::Dummy;
        let config = AccountConfig {
            email_listing_datetime_fmt: Some("%d/%m/%Y %Hh%M".into()),
            email_listing_datetime_local_tz: Some(true),
            ..AccountConfig::default()
        };

        let envelopes = email::envelope::Envelopes::from_iter([email::envelope::Envelope {
            date: DateTime::parse_from_rfc3339("2023-06-15T09:42:00+04:00").unwrap(),
            ..Default::default()
        }]);
        let envelopes = Envelopes::from_backend(&config, &id_mapper, envelopes).unwrap();

        let expected_date = "15/06/2023 05h42";
        let date = &envelopes.first().unwrap().date;

        assert_eq!(date, expected_date);
    }
}
