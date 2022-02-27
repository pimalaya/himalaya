mod wizard_error;
mod wizard_io;

use wizard_error::WizardError;
use wizard_io::WizardIO;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wizard {
}

impl Wizard {
    pub fn start() -> Result<String, WizardError> {
        let wizard_io = WizardIO::new();

        Ok("Hi".to_string())
    }
}
