use anyhow::anyhow;
use dialoguer::Validator;
use email_address::EmailAddress;

pub(crate) struct EmailValidator;

impl<T: ToString> Validator<T> for EmailValidator {
    type Err = anyhow::Error;

    fn validate(&mut self, input: &T) -> Result<(), Self::Err> {
        let input = input.to_string();
        if EmailAddress::is_valid(&input) {
            Ok(())
        } else {
            Err(anyhow!("Invalid email address: {}", input))
        }
    }
}
