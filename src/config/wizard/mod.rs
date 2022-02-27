mod wizard_error;
mod wizard_io;

use wizard_error::WizardError;
use wizard_io::WizardIO;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wizard {
    io: WizardIO,
}

impl Wizard {
    pub fn new() -> Self {
        Self {
            io: WizardIO::new(),
        }
    }

    pub fn start() -> Result<String, WizardError> {
        let wizard = Wizard::new();

        if !wizard.is_allowed_to_create_config() {
            return Err(WizardError::Unallowed);
        }

        Ok("Hi".to_string())
    }

    fn is_allowed_to_create_config(&self) -> bool {
        println!("Looks like that you don't have a config file.");
        print!("Should we create the config file together? [Y/n] ");

        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer);

        buffer == "Y" || buffer == "y"
    }
}
