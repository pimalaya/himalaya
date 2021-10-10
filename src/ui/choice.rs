use anyhow::{anyhow, Context, Result};
use log::{debug, error};
use std::io::{self, Write};

pub enum PreEditChoice {
    Edit,
    Discard,
    Quit,
}

pub fn pre_edit() -> Result<PreEditChoice> {
    println!("A draft was found:");
    print!("(e)dit, (d)iscard or (q)uit? ");
    io::stdout().flush().context("cannot flush stdout")?;

    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .context("cannot read stdin")?;

    match buf.bytes().next().map(|bytes| bytes as char) {
        Some('e') => {
            debug!("edit choice matched");
            Ok(PreEditChoice::Edit)
        }
        Some('d') => {
            debug!("discard choice matched");
            Ok(PreEditChoice::Discard)
        }
        Some('q') => {
            debug!("quit choice matched");
            Ok(PreEditChoice::Quit)
        }
        Some(choice) => {
            error!(r#"invalid choice "{}""#, choice);
            Err(anyhow!(r#"invalid choice "{}""#, choice))
        }
        None => {
            error!("empty choice");
            Err(anyhow!("empty choice"))
        }
    }
}

pub enum PostEditChoice {
    Send,
    Edit,
    LocalDraft,
    RemoteDraft,
    Discard,
}

pub fn post_edit() -> Result<PostEditChoice> {
    print!("(s)end, (e)dit, (l)ocal/(r)emote draft or (d)iscard? ");
    io::stdout().flush().context("cannot flush stdout")?;

    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .context("cannot read stdin")?;

    match buf.bytes().next().map(|bytes| bytes as char) {
        Some('s') => {
            debug!("send choice matched");
            Ok(PostEditChoice::Send)
        }
        Some('l') => {
            debug!("save local draft choice matched");
            Ok(PostEditChoice::LocalDraft)
        }
        Some('r') => {
            debug!("save remote draft matched");
            Ok(PostEditChoice::RemoteDraft)
        }
        Some('e') => {
            debug!("edit choice matched");
            Ok(PostEditChoice::Edit)
        }
        Some('d') => {
            debug!("discard choice matched");
            Ok(PostEditChoice::Discard)
        }
        Some(choice) => {
            error!(r#"invalid choice "{}""#, choice);
            Err(anyhow!(r#"invalid choice "{}""#, choice))
        }
        None => {
            error!("empty choice");
            Err(anyhow!("empty choice"))
        }
    }
}
