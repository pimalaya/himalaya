---
cairn: change
id: config-paths-expand-at-deserialize
status: landed
created: 2026-08-29
---

# Every path key expands as the configuration is read

## Why

`config.sample.toml` writes `maildir.root = "~/Mail/example"`, and reading that account looked for a directory literally named `~` under the working directory. The same held for `m2dir.root` and for the `tls.cert` of every backend: the field is a bare `PathBuf`, serde hands it over verbatim, and the reader opens what it was given.

Two path keys did work, `pimdir.root` and `downloads-dir`, because the one place reading each of them called `shellexpand` first. That is the shape the bug is made of: expansion sat at the call site, so it held only where somebody remembered it, and a second reader of the same field inherited nothing. The pimdir requirement in the backends capability describes exactly that, a fix written for one backend.

The family answer is `pimalaya_config::toml::shell_expanded_path`, a `deserialize_with` on the field, which carillon already uses. Expansion then happens once, as the file is read, and no future reader can forget.

himalaya-tui reads the same file under the same project name, so its `MaildirConfig` moves onto the same deserializer and the same spellings.

## What

`maildir.root`, `m2dir.root` and `pimdir.root` take `#[serde(deserialize_with = "shell_expanded_path")]`. `downloads-dir`, on both the global and the account block, and `tls.cert` take a private `opt_shell_expanded_path` beside them, pimalaya-config shipping no optional variant yet.

`PimdirClient::new` and `Account::downloads_dir` drop their own expansion, the value reaching them already expanded.

## Scope / non-goals

The string keys already expanding at deserialize through `shell_expanded_string` are unchanged. The wizard keeps expanding the folder path it prompts for, which it does to check the directory exists before writing it, not to read it.
