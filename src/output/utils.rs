use error_chain::error_chain;
use serde::{
    ser::{self, SerializeStruct},
    Serialize,
};
use std::{fmt, process::Command, result};

error_chain! {
    foreign_links {
        Utf8(std::string::FromUtf8Error);
        Io(std::io::Error);
    }
}

pub struct Info(pub String);

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ser::Serialize for Info {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("Info", 1)?;
        state.serialize_field("info", &self.0)?;
        state.end()
    }
}

pub fn run_cmd(cmd: &str) -> Result<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", cmd]).output()
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()
    }?;

    Ok(String::from_utf8(output.stdout)?)
}

pub fn print<T: fmt::Display + Serialize>(output_type: &str, silent: &bool, item: T) -> Result<()> {
    if silent == &false {
        match output_type {
            "json" => print!(
                "{}",
                serde_json::to_string(&item).chain_err(|| "Could not decode JSON")?
            ),
            "text" | _ => println!("{}", item.to_string()),
        }
    }

    Ok(())
}
