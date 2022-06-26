use anyhow::{Context, Result};
use himalaya_lib::{
    account::{AccountConfig, DEFAULT_DRAFT_FOLDER, DEFAULT_SENT_FOLDER},
    backend::Backend,
    msg::{local_draft_path, remove_local_draft, Msg, TplOverride},
};
use log::{debug, info};
use std::{env, fs, process::Command};

use crate::{
    output::PrinterService,
    smtp::SmtpService,
    ui::choice::{self, PostEditChoice, PreEditChoice},
};

pub fn open_with_tpl(tpl: String) -> Result<String> {
    let path = local_draft_path();

    debug!("create draft");
    fs::write(&path, tpl.as_bytes()).context(format!("cannot write local draft at {:?}", path))?;

    debug!("open editor");
    Command::new(env::var("EDITOR").context(r#"cannot find "$EDITOR" env var"#)?)
        .arg(&path)
        .status()
        .context("cannot launch editor")?;

    debug!("read draft");
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;

    Ok(content)
}

pub fn open_with_draft() -> Result<String> {
    let path = local_draft_path();
    let tpl =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;
    open_with_tpl(tpl)
}

fn _edit_msg_with_editor(msg: &Msg, tpl: TplOverride, account: &AccountConfig) -> Result<Msg> {
    let tpl = msg.to_tpl(tpl, account)?;
    let tpl = open_with_tpl(tpl)?;
    Msg::from_tpl(&tpl).context("cannot parse message from template")
}

pub fn edit_msg_with_editor<'a, P: PrinterService, B: Backend<'a> + ?Sized, S: SmtpService>(
    mut msg: Msg,
    tpl: TplOverride,
    account: &AccountConfig,
    printer: &mut P,
    backend: Box<&'a mut B>,
    smtp: &mut S,
) -> Result<Box<&'a mut B>> {
    info!("start editing with editor");

    let draft = local_draft_path();
    if draft.exists() {
        loop {
            match choice::pre_edit() {
                Ok(choice) => match choice {
                    PreEditChoice::Edit => {
                        let tpl = open_with_draft()?;
                        msg.merge_with(Msg::from_tpl(&tpl)?);
                        break;
                    }
                    PreEditChoice::Discard => {
                        msg.merge_with(_edit_msg_with_editor(&msg, tpl.clone(), account)?);
                        break;
                    }
                    PreEditChoice::Quit => return Ok(backend),
                },
                Err(err) => {
                    println!("{}", err);
                    continue;
                }
            }
        }
    } else {
        msg.merge_with(_edit_msg_with_editor(&msg, tpl.clone(), account)?);
    }

    loop {
        match choice::post_edit() {
            Ok(PostEditChoice::Send) => {
                printer.print_str("Sending message…")?;
                let sent_msg = smtp.send(account, &msg)?;
                let sent_folder = account
                    .mailboxes
                    .get("sent")
                    .map(|s| s.as_str())
                    .unwrap_or(DEFAULT_SENT_FOLDER);
                printer.print_str(format!("Adding message to the {:?} folder…", sent_folder))?;
                backend.add_msg(&sent_folder, &sent_msg, "seen")?;
                remove_local_draft()?;
                printer.print_struct("Done!")?;
                break;
            }
            Ok(PostEditChoice::Edit) => {
                msg.merge_with(_edit_msg_with_editor(&msg, tpl.clone(), account)?);
                continue;
            }
            Ok(PostEditChoice::LocalDraft) => {
                printer.print_struct("Message successfully saved locally")?;
                break;
            }
            Ok(PostEditChoice::RemoteDraft) => {
                let tpl = msg.to_tpl(TplOverride::default(), account)?;
                let draft_folder = account
                    .mailboxes
                    .get("draft")
                    .map(|s| s.as_str())
                    .unwrap_or(DEFAULT_DRAFT_FOLDER);
                backend.add_msg(&draft_folder, tpl.as_bytes(), "seen draft")?;
                remove_local_draft()?;
                printer.print_struct(format!("Message successfully saved to {}", draft_folder))?;
                break;
            }
            Ok(PostEditChoice::Discard) => {
                remove_local_draft()?;
                break;
            }
            Err(err) => {
                println!("{}", err);
                continue;
            }
        }
    }

    Ok(backend)
}
