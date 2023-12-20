use anyhow::{Context, Result};
use email::{
    account::config::AccountConfig,
    email::utils::{local_draft_path, remove_local_draft},
    flag::{Flag, Flags},
    folder::DRAFTS,
};
use log::debug;
use mml::MmlCompilerBuilder;
use process::SingleCmd;
use std::{env, fs};

use crate::{
    backend::Backend,
    printer::Printer,
    ui::choice::{self, PostEditChoice, PreEditChoice},
};

pub async fn open_with_tpl(tpl: String) -> Result<String> {
    let path = local_draft_path();

    debug!("create draft");
    fs::write(&path, tpl.as_bytes()).context(format!("cannot write local draft at {:?}", path))?;

    debug!("open editor");
    let editor = env::var("EDITOR").context("cannot get editor from env var")?;
    SingleCmd::from(format!("{editor} {}", &path.to_string_lossy()))
        .with_output_piped(false)
        .run()
        .await
        .context("cannot launch editor")?;

    debug!("read draft");
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;

    Ok(content)
}

pub async fn open_with_local_draft() -> Result<String> {
    let path = local_draft_path();
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;
    open_with_tpl(content).await
}

pub async fn edit_tpl_with_editor<P: Printer>(
    #[allow(unused)] config: &AccountConfig,
    printer: &mut P,
    backend: &Backend,
    mut tpl: String,
) -> Result<()> {
    let draft = local_draft_path();
    if draft.exists() {
        loop {
            match choice::pre_edit() {
                Ok(choice) => match choice {
                    PreEditChoice::Edit => {
                        tpl = open_with_local_draft().await?;
                        break;
                    }
                    PreEditChoice::Discard => {
                        tpl = open_with_tpl(tpl).await?;
                        break;
                    }
                    PreEditChoice::Quit => return Ok(()),
                },
                Err(err) => {
                    println!("{}", err);
                    continue;
                }
            }
        }
    } else {
        tpl = open_with_tpl(tpl).await?;
    }

    loop {
        match choice::post_edit() {
            Ok(PostEditChoice::Send) => {
                printer.print_log("Sending email…")?;

                #[allow(unused_mut)]
                let mut compiler = MmlCompilerBuilder::new();

                #[cfg(feature = "pgp")]
                compiler.set_some_pgp(config.pgp.clone());

                let email = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

                backend.send_raw_message(&email).await?;

                remove_local_draft()?;
                printer.print("Done!")?;
                break;
            }
            Ok(PostEditChoice::Edit) => {
                tpl = open_with_tpl(tpl).await?;
                continue;
            }
            Ok(PostEditChoice::LocalDraft) => {
                printer.print("Email successfully saved locally")?;
                break;
            }
            Ok(PostEditChoice::RemoteDraft) => {
                #[allow(unused_mut)]
                let mut compiler = MmlCompilerBuilder::new();

                #[cfg(feature = "pgp")]
                compiler.set_some_pgp(config.pgp.clone());

                let email = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

                backend
                    .add_raw_message_with_flags(
                        DRAFTS,
                        &email,
                        &Flags::from_iter([Flag::Seen, Flag::Draft]),
                    )
                    .await?;
                remove_local_draft()?;
                printer.print("Email successfully saved to drafts")?;
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

    Ok(())
}
